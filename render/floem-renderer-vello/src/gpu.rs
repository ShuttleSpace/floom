//! A real GPU presenter: wgpu swap-chain + vello rasterisation.
//!
//! This is what turns a [`Scene`] into pixels in an actual window. The pipeline
//! is the one vello recommends, and it has three stages:
//!
//! ```text
//!   floem Scene ──▶ vello Scene ──▶ Rgba8Unorm texture ──▶ swap-chain ──▶ screen
//!                  (translation)     (vello computes)      (blit)         (present)
//! ```
//!
//! Vello renders into a fixed-format storage texture rather than directly into
//! the swap-chain, because swap-chain formats differ per platform and vello's
//! compute pipelines need `Rgba8Unorm` storage support. A `TextureBlitter`
//! copies the result across, which is what makes one code path work on Metal,
//! Vulkan and D3D12 alike.
//!
//! ## Why `unsafe` appears here
//!
//! wgpu wants to own the window while a surface exists (`create_surface` takes
//! an owned handle). We do not have one: the ABI hands over borrowed raw handles
//! so the render layer never depends on a windowing library. The bridge is
//! `create_surface_unsafe`, whose safety contract is exactly the one
//! [`AttachedSurface`] documents — the handles outlive the surface. There is one
//! `unsafe` block in this file, it is the only place the borrowed handles are
//! turned into a live resource, and the invariant is enforced by
//! [`VelloGpuPresenter::detach`] plus the app layer's drop order.
//!
//! ## Not yet implemented
//!
//! Nothing: rects, shaped text and images all render. Text goes through the
//! glyph-run resource (see `floem-text`); image uploads are cached per
//! resource id (see "Image texture caching" below).

//! ## Image texture caching
//!
//! Vello keeps its own GPU-side image atlas keyed by `ImageData`'s blob id —
//! and `Blob::new` mints a *fresh* id on every call. Building the
//! [`ImageBrush`] per frame (as this used to do) therefore defeats vello's
//! atlas: every frame looked like a brand-new image and re-uploaded the full
//! bitmap. The presenter now caches one `ImageBrush` per image resource
//! (keyed by the resource's generational id) and reuses it across frames, so
//! pixels upload once per registration, not once per frame. Cached entries not
//! referenced by the current frame are pruned, so evicted/replaced resources
//! never leak.

use floem_renderer::{AttachedSurface, Presenter, SurfaceHandles};
use floem_scene::{ResourceKind, ResourceRegistry, Scene, SceneNode};
use floem_types::{Size, SurfaceId, WindowId};
use std::collections::{HashMap, HashSet};

use vello::kurbo::{Affine, Rect as KurboRect, RoundedRect as KurboRoundedRect};
use vello::peniko::{
    Blob, Color as PenikoColor, Fill, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
    ImageSampler,
};
use vello::util::RenderSurface;
use vello::wgpu;
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{
    CompositeAlphaMode, DeviceDescriptor, Instance, InstanceDescriptor, PresentMode,
    RequestAdapterOptions, Surface, SurfaceConfiguration, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};
use vello::{AaConfig, RenderParams, RendererOptions, Scene as VelloScene};

/// What the last `present` actually drew.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GpuFrameStats {
    pub frames: u64,
    /// Rects encoded into the last frame.
    pub rects: u64,
    /// Glyph runs (one per shaped text run) encoded into the last frame.
    pub glyph_runs: u64,
    /// Images encoded into the last frame.
    pub images: u64,
    /// Nodes skipped: only nodes with a dangling / wrong-kind resource id land
    /// here now that rect, text and image all have pipelines.
    pub unsupported_nodes: u64,
    /// Frames where an image node reused a cached GPU texture.
    pub texture_cache_hits: u64,
    /// Image textures uploaded (first sighting of a resource id, or re-encode
    /// after eviction).
    pub texture_cache_misses: u64,
    /// Present calls skipped because the scene reported no damage since the
    /// last presented frame (and no attach/resize made the screen stale).
    pub present_skips: u64,
}

/// Vello + wgpu [`Presenter`].
pub struct VelloGpuPresenter {
    instance: Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: vello::Renderer,
    surface: Option<RenderSurface<'static>>,
    vello_scene: VelloScene,
    scale: f64,
    /// Logical size from the most recent attach/resize.
    size: Size,
    stats: GpuFrameStats,
    window: Option<WindowId>,
    surface_id: Option<SurfaceId>,
    /// Cached `ImageBrush` per image resource, keyed by `ResourceId::raw()`.
    /// The brush holds an `Arc` to the registry's pixels, so building it once
    /// shares bytes with the scene layer and uploads once to vello's atlas.
    image_cache: HashMap<u64, ImageBrush>,
    /// Set by attach/resize: the swap-chain's content is undefined (or stale),
    /// so the next present must run even if the scene reports no damage.
    /// Cleared once a frame has actually been presented.
    needs_present: bool,
}

impl std::fmt::Debug for VelloGpuPresenter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelloGpuPresenter")
            .field("attached", &self.surface.is_some())
            .field("size", &self.size)
            .field("scale", &self.scale)
            .field("stats", &self.stats)
            .finish()
    }
}

impl VelloGpuPresenter {
    /// Create a presenter, or `None` when no usable GPU adapter exists.
    ///
    /// Returning `None` instead of panicking matters: the app falls back to the
    /// software presenter, so a machine without a working GPU still paints.
    pub fn new() -> Option<Self> {
        let instance = Instance::new(&InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        }))
        .ok()?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default())).ok()?;

        let renderer = vello::Renderer::new(
            &device,
            RendererOptions {
                pipeline_cache: None,
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
            },
        )
        .ok()?;

        Some(Self {
            instance,
            adapter,
            device,
            queue,
            renderer,
            surface: None,
            vello_scene: VelloScene::new(),
            scale: 1.0,
            size: Size::new(0.0, 0.0),
            stats: GpuFrameStats::default(),
            window: None,
            surface_id: None,
            image_cache: HashMap::new(),
            needs_present: true,
        })
    }

    pub fn stats(&self) -> GpuFrameStats {
        self.stats
    }

    pub fn is_attached(&self) -> bool {
        self.surface.is_some()
    }

    /// The window this presenter is currently drawing into.
    pub fn window(&self) -> Option<WindowId> {
        self.window
    }

    /// The surface this presenter is currently drawing into.
    pub fn surface_id(&self) -> Option<SurfaceId> {
        self.surface_id
    }

    /// Adapter description, for diagnostics ("which GPU is this running on").
    pub fn adapter_info(&self) -> String {
        let info = self.adapter.get_info();
        format!("{} ({:?})", info.name, info.backend)
    }

    fn build_surface(&mut self, handles: SurfaceHandles, physical: (u32, u32)) -> Option<()> {
        let (width, height) = physical;
        if width == 0 || height == 0 {
            return None;
        }

        // SAFETY: the caller guarantees (documented on `AttachedSurface`) that
        // these handles stay valid until `detach`. Dropping any previous surface
        // first keeps the invariant to one live surface per window.
        let surface: Surface<'static> = unsafe {
            self.instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                    raw_display_handle: handles.display,
                    raw_window_handle: handles.window,
                })
                .ok()?
        };

        let caps = surface.get_capabilities(&self.adapter);
        // sRGB would apply a second transfer function on top of vello's own
        // linear-space compositing, so prefer the plain unorm formats.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| matches!(f, TextureFormat::Rgba8Unorm | TextureFormat::Bgra8Unorm))
            .or_else(|| caps.formats.first().copied())?;

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: PresentMode::AutoVsync,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 1,
        };
        surface.configure(&self.device, &config);

        let (target_texture, target_view) = create_target(&self.device, width, height);
        let blitter = TextureBlitter::new(&self.device, format);

        self.surface = Some(RenderSurface {
            surface,
            config,
            dev_id: 0,
            format,
            target_texture,
            target_view,
            blitter,
        });
        Some(())
    }

    fn reconfigure(&mut self, physical: (u32, u32)) {
        let (width, height) = physical;
        if width == 0 || height == 0 {
            return;
        }
        let Some(surface) = self.surface.as_mut() else {
            return;
        };
        if surface.config.width == width && surface.config.height == height {
            return;
        }
        let (target_texture, target_view) = create_target(&self.device, width, height);
        surface.target_texture = target_texture;
        surface.target_view = target_view;
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&self.device, &surface.config);
    }

    /// Translate the retained Floem scene into a vello scene.
    ///
    /// Rebuilt from scratch every presented frame: vello's `Scene` is a
    /// command list, not a retained tree, so there is nothing to diff. Frames
    /// with empty damage never reach here — `present` skips them entirely
    /// before encoding.
    fn encode(
        &mut self,
        scene: &Scene,
        resources: &ResourceRegistry,
    ) -> (u64, u64, u64, u64, u64, u64) {
        self.vello_scene.reset();
        let mut rects = 0;
        let mut glyph_runs = 0;
        let mut images = 0;
        let mut unsupported = 0;
        let mut cache_hits = 0;
        let mut cache_misses = 0;
        let mut used_images: HashSet<u64> = HashSet::new();
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                } => {
                    let rect = KurboRect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1);
                    // One fill for both shapes: a zero radius is a plain rect.
                    let shape = KurboRoundedRect::from_rect(rect, f64::from(radius.max(0.0)));
                    self.vello_scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        PenikoColor::from_rgba8(fill.r, fill.g, fill.b, fill.a),
                        None,
                        &shape,
                    );
                    rects += 1;
                }
                SceneNode::Text {
                    origin,
                    glyphs,
                    color,
                    ..
                } => {
                    let Some(ResourceKind::GlyphRun { runs }) =
                        resources.get(*glyphs).map(|r| &r.kind)
                    else {
                        // Dangling or wrong-kind id: skip rather than fail.
                        continue;
                    };
                    for run in runs {
                        if run.font.data.as_ref().is_empty() {
                            continue;
                        }
                        // Parley positions glyphs relative to the run origin;
                        // the scene node carries the text block origin. Vello
                        // wants pixel-space glyph coordinates, so fold both
                        // offsets into the transform and pass glyph positions
                        // through untouched.
                        let transform =
                            Affine::translate((origin.x + run.origin.x, origin.y + run.origin.y));
                        let glyphs = run.glyphs.iter().map(|g| vello::Glyph {
                            id: g.id,
                            x: g.x,
                            y: g.y,
                        });
                        self.vello_scene
                            .draw_glyphs(&run.font)
                            .font_size(run.font_size)
                            .transform(transform)
                            .brush(PenikoColor::from_rgba8(color.r, color.g, color.b, color.a))
                            .draw(Fill::NonZero, glyphs);
                        glyph_runs += 1;
                    }
                }
                SceneNode::Image { bounds, image } => {
                    let Some(ResourceKind::Image {
                        width,
                        height,
                        pixels,
                    }) = resources.get(*image).map(|r| &r.kind)
                    else {
                        // Dangling or wrong-kind id: skip rather than fail.
                        unsupported += 1;
                        continue;
                    };
                    if bounds.width() <= 0.0
                        || bounds.height() <= 0.0
                        || *width == 0
                        || *height == 0
                    {
                        continue;
                    }
                    // Texture cache: one ImageBrush per resource id. The
                    // brush's Blob shares the registry's `Arc<[u8]>` and keeps
                    // a stable id, which is what vello's atlas keys on —
                    // rebuilding it per frame would re-upload every frame.
                    let key = image.raw();
                    used_images.insert(key);
                    let brush = match self.image_cache.entry(key) {
                        std::collections::hash_map::Entry::Occupied(entry) => {
                            cache_hits += 1;
                            entry.into_mut()
                        }
                        std::collections::hash_map::Entry::Vacant(entry) => {
                            cache_misses += 1;
                            // One Arc clone, no pixel bytes copied: the Blob
                            // shares ownership with the registry. The extra
                            // Arc layer exists because `Blob` wants an
                            // `Arc<dyn AsRef<[u8]>>` and an unsized pointee
                            // cannot coerce directly — this happens once per
                            // registration (cache miss), not per frame.
                            let shared = std::sync::Arc::new(std::sync::Arc::clone(pixels));
                            entry.insert(ImageBrush {
                                image: ImageData {
                                    data: Blob::new(shared),
                                    format: ImageFormat::Rgba8,
                                    alpha_type: ImageAlphaType::AlphaPremultiplied,
                                    width: *width,
                                    height: *height,
                                },
                                sampler: ImageSampler::default(),
                            })
                        }
                    };
                    let sx = bounds.width() / f64::from(*width);
                    let sy = bounds.height() / f64::from(*height);
                    let transform = Affine::translate((bounds.x0, bounds.y0))
                        * Affine::scale_non_uniform(sx, sy);
                    self.vello_scene.draw_image(&*brush, transform);
                    images += 1;
                }
                SceneNode::Group { .. } => {}
            }
        }
        // Prune brushes the scene no longer references. The key includes the
        // generation, so a re-registered slot is a fresh entry by construction.
        self.image_cache.retain(|key, _| used_images.contains(key));
        (
            rects,
            glyph_runs,
            images,
            unsupported,
            cache_hits,
            cache_misses,
        )
    }
}

fn create_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("floem-vello-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        format: TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

impl Presenter for VelloGpuPresenter {
    fn attach(&mut self, surface: AttachedSurface) -> bool {
        let Some(handles) = surface.handles else {
            // Headless / in-memory surface: not ours to draw into.
            return false;
        };
        self.window = Some(surface.window);
        self.surface_id = Some(surface.surface);
        self.scale = if surface.scale > 0.0 {
            surface.scale
        } else {
            1.0
        };
        self.size = surface.size;
        self.build_surface(handles, surface.physical_size())
            .is_some()
    }

    fn resize(&mut self, size: Size, scale: f64) {
        self.size = size;
        self.scale = if scale > 0.0 { scale } else { 1.0 };
        // Even a no-op resize may follow a period where the surface was
        // occluded/invalidated; re-present the next frame to be safe.
        self.needs_present = true;
        let physical = (
            (size.width.max(0.0) * self.scale).ceil() as u32,
            (size.height.max(0.0) * self.scale).ceil() as u32,
        );
        self.reconfigure(physical);
    }

    fn present(&mut self, scene: &Scene, resources: &ResourceRegistry) {
        // Damage-driven frame skip: when the scene reports nothing changed
        // since the last presented frame and no attach/resize made the screen
        // stale, re-encoding the command list and re-running the compute
        // rasterisation would draw exactly the same pixels. Note the scene's
        // damage is the *producer's* contract — the host resets it each paint,
        // so an unchanged tree genuinely arrives empty here.
        if !self.needs_present && scene.damage().is_empty() {
            self.stats.present_skips += 1;
            self.stats.rects = 0;
            self.stats.glyph_runs = 0;
            self.stats.images = 0;
            self.stats.unsupported_nodes = 0;
            self.stats.texture_cache_hits = 0;
            self.stats.texture_cache_misses = 0;
            return;
        }
        // Any non-skipped present refreshes the screen (or would, with a
        // surface): the "stale screen" reason to present is consumed.
        self.needs_present = false;

        let (rects, glyph_runs, images, unsupported, cache_hits, cache_misses) =
            self.encode(scene, resources);
        // Encode stats are final whether or not a surface was attached — the
        // encoding work (and any skipped nodes) happened either way.
        self.stats.rects = rects;
        self.stats.glyph_runs = glyph_runs;
        self.stats.images = images;
        self.stats.unsupported_nodes = unsupported;
        self.stats.texture_cache_hits = cache_hits;
        self.stats.texture_cache_misses = cache_misses;
        let Some(surface) = self.surface.as_mut() else {
            return;
        };

        let frame = match surface.surface.get_current_texture() {
            Ok(frame) => frame,
            // The swap-chain went stale (resize, monitor change). Reconfigure
            // and skip this frame rather than painting into a dead surface.
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.surface.configure(&self.device, &surface.config);
                return;
            }
            Err(_) => return,
        };

        let params = RenderParams {
            base_color: PenikoColor::TRANSPARENT,
            width: surface.config.width,
            height: surface.config.height,
            antialiasing_method: AaConfig::Msaa16,
        };
        if self
            .renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                &self.vello_scene,
                &surface.target_view,
                &params,
            )
            .is_err()
        {
            return;
        }

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("floem surface blit"),
            });
        surface.blitter.copy(
            &self.device,
            &mut encoder,
            &surface.target_view,
            &frame.texture.create_view(&TextureViewDescriptor::default()),
        );
        self.queue.submit([encoder.finish()]);
        frame.present();

        self.stats.frames += 1;
    }

    fn detach(&mut self) {
        // Dropping the `RenderSurface` releases the swap-chain while the wgpu
        // device stays alive, so a later `attach` only reconfigures.
        self.surface = None;
        self.window = None;
        self.surface_id = None;
    }

    fn frames_presented(&self) -> u64 {
        self.stats.frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GPU tests are opportunistic: CI machines and containers frequently have
    /// no usable adapter, and "no GPU" is a supported configuration (the app
    /// falls back to the software presenter), so these tests skip rather than
    /// fail in that case.
    fn presenter() -> Option<VelloGpuPresenter> {
        VelloGpuPresenter::new()
    }

    fn headless_surface() -> AttachedSurface {
        AttachedSurface::new(
            WindowId::new(0, 1),
            SurfaceId::new(0, 1),
            Size::new(64.0, 48.0),
            1.0,
            None,
        )
    }

    #[test]
    fn headless_surface_is_refused() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        assert!(!presenter.attach(headless_surface()));
        assert!(!presenter.is_attached());
        assert!(presenter.window().is_none());
    }

    #[test]
    fn present_without_a_surface_is_a_noop() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(4.0, 4.0),
            ),
            fill: floem_types::Color::rgb(1, 2, 3),
            radius: 0.0,
        });
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        assert_eq!(presenter.frames_presented(), 0);
    }

    #[test]
    fn detach_is_idempotent() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        presenter.detach();
        presenter.detach();
        assert!(!presenter.is_attached());
    }

    #[test]
    fn adapter_is_reported() {
        let Some(presenter) = presenter() else {
            return;
        };
        assert!(!presenter.adapter_info().is_empty());
    }

    #[test]
    fn zero_sized_surface_is_refused() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        // No handles *and* no extent: still refused rather than panicking.
        let mut surface = headless_surface();
        surface.size = Size::new(0.0, 0.0);
        assert_eq!(surface.physical_size(), (0, 0));
        assert!(!presenter.attach(surface));
    }

    #[test]
    fn image_nodes_encode_their_resource() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let img = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![255u8; 16].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Image {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(1.0, 2.0),
                Size::new(8.0, 4.0),
            ),
            image: img,
        });
        scene.finish_update();

        // No surface attached: encoding still happens (and is counted), only
        // the swap-chain present is skipped.
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1, "the image node encoded");
        assert_eq!(stats.rects, 0);
        assert_eq!(stats.unsupported_nodes, 0);
        assert_eq!(presenter.frames_presented(), 0, "nothing was presented");
    }

    /// The same image resource presented across frames must hit the texture
    /// cache from the second frame on — one upload per registration, not one
    /// per frame.
    #[test]
    fn repeated_frames_hit_the_texture_cache() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let img = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 4,
                height: 4,
                pixels: std::sync::Arc::from(vec![7u8; 64].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Image {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(8.0, 8.0),
            ),
            image: img,
        });
        scene.finish_update();

        presenter.present(&scene, &registry);
        assert_eq!(presenter.stats().texture_cache_misses, 1, "first upload");
        assert_eq!(presenter.stats().texture_cache_hits, 0);

        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1, "still encoded every frame");
        assert_eq!(stats.texture_cache_hits, 1, "second frame reuses it");
        assert_eq!(
            stats.texture_cache_misses, 0,
            "per-frame stats, not cumulative: no new upload"
        );
    }

    /// Two distinct image resources each get their own cached texture; a
    /// resource id the scene stops referencing gets pruned (a re-encounter
    /// after pruning re-uploads, never serves a stale generation).
    #[test]
    fn distinct_images_cache_independently_and_unreferenced_entries_prune() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let img_a = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![1u8; 16].into_boxed_slice()),
            },
        });
        let img_b = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 3,
                height: 2,
                pixels: std::sync::Arc::from(vec![2u8; 24].into_boxed_slice()),
            },
        });

        let mut both = Scene::new();
        both.begin_update();
        for image in [img_a, img_b] {
            let _ = both.insert(SceneNode::Image {
                bounds: floem_types::Rect::from_origin_size(
                    floem_types::Point::new(0.0, 0.0),
                    Size::new(6.0, 4.0),
                ),
                image,
            });
        }
        both.finish_update();

        presenter.present(&both, &registry);
        let s = presenter.stats();
        assert_eq!(s.images, 2);
        assert_eq!(s.texture_cache_misses, 2, "two distinct uploads");

        presenter.present(&both, &registry);
        let s = presenter.stats();
        assert_eq!(s.texture_cache_hits, 2, "both cached");

        // Scene referencing only img_b: img_a's brush must be pruned.
        let mut only_b = Scene::new();
        only_b.begin_update();
        let _ = only_b.insert(SceneNode::Image {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(6.0, 4.0),
            ),
            image: img_b,
        });
        only_b.finish_update();

        presenter.present(&only_b, &registry);
        let s = presenter.stats();
        assert_eq!(s.images, 1);
        assert_eq!(s.texture_cache_hits, 1, "img_b still cached");
        assert_eq!(s.texture_cache_misses, 0);

        // img_a returns to the scene: its brush was pruned, so it re-uploads.
        presenter.present(&both, &registry);
        let s = presenter.stats();
        assert_eq!(s.texture_cache_misses, 1, "pruned img_a re-uploads once");
        assert_eq!(s.texture_cache_hits, 1, "img_b still a hit");
    }

    #[test]
    fn dangling_image_resource_is_counted_unsupported() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut scene = Scene::new();
        scene.begin_update();
        // A resource id that resolves to nothing (empty registry).
        let _ = scene.insert(SceneNode::Image {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(8.0, 4.0),
            ),
            image: floem_types::ResourceId::new(99, 1),
        });
        scene.finish_update();

        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.images, 0);
        assert_eq!(
            stats.unsupported_nodes, 1,
            "dangling id is skipped + counted"
        );
    }

    /// A present whose scene reports no damage (and no attach/resize made the
    /// screen stale) must not re-encode: the per-frame stats read zero and the
    /// skip is counted. A resize re-arms presenting.
    #[test]
    fn empty_damage_skips_present_until_resize() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let img = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![9u8; 16].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Image {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(6.0, 6.0),
            ),
            image: img,
        });
        scene.finish_update();

        // First present: there is damage, the image encodes (and uploads).
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1);
        assert_eq!(stats.present_skips, 0);

        // Same scene, damage cleared: the presenter has nothing new to draw.
        scene.clear_damage();
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.present_skips, 1, "unchanged scene skipped");
        assert_eq!(
            stats.images, 0,
            "a skipped frame encoded nothing (per-frame stats)"
        );
        assert_eq!(presenter.frames_presented(), 0, "headless: still no frame");

        // Resize marks the screen stale: the next present must run even with
        // empty damage.
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.present_skips, 1, "no extra skip after resize");
        assert_eq!(stats.images, 1, "re-encoded after resize");
        assert_eq!(
            stats.texture_cache_hits, 1,
            "re-encode reuses the cached texture"
        );
    }
}
