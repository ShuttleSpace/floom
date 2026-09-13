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
//! `SceneNode::Text` and `SceneNode::Image` are counted and skipped: they need
//! the glyph/image resource pipeline (atlas uploads) that lives with the text
//! stack, not with surface presentation. Rects — the whole of the current paint
//! output — render correctly.

use floem_renderer::{AttachedSurface, Presenter, SurfaceHandles};
use floem_scene::{ResourceKind, ResourceRegistry, Scene, SceneNode};
use floem_types::{Size, SurfaceId, WindowId};

use vello::kurbo::{Affine, Rect as KurboRect};
use vello::peniko::{Color as PenikoColor, Fill};
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
    /// Nodes skipped because the GPU path has no pipeline for them yet.
    pub unsupported_nodes: u64,
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
    /// Rebuilt from scratch every frame: vello's `Scene` is a command list, not
    /// a retained tree, so there is nothing to diff. `floem-scene`'s damage
    /// region is what the renderer would use to skip work; until partial
    /// rendering lands the whole list is re-encoded each frame.
    fn encode(&mut self, scene: &Scene, resources: &ResourceRegistry) -> (u64, u64, u64) {
        self.vello_scene.reset();
        let mut rects = 0;
        let mut glyph_runs = 0;
        let mut unsupported = 0;
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect { bounds, fill } => {
                    let rect = KurboRect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1);
                    self.vello_scene.fill(
                        Fill::NonZero,
                        Affine::IDENTITY,
                        PenikoColor::from_rgba8(fill.r, fill.g, fill.b, fill.a),
                        None,
                        &rect,
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
                SceneNode::Group { .. } => {}
                SceneNode::Image { .. } => unsupported += 1,
            }
        }
        (rects, glyph_runs, unsupported)
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
        let physical = (
            (size.width.max(0.0) * self.scale).ceil() as u32,
            (size.height.max(0.0) * self.scale).ceil() as u32,
        );
        self.reconfigure(physical);
    }

    fn present(&mut self, scene: &Scene, resources: &ResourceRegistry) {
        let (rects, glyph_runs, unsupported) = self.encode(scene, resources);
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
        self.stats.rects = rects;
        self.stats.glyph_runs = glyph_runs;
        self.stats.unsupported_nodes = unsupported;
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
}
