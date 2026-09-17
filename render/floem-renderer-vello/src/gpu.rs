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

//! ## Incremental encode
//!
//! Vello's `Scene` is a command list, so there is no retained tree to diff —
//! but there is something better to cache. Each scene node's commands are
//! encoded into its own fragment keyed by the node's (stable) id, and reused
//! wholesale while the node's payload hash is unchanged: a frame where one
//! widget moved re-encodes that one widget and appends the rest. Fragments
//! also pin the image brushes their commands were built from, so a reused
//! fragment never has its texture evicted from under it.

use floem_renderer::{
    AttachedSurface, ExternalFrame, ExternalTextureCache, ExternalTextureStats, ImportDecision,
    Presenter, SurfaceHandles,
};
use floem_scene::{ResourceKind, ResourceRegistry, Scene, SceneNode};
use floem_types::{Size, SurfaceId, WindowId};
use std::collections::{HashMap, HashSet};

use vello::kurbo::{Affine, Rect as KurboRect, RoundedRect as KurboRoundedRect};
use vello::peniko::{
    BlendMode, Blob, Color as PenikoColor, Fill, ImageAlphaType, ImageBrush, ImageData,
    ImageFormat, ImageSampler,
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

/// Draws `brush` inside `transform`, dimmed to `alpha`.
///
/// vello 0.7 has no per-draw alpha: `fill` stops at the brush and
/// `draw_image` takes nothing but a brush and a transform. So a faded image is
/// a one-draw layer with the alpha on the layer instead, clipped to the image's
/// own rectangle *in its own transform* — the layer then covers exactly the
/// pixels the image would have covered, and nothing else picks up the alpha.
///
/// The opaque case skips the layer entirely: pushing one costs an offscreen
/// pass, and almost every image on screen is fully opaque.
fn draw_image_at(
    target: &mut VelloScene,
    brush: &ImageBrush,
    transform: Affine,
    width: u32,
    height: u32,
    alpha: u8,
) {
    if alpha >= u8::MAX {
        target.draw_image(brush, transform);
        return;
    }
    let rect = KurboRect::new(0.0, 0.0, f64::from(width), f64::from(height));
    target.push_layer(
        Fill::NonZero,
        BlendMode::default(),
        f32::from(alpha) / f32::from(u8::MAX),
        transform,
        &rect,
    );
    target.draw_image(brush, transform);
    target.pop_layer();
}

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
    /// Scene nodes drawn by appending a cached, unchanged vello fragment
    /// instead of re-encoding the node's commands.
    pub fragments_reused: u64,
    /// Scene nodes freshly encoded into vello commands this frame (their
    /// payload changed, or the node is new).
    pub fragments_encoded: u64,
    /// Pixels vello rasterised this frame. With a real surface and partial
    /// damage this is the damaged area, not the whole screen.
    pub rasterized_pixels: u64,
    /// Frames that rasterised only the damaged region instead of the whole
    /// target (1 when partial, 0 when full-frame).
    pub partial_rasterizations: u64,
    /// External textures seen for the first time. Cumulative: a resource id is
    /// imported once and refreshed from then on.
    pub external_imports: u64,
    /// Refreshes of an external texture whose producer reported new contents.
    pub external_refreshes: u64,
    /// Frames a producer handed over that needed no work at all. For content
    /// slower than the display refresh this is the normal case.
    pub external_reuses: u64,
    /// Bytes copied across for external textures. Zero when a producer keeps
    /// its pixels on the GPU — that is the fast path, not a missing count.
    pub external_bytes: u64,
}

/// Scratch texture for partial rasterisation: vello has no scissor, so a
/// damaged region is rendered into a texture the size of that region and
/// copied back into the persistent target at the region's origin.
struct ScratchTarget {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
}

/// What encoding one scene node produced: the draw commands land in a
/// fragment scene, the counters feed the per-frame stats.
#[derive(Debug, Clone, Default)]
struct NodeEncodeStats {
    rects: u64,
    glyph_runs: u64,
    images: u64,
    unsupported: u64,
    cache_hits: u64,
    cache_misses: u64,
    /// Image resources this fragment's commands still reference. A reused
    /// fragment keeps those brushes alive for exactly as long as the
    /// fragment does — the alternative (re-deriving usage from the frames
    /// that happened to re-encode) would evict a brush a live fragment is
    /// still built from.
    image_keys: Vec<u64>,
}

/// A cached translation of one scene node into vello commands.
///
/// Vello's `Scene` is a command list, but a cheap one to *append*: an
/// unchanged node's commands are reused wholesale instead of re-encoded, so a
/// frame with one moving widget pays encoding only for that widget. The
/// counters let a reused fragment still report its draws (and texture reuse)
/// in the per-frame stats.
struct SceneFragment {
    fingerprint: u64,
    scene: VelloScene,
    stats: NodeEncodeStats,
}

/// A payload hash distinguishing "this node's draws would be identical" from
/// "re-encode". Field bits, not `PartialEq`: two equal payloads hash equal,
/// and any field change (bounds, colour, resource id — which carries a
/// generation) flips the hash.
fn node_fingerprint(node: &SceneNode) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match node {
        SceneNode::Group {
            children,
            transform,
        } => {
            hasher.write_u8(0);
            for child in children {
                hasher.write_u64(child.raw());
            }
            for v in [
                transform.a,
                transform.b,
                transform.c,
                transform.d,
                transform.e,
                transform.f,
            ] {
                hasher.write_u64(v.to_bits());
            }
        }
        SceneNode::Rect {
            bounds,
            fill,
            radius,
        } => {
            hasher.write_u8(1);
            for v in [bounds.x0, bounds.y0, bounds.x1, bounds.y1] {
                hasher.write_u64(v.to_bits());
            }
            hasher.write_u32(
                u32::from(fill.r) << 24
                    | u32::from(fill.g) << 16
                    | u32::from(fill.b) << 8
                    | u32::from(fill.a),
            );
            hasher.write_u32(radius.to_bits());
        }
        SceneNode::Image {
            bounds,
            image,
            alpha,
        } => {
            hasher.write_u8(2);
            for v in [bounds.x0, bounds.y0, bounds.x1, bounds.y1] {
                hasher.write_u64(v.to_bits());
            }
            hasher.write_u64(image.raw());
            // In the fingerprint on purpose: the encoder bakes the brush into a
            // cached fragment, so an alpha that is not part of the key would
            // keep serving the fragment built at the old one — a frozen
            // mid-fade image, arriving through the cache rather than the clock.
            hasher.write_u8(*alpha);
        }
        SceneNode::Text {
            origin,
            size,
            glyphs,
            color,
        } => {
            hasher.write_u8(3);
            for v in [origin.x, origin.y, size.width, size.height] {
                hasher.write_u64(v.to_bits());
            }
            hasher.write_u64(glyphs.raw());
            hasher.write_u32(
                u32::from(color.r) << 24
                    | u32::from(color.g) << 16
                    | u32::from(color.b) << 8
                    | u32::from(color.a),
            );
        }
    }
    hasher.finish()
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
    /// Brushes for textures whose pixels the framework does not own — a
    /// `Texture`-mode participant's frames. Keyed by the same resource id the
    /// participant's image node references, so the two caches are the same
    /// lookup from `encode_node`'s point of view and a resource cannot be in
    /// both.
    external_brushes: HashMap<u64, ImageBrush>,
    /// When each external texture's contents last changed.
    external: ExternalTextureCache,
    /// Cached vello command fragments per scene node, keyed by
    /// `SceneNodeId::raw()` (stable across frames — index + generation).
    /// Unchanged nodes are appended, not re-encoded; entries the scene no
    /// longer holds are pruned each frame.
    fragments: HashMap<u64, SceneFragment>,
    /// Set by attach/resize: the swap-chain's content is undefined (or stale),
    /// so the next present must run even if the scene reports no damage.
    /// Cleared once a frame has actually been presented.
    needs_present: bool,
    /// Scratch texture for partial rasterisation, grown on demand and reused
    /// across frames (a damaged region rarely keeps the same size).
    scratch: Option<ScratchTarget>,
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
            external_brushes: HashMap::new(),
            external: ExternalTextureCache::new(),
            fragments: HashMap::new(),
            needs_present: true,
            scratch: None,
        })
    }

    pub fn stats(&self) -> GpuFrameStats {
        self.stats
    }

    /// Hands the presenter a frame of a texture the framework does not own —
    /// a `Texture`-mode participant's contents.
    ///
    /// Call it once per produced frame, before the frame that draws it. The
    /// answer is the whole point: content slower than the display refresh (a
    /// 30 fps stream in a 60 fps app, a map that is not moving, a paused video)
    /// answers [`ImportDecision::Unchanged`] and costs nothing at all.
    ///
    /// The resource id is the identity, not the frame, so a refresh keeps
    /// everything keyed on it and costs one upload rather than a second
    /// texture. Cached fragments that drew this resource are dropped, because
    /// vello commands embed the brush they were built from: a reused fragment
    /// would otherwise keep drawing the pixels it was built with, which is
    /// exactly the "frozen video" bug of device case D8.
    pub fn import_texture(&mut self, frame: ExternalFrame) -> ImportDecision {
        let key = frame.resource.raw();
        let (width, height) = (frame.width, frame.height);
        let decision = self.external.import(&frame);
        if !decision.is_upload() {
            return decision;
        }
        if let Some(pixels) = frame.pixels {
            // One Arc clone; the same shape the registry path uses, and for the
            // same reason — `Blob` wants `Arc<dyn AsRef<[u8]>>`.
            let shared = std::sync::Arc::new(pixels);
            self.external_brushes.insert(
                key,
                ImageBrush {
                    image: ImageData {
                        data: Blob::new(shared),
                        format: ImageFormat::Rgba8,
                        alpha_type: ImageAlphaType::AlphaPremultiplied,
                        width,
                        height,
                    },
                    sampler: ImageSampler::default(),
                },
            );
        }
        self.fragments
            .retain(|_, fragment| !fragment.stats.image_keys.contains(&key));
        decision
    }

    /// Whether the presenter is holding this resource as an external texture.
    pub fn has_external_texture(&self, resource: floem_types::ResourceId) -> bool {
        self.external.contains(resource)
    }

    /// Drops external textures that are neither still being produced nor still
    /// drawn by a cached node.
    ///
    /// Driven from the fragment table rather than from the encode loop, because
    /// it has to run on *skipped* frames too: a producer going quiet is not
    /// scene damage, so a frame the presenter has nothing to draw is precisely
    /// when a texture most needs dropping. On a skipped frame the fragments are
    /// the last encoded frame's, which is the current scene by definition —
    /// nothing changed, or the frame would not have been skipped.
    fn prune_external(&mut self) {
        let referenced: HashSet<u64> = self
            .fragments
            .values()
            .flat_map(|fragment| fragment.stats.image_keys.iter().copied())
            .collect();
        self.external.prune(&referenced);
        let live: HashSet<u64> = self.external.live_keys().collect();
        self.external_brushes.retain(|key, _| live.contains(key));
    }

    /// Cumulative external-texture counters: imports, refreshes, reuses, bytes.
    ///
    /// These are the numbers task 16.3 step 4 is decided with — bytes per frame
    /// and imports per second are what say whether `Texture` mode is a fallback
    /// or a default.
    pub fn external_stats(&self) -> ExternalTextureStats {
        self.external.stats()
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
        let (scratch_texture, scratch_view) = create_scratch(&self.device, width, height);
        self.scratch = Some(ScratchTarget {
            texture: scratch_texture,
            view: scratch_view,
            width,
            height,
        });
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
        let (scratch_texture, scratch_view) = create_scratch(&self.device, width, height);
        surface.target_texture = target_texture;
        surface.target_view = target_view;
        self.scratch = Some(ScratchTarget {
            texture: scratch_texture,
            view: scratch_view,
            width,
            height,
        });
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&self.device, &surface.config);
    }

    /// Translate the retained Floem scene into a vello scene, incrementally.
    ///
    /// Each node's commands live in a cached [`SceneFragment`] keyed by the
    /// node's stable id: a node whose payload hash is unchanged is *appended*
    /// to the frame, not re-encoded, so a frame where one widget moved pays
    /// encoding only for that widget. Fragments the scene no longer holds are
    /// pruned. Frames with empty damage never reach here — `present` skips
    /// them entirely before encoding.
    fn encode(
        &mut self,
        scene: &Scene,
        resources: &ResourceRegistry,
        offset: (f64, f64),
    ) -> GpuFrameStats {
        self.vello_scene.reset();
        // When only a region is rasterised the scene is composed shifted, so
        // the region's top-left becomes the scratch texture's origin.
        let shift = Affine::translate((-offset.0, -offset.1));
        let mut totals = GpuFrameStats {
            present_skips: 0,
            ..GpuFrameStats::default()
        };
        let mut used_fragments: HashSet<u64> = HashSet::new();
        let mut used_images: HashSet<u64> = HashSet::new();
        for (id, node) in scene.iter() {
            let fingerprint = node_fingerprint(node);
            let key = id.raw();
            if let Some(cached) = self.fragments.get_mut(&key) {
                if cached.fingerprint == fingerprint {
                    // Unchanged node: append its cached commands and report
                    // its draws (a reused image is a texture-cache hit — the
                    // pixels are already resident).
                    self.vello_scene.append(&cached.scene, Some(shift));
                    totals.rects += cached.stats.rects;
                    totals.glyph_runs += cached.stats.glyph_runs;
                    totals.images += cached.stats.images;
                    totals.unsupported_nodes += cached.stats.unsupported;
                    totals.texture_cache_hits += cached.stats.images;
                    totals.fragments_reused += 1;
                    for key in cached.stats.image_keys.iter().copied() {
                        used_images.insert(key);
                    }
                    used_fragments.insert(key);
                    continue;
                }
            }
            let mut fragment_scene = VelloScene::new();
            let stats = self.encode_node(&mut fragment_scene, node, resources, &mut used_images);
            // Accumulate before the stats move into the fragment.
            totals.rects += stats.rects;
            totals.glyph_runs += stats.glyph_runs;
            totals.images += stats.images;
            totals.unsupported_nodes += stats.unsupported;
            totals.texture_cache_hits += stats.cache_hits;
            totals.texture_cache_misses += stats.cache_misses;
            totals.fragments_encoded += 1;
            used_fragments.insert(key);
            self.fragments.insert(
                key,
                SceneFragment {
                    fingerprint,
                    scene: fragment_scene,
                    stats,
                },
            );
            self.vello_scene
                .append(&self.fragments[&key].scene, Some(shift));
        }
        // Prune fragments the scene no longer holds (and brushes nothing
        // re-encoded this frame references). The keys include generations, so
        // a re-created node or re-registered slot is a fresh entry by
        // construction.
        self.fragments.retain(|key, _| used_fragments.contains(key));
        self.image_cache.retain(|key, _| used_images.contains(key));
        totals
    }

    /// Encodes one scene node's draws into `target`, updating the image
    /// texture cache. Pure per-node work — no frame state.
    fn encode_node(
        &mut self,
        target: &mut VelloScene,
        node: &SceneNode,
        resources: &ResourceRegistry,
        used_images: &mut HashSet<u64>,
    ) -> NodeEncodeStats {
        let mut stats = NodeEncodeStats::default();
        match node {
            SceneNode::Rect {
                bounds,
                fill,
                radius,
            } => {
                let rect = KurboRect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1);
                // One fill for both shapes: a zero radius is a plain rect.
                let shape = KurboRoundedRect::from_rect(rect, f64::from(radius.max(0.0)));
                target.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    PenikoColor::from_rgba8(fill.r, fill.g, fill.b, fill.a),
                    None,
                    &shape,
                );
                stats.rects += 1;
            }
            SceneNode::Text {
                origin,
                glyphs,
                color,
                ..
            } => {
                let Some(ResourceKind::GlyphRun { runs }) = resources.get(*glyphs).map(|r| &r.kind)
                else {
                    // Dangling or wrong-kind id: skip rather than fail.
                    return stats;
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
                    target
                        .draw_glyphs(&run.font)
                        .font_size(run.font_size)
                        .transform(transform)
                        .brush(PenikoColor::from_rgba8(color.r, color.g, color.b, color.a))
                        .draw(Fill::NonZero, glyphs);
                    stats.glyph_runs += 1;
                }
            }
            SceneNode::Image {
                bounds,
                image,
                alpha,
            } => {
                if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                    return stats;
                }
                let key = image.raw();
                // An external texture has no pixels in the registry — they are
                // the producer's — so it is resolved *before* the registry
                // lookup rather than through it. This is what lets a
                // `Texture`-mode participant be an ordinary scene node: it
                // composites inside the scene and is occluded, clipped and
                // transformed by the framework like anything else (D10).
                if let Some((width, height)) = self.external.size(*image) {
                    let brush = self.external_brushes.get(&key).cloned();
                    if let Some(brush) = brush {
                        used_images.insert(key);
                        stats.image_keys.push(key);
                        stats.cache_hits += 1;
                        let sx = bounds.width() / f64::from(width);
                        let sy = bounds.height() / f64::from(height);
                        let transform = Affine::translate((bounds.x0, bounds.y0))
                            * Affine::scale_non_uniform(sx, sy);
                        draw_image_at(target, &brush, transform, width, height, *alpha);
                        stats.images += 1;
                    }
                    return stats;
                }
                let Some(ResourceKind::Image {
                    width,
                    height,
                    pixels,
                }) = resources.get(*image).map(|r| &r.kind)
                else {
                    // Dangling or wrong-kind id: skip rather than fail.
                    stats.unsupported += 1;
                    return stats;
                };
                if *width == 0 || *height == 0 {
                    return stats;
                }
                // Texture cache: one ImageBrush per resource id. The
                // brush's Blob shares the registry's `Arc<[u8]>` and keeps
                // a stable id, which is what vello's atlas keys on —
                // rebuilding it per frame would re-upload every frame.
                let key = image.raw();
                used_images.insert(key);
                stats.image_keys.push(key);
                let brush = match self.image_cache.entry(key) {
                    std::collections::hash_map::Entry::Occupied(entry) => {
                        stats.cache_hits += 1;
                        entry.into_mut()
                    }
                    std::collections::hash_map::Entry::Vacant(entry) => {
                        stats.cache_misses += 1;
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
                let transform =
                    Affine::translate((bounds.x0, bounds.y0)) * Affine::scale_non_uniform(sx, sy);
                draw_image_at(target, brush, transform, *width, *height, *alpha);
                stats.images += 1;
            }
            SceneNode::Group { .. } => {}
        }
        stats
    }

    /// Number of cached node fragments (diagnostics: the working set size).
    pub fn fragment_count(&self) -> usize {
        self.fragments.len()
    }

    /// The damaged region in physical pixels, or `None` when the whole target
    /// must be rasterised (full-frame damage, or damage covering nearly all
    /// of it — at that point a region render plus a copy is not cheaper).
    ///
    /// `damage` is in logical pixels; `scale` converts it to the target's
    /// physical grid. The rect is padded by one pixel on each side so an
    /// anti-aliased edge that straddles the reported bounds is still
    /// repainted.
    fn damage_region(
        damage: &floem_scene::DamageRegion,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Option<(u32, u32, u32, u32)> {
        if width == 0 || height == 0 || damage.is_empty() || damage.is_full_frame() {
            return None;
        }
        let mut x0 = f64::INFINITY;
        let mut y0 = f64::INFINITY;
        let mut x1 = f64::NEG_INFINITY;
        let mut y1 = f64::NEG_INFINITY;
        for rect in damage.rects() {
            x0 = x0.min(rect.x0);
            y0 = y0.min(rect.y0);
            x1 = x1.max(rect.x1);
            y1 = y1.max(rect.y1);
        }
        if !(x1 > x0) || !(y1 > y0) {
            return None;
        }
        let px0 = (x0 * scale).floor() - 1.0;
        let py0 = (y0 * scale).floor() - 1.0;
        let px1 = (x1 * scale).ceil() + 1.0;
        let py1 = (y1 * scale).ceil() + 1.0;
        let x = px0.max(0.0) as u32;
        let y = py0.max(0.0) as u32;
        let right = (px1.ceil() as i64).clamp(0, width as i64) as u32;
        let bottom = (py1.ceil() as i64).clamp(0, height as i64) as u32;
        if right <= x || bottom <= y {
            return None;
        }
        let (w, h) = (right - x, bottom - y);
        // Partial rasterisation only pays off below ~3/4 of the target.
        if u64::from(w) * u64::from(h) * 4 >= u64::from(width) * u64::from(height) * 3 {
            return None;
        }
        Some((x, y, w, h))
    }

    /// The scratch view, provided it can hold a `width x height` region.
    ///
    /// The scratch texture is allocated once per target size (see
    /// `build_surface`/`reconfigure`) rather than per region: a region changes
    /// size almost every frame, and reallocating a GPU texture each time would
    /// cost more than the rasterisation it saves. Vello writes only the
    /// `width x height` area it is given, and the copy reads exactly that back.
    fn scratch_for(&self, width: u32, height: u32) -> Option<wgpu::TextureView> {
        let scratch = self.scratch.as_ref()?;
        if width > scratch.width || height > scratch.height {
            return None;
        }
        Some(scratch.view.clone())
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
        usage: TextureUsages::STORAGE_BINDING
            | TextureUsages::TEXTURE_BINDING
            | TextureUsages::COPY_DST,
        format: TextureFormat::Rgba8Unorm,
        view_formats: &[],
    });
    let view = texture.create_view(&TextureViewDescriptor::default());
    (texture, view)
}

fn create_scratch(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("floem-vello-scratch"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        // Storage: vello renders into it. Copy src: it is copied back into
        // the persistent target.
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::COPY_SRC,
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
        // Cumulative, and published before any early return: a skipped present
        // has not made them stale.
        let external = self.external.stats();
        self.stats.external_imports = external.first_imports;
        self.stats.external_refreshes = external.refreshes;
        self.stats.external_reuses = external.reuses;
        self.stats.external_bytes = external.bytes_uploaded;
        // Damage-driven frame skip: when the scene reports nothing changed
        // since the last presented frame and no attach/resize made the screen
        // stale, re-encoding the command list and re-running the compute
        // rasterisation would draw exactly the same pixels. Note the scene's
        // damage is the *producer's* contract — the host resets it each paint,
        // so an unchanged tree genuinely arrives empty here.
        if !self.needs_present && scene.damage().is_empty() {
            self.prune_external();
            self.stats.present_skips += 1;
            self.stats.rects = 0;
            self.stats.glyph_runs = 0;
            self.stats.images = 0;
            self.stats.unsupported_nodes = 0;
            self.stats.texture_cache_hits = 0;
            self.stats.texture_cache_misses = 0;
            self.stats.fragments_reused = 0;
            self.stats.fragments_encoded = 0;
            self.stats.rasterized_pixels = 0;
            self.stats.partial_rasterizations = 0;
            return;
        }
        // Any non-skipped present refreshes the screen (or would, with a
        // surface): the "stale screen" reason to present is consumed. It also
        // decides the rasterisation area — after an attach/resize the target
        // is a brand-new texture with undefined content, so only a full-frame
        // rasterisation can fill it.
        let stale_screen = self.needs_present;
        self.needs_present = false;

        // Physical extent of the target, whether or not a surface is attached.
        let (pw, ph) = match self.surface.as_ref() {
            Some(surface) => (surface.config.width, surface.config.height),
            None => (
                (self.size.width.max(0.0) * self.scale).ceil() as u32,
                (self.size.height.max(0.0) * self.scale).ceil() as u32,
            ),
        };
        let region = if stale_screen {
            None
        } else {
            Self::damage_region(scene.damage(), pw, ph, self.scale)
        };
        let offset = region.map_or((0.0, 0.0), |(x, y, _, _)| (f64::from(x), f64::from(y)));
        let encoded = self.encode(scene, resources, offset);
        // After `encode`, the fragment table is this frame's scene — which is
        // what says whether an external texture is still being drawn.
        self.prune_external();
        self.stats.rasterized_pixels = match region {
            Some((_, _, w, h)) => u64::from(w) * u64::from(h),
            None => u64::from(pw) * u64::from(ph),
        };
        self.stats.partial_rasterizations = u64::from(region.is_some());
        // Encode stats are final whether or not a surface was attached — the
        // encoding work (and any skipped nodes) happened either way.
        self.stats.rects = encoded.rects;
        self.stats.glyph_runs = encoded.glyph_runs;
        self.stats.images = encoded.images;
        self.stats.unsupported_nodes = encoded.unsupported_nodes;
        self.stats.texture_cache_hits = encoded.texture_cache_hits;
        self.stats.texture_cache_misses = encoded.texture_cache_misses;
        self.stats.fragments_reused = encoded.fragments_reused;
        self.stats.fragments_encoded = encoded.fragments_encoded;
        // Prepare the scratch view up front: both it and the surface live on
        // `self`, and the surface is borrowed mutably below.
        let scratch = if self.surface.is_some() {
            match region {
                Some((_, _, w, h)) => match self.scratch_for(w, h) {
                    Some(view) => Some(view),
                    None => return,
                },
                None => None,
            }
        } else {
            None
        };

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

        match scratch {
            // Partial frame: rasterise only the damaged region into the
            // scratch texture (already composed shifted, so the region's
            // origin is the texture's origin), then copy it into the
            // persistent target. Everything outside the region keeps the
            // pixels the last frame left there.
            Some(view) => {
                let (rx, ry, w, h) = region.expect("a partial frame always carries its region");
                let params = RenderParams {
                    base_color: PenikoColor::TRANSPARENT,
                    width: w,
                    height: h,
                    antialiasing_method: AaConfig::Msaa16,
                };
                if self
                    .renderer
                    .render_to_texture(&self.device, &self.queue, &self.vello_scene, &view, &params)
                    .is_err()
                {
                    return;
                }
                let Some(scratch_texture) = self.scratch.as_ref().map(|s| &s.texture) else {
                    return;
                };
                let mut encoder =
                    self.device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("floem damage copy"),
                        });
                encoder.copy_texture_to_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: scratch_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::TexelCopyTextureInfo {
                        texture: &surface.target_texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: rx, y: ry, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                );
                self.queue.submit([encoder.finish()]);
            }
            // Full frame: rasterise straight into the target, as before.
            None => {
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
            }
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

    /// Two image nodes differing only in alpha must not share a fragment.
    ///
    /// The fragment cache keys on this fingerprint and bakes the brush into the
    /// encoded commands, so an alpha left out of the key would keep serving the
    /// fragment built at the old alpha — a picture frozen mid-fade, arriving
    /// through the cache rather than through the clock. No GPU needed: the
    /// fingerprint is pure.
    #[test]
    fn an_images_alpha_is_part_of_its_fingerprint() {
        let bounds = floem_types::Rect::from_origin_size(
            floem_types::Point::new(1.0, 2.0),
            Size::new(8.0, 4.0),
        );
        let image = floem_types::ResourceId::new(0, 1);
        let at = |alpha| {
            node_fingerprint(&SceneNode::Image {
                bounds,
                image,
                alpha,
            })
        };
        assert_ne!(at(255), at(128), "a dimmed image is a different encoding");
        assert_ne!(at(255), at(254), "even one step must re-encode");
        assert_eq!(at(200), at(200), "and the same payload is stable");
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
        let _ = scene.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(1.0, 2.0),
                Size::new(8.0, 4.0),
            ),
            img,
        ));
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
        let _ = scene.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(8.0, 8.0),
            ),
            img,
        ));
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
            let _ = both.insert(SceneNode::image(
                floem_types::Rect::from_origin_size(
                    floem_types::Point::new(0.0, 0.0),
                    Size::new(6.0, 4.0),
                ),
                image,
            ));
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
        let _ = only_b.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(6.0, 4.0),
            ),
            img_b,
        ));
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
        let _ = scene.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(8.0, 4.0),
            ),
            floem_types::ResourceId::new(99, 1),
        ));
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
    /// An unchanged scene encodes once: the second present appends every
    /// cached fragment instead of re-encoding it, while still reporting the
    /// same draws.
    #[test]
    fn unchanged_scene_reuses_cached_fragments() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut scene = Scene::new();
        scene.begin_update();
        let id = scene.alloc_id();
        scene.upsert(id, rect_node(0.0));
        let second = scene.alloc_id();
        scene.upsert(second, rect_node(1.0));
        scene.finish_update();

        presenter.present(&scene, &ResourceRegistry::new());
        let first = presenter.stats();
        assert_eq!(first.fragments_encoded, 2, "both nodes encoded");
        assert_eq!(first.fragments_reused, 0);
        assert_eq!(first.rects, 2);

        // Re-present the identical tree (a resize re-arms presenting even
        // with empty damage).
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.fragments_reused, 2, "every fragment reused");
        assert_eq!(
            stats.fragments_encoded, 0,
            "nothing was re-encoded (per-frame stats)"
        );
        assert_eq!(stats.rects, 2, "the draws are still reported");
        assert_eq!(presenter.fragment_count(), 2, "two cached fragments");
    }

    /// Changing one node re-encodes only that node's fragment; removing one
    /// prunes it from the cache on the same frame.
    #[test]
    fn only_the_changed_node_is_reencoded() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut scene = Scene::new();
        scene.begin_update();
        let moved = scene.alloc_id();
        scene.upsert(moved, rect_node(0.0));
        let steady = scene.alloc_id();
        scene.upsert(steady, rect_node(1.0));
        let doomed = scene.alloc_id();
        scene.upsert(doomed, rect_node(2.0));
        scene.finish_update();

        presenter.present(&scene, &ResourceRegistry::new());
        assert_eq!(presenter.stats().fragments_encoded, 3);

        // Move one node: that node re-encodes, the other two are appended.
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        scene.begin_update();
        scene.upsert(moved, rect_node(7.0));
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.fragments_encoded, 1, "only the moved node re-encodes");
        assert_eq!(stats.fragments_reused, 2, "the rest are cached");
        assert_eq!(stats.rects, 3, "all three still drawn");

        // Drop a node: its fragment is pruned in the same frame.
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        scene.begin_update();
        scene.remove(doomed);
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.rects, 2, "two nodes left");
        assert_eq!(
            presenter.fragment_count(),
            2,
            "the removed node's fragment is gone"
        );
    }

    /// A damaged frame rasterises just that region; a frame that follows an
    /// attach/resize rasterises everything, because the target texture is new
    /// and its content is undefined.
    #[test]
    fn damaged_frames_rasterize_only_the_damaged_region() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        let mut scene = Scene::new();
        scene.begin_update();
        let id = scene.alloc_id();
        scene.upsert(id, rect_node(0.0));
        scene.finish_update();

        // Stale screen: full-frame rasterisation regardless of the damage.
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(
            stats.partial_rasterizations, 0,
            "a new target must be filled completely"
        );
        assert_eq!(stats.rasterized_pixels, 64 * 48);

        // Move the rect: the damage is its old and new bounds, a few pixels.
        scene.begin_update();
        scene.upsert(id, rect_node(1.0));
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(
            stats.partial_rasterizations, 1,
            "a damaged frame rasterises a region"
        );
        assert!(
            stats.rasterized_pixels > 0 && stats.rasterized_pixels < 64 * 48,
            "far less than the whole screen, got {}",
            stats.rasterized_pixels
        );
    }

    /// Full-frame damage (a group node reports no bounds, so its change marks
    /// the whole frame) forces a full rasterisation.
    #[test]
    fn full_frame_damage_rasterizes_the_whole_target() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        let mut scene = Scene::new();
        scene.begin_update();
        let id = scene.alloc_id();
        scene.upsert(id, rect_node(0.0));
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());

        // A group node has no bounds hint: changing it marks the full frame.
        scene.begin_update();
        let group = scene.alloc_id();
        scene.upsert(
            group,
            SceneNode::Group {
                children: Default::default(),
                transform: floem_types::Transform::IDENTITY,
            },
        );
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(
            stats.partial_rasterizations, 0,
            "full-frame damage is not partial"
        );
        assert_eq!(stats.rasterized_pixels, 64 * 48);
    }

    fn rect_node(x: f64) -> SceneNode {
        SceneNode::Rect {
            bounds: floem_types::Rect::from_origin_size(
                floem_types::Point::new(x, 0.0),
                Size::new(4.0, 4.0),
            ),
            fill: floem_types::Color::rgb(1, 2, 3),
            radius: 0.0,
        }
    }

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
        let _ = scene.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(6.0, 6.0),
            ),
            img,
        ));
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

    // --- external textures (16.3) ------------------------------------------

    /// A scene whose only node draws `texture` — the shape a `Texture`-mode
    /// participant has.
    fn scene_drawing(texture: floem_types::ResourceId) -> Scene {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floem_types::Rect::from_origin_size(
                floem_types::Point::new(0.0, 0.0),
                Size::new(8.0, 8.0),
            ),
            texture,
        ));
        scene.finish_update();
        scene
    }

    /// A producer's frame: the pixels are the producer's, so the registry holds
    /// no `Image` for this id at all.
    fn external(texture: floem_types::ResourceId, sequence: u64) -> ExternalFrame {
        ExternalFrame {
            resource: texture,
            sequence,
            width: 2,
            height: 2,
            pixels: Some(std::sync::Arc::from(vec![255u8; 16].into_boxed_slice())),
        }
    }

    /// The plan's step 2 in one test: an external texture is drawn as an
    /// ordinary scene node, and re-imported only when its producer says the
    /// contents changed.
    ///
    /// "An ordinary scene node" is the point. The framework drew it, so it can
    /// be occluded, clipped and transformed — which is exactly what Overlay
    /// mode cannot do (device case D10).
    #[test]
    fn an_external_texture_is_drawn_and_re_imported_only_when_it_changes() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        // A placeholder with no pixels behind it: the participant's contents are
        // the producer's, not the registry's.
        let texture = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Blob { bytes: 0 },
        });
        let scene = scene_drawing(texture);

        assert_eq!(
            presenter.import_texture(external(texture, 1)),
            ImportDecision::First
        );
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1, "drawn, not skipped");
        assert_eq!(
            stats.unsupported_nodes, 0,
            "an external resource is not a dangling id"
        );
        assert_eq!(stats.external_imports, 1);
        assert_eq!(stats.external_bytes, 16, "one upload's worth of pixels");

        // Same sequence: the producer had nothing new, so nothing crosses.
        assert_eq!(
            presenter.import_texture(external(texture, 1)),
            ImportDecision::Unchanged
        );
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1, "still drawn");
        assert_eq!(stats.external_reuses, 1);
        assert_eq!(stats.external_bytes, 16, "and nothing was re-uploaded");

        // New contents: one refresh, same identity throughout.
        assert_eq!(
            presenter.import_texture(external(texture, 2)),
            ImportDecision::Refresh
        );
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.external_refreshes, 1);
        assert_eq!(stats.external_bytes, 32);
        assert_eq!(
            presenter.external_stats().first_imports,
            1,
            "three frames of content, one texture"
        );
    }

    /// A refreshed texture must not be served out of a reused fragment.
    ///
    /// Vello commands embed the brush they were built from, so caching the
    /// encode per node is exactly wrong for content that changes underneath an
    /// unchanged node — the "content frozen at the pre-transform size" bug of
    /// device case D8, arriving through the fragment cache instead of the
    /// texture cache. `unchanged_scene_reuses_cached_fragments` is the other
    /// half of this rule: nothing is invalidated when nothing changed.
    #[test]
    fn refreshing_a_texture_invalidates_the_fragment_that_drew_it() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let texture = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Blob { bytes: 0 },
        });
        let scene = scene_drawing(texture);

        presenter.import_texture(external(texture, 1));
        presenter.present(&scene, &registry);
        assert_eq!(presenter.fragment_count(), 1, "the node was encoded");

        // Unchanged: the fragment survives and is reused.
        presenter.import_texture(external(texture, 1));
        presenter.present(&scene, &registry);
        assert_eq!(presenter.fragment_count(), 1);
        assert_eq!(presenter.stats().fragments_reused, 1);

        // New pixels: the cached commands have the old brush baked in.
        presenter.import_texture(external(texture, 2));
        assert_eq!(
            presenter.fragment_count(),
            0,
            "the fragment that drew this texture was dropped"
        );
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.fragments_encoded, 1, "so it was encoded again");
        assert_eq!(
            stats.fragments_reused, 0,
            "and not served from the cache with last frame's pixels"
        );
        assert_eq!(stats.images, 1);
    }

    /// A texture that is neither being produced any more nor drawn by the scene
    /// is dropped — those are the two ways to stay alive.
    ///
    /// A removed node damages the scene, so the frame that drops it is an
    /// ordinary encode: a node leaving the tree is what makes a texture stop
    /// being referenced, and the producer stops with it.
    #[test]
    fn an_external_texture_is_pruned_when_its_producer_and_its_node_are_both_gone() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let texture = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Blob { bytes: 0 },
        });
        let scene = scene_drawing(texture);
        presenter.import_texture(external(texture, 1));
        presenter.present(&scene, &registry);
        assert!(presenter.has_external_texture(texture));

        // The node is gone, so the participant stopped producing: a frame that
        // draws something else and imports nothing.
        let mut without = Scene::new();
        without.begin_update();
        let _ = without.insert(rect_node(0.0));
        without.finish_update();
        presenter.present(&without, &registry);

        assert!(
            !presenter.has_external_texture(texture),
            "neither producing nor drawn: the texture is gone"
        );
        assert_eq!(presenter.external_stats().evictions, 1);
    }
}
