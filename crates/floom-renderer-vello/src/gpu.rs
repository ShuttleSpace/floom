//! A real GPU presenter: wgpu swap-chain + vello rasterisation.
//!
//! This is what turns a [`Scene`] into pixels in an actual window. The pipeline
//! is the one vello recommends, and it has three stages:
//!
//! ```text
//!   floom Scene ──▶ vello Scene ──▶ Rgba8Unorm texture ──▶ swap-chain ──▶ screen
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
//! glyph-run resource (see `floom-text`); image uploads are cached per
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

use floom_renderer::{
    AttachedSurface, DegradationReport, ExternalFrame, ExternalTextureCache, ExternalTextureStats,
    ImportDecision, Presenter, RendererCapabilities, SurfaceHandles,
};
use floom_scene::{ResourceKind, ResourceRegistry, Scene, SceneNode};
use floom_types::{ClipRegion, PathSeg, Rect, Shader, Size, SurfaceId, WindowId};
use std::collections::{HashMap, HashSet};

use crate::budget::{CacheTrimTelemetry, GpuCacheBudget, MemoryPressure, TrimPlan, plan_trim};

use vello::kurbo::{
    Affine, BezPath, Cap as KurboLineCap, Join as KurboLineJoin, Point as KurboPoint,
    Rect as KurboRect, RoundedRect as KurboRoundedRect, Stroke as KurboStroke,
};
use vello::peniko::{
    BlendMode, Blob, Brush as PenikoBrush, Color as PenikoColor, Extend, Fill, Gradient,
    GradientKind, ImageAlphaType, ImageBrush, ImageData, ImageFormat, ImageSampler,
    LinearGradientPosition, RadialGradientPosition,
};
use vello::util::RenderSurface;
use vello::wgpu;
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{
    BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BufferBindingType, CompositeAlphaMode, DeviceDescriptor, Instance, InstanceDescriptor,
    PipelineLayout, PipelineLayoutDescriptor, PresentMode, RenderPipeline, RequestAdapterOptions,
    ShaderStages, Surface, SurfaceConfiguration, Texture, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};
use vello::{AaConfig, RenderParams, RendererOptions, Scene as VelloScene};

/// A [`Shadow`] folded into the parameters of vello's blur primitive.
///
/// vello's `draw_blurred_rounded_rect` knows three things: a rect, a corner
/// radius and a standard deviation. A CSS `box-shadow` knows four more than
/// that — an offset, a spread, and a colour, plus a blur that is a *radius*
/// rather than a σ. This is the bridge, and every choice in it is a choice
/// about which of the two vocabularies wins.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowBlurRect {
    /// The silhouette, already offset by the shadow's own offset. Vello's
    /// primitive has no offset of its own, so it goes into the rect.
    pub rect: KurboRect,
    /// The corner radius, already grown by the spread.
    pub radius: f64,
    /// The Gaussian standard deviation. CSS's `blur` is a radius, not a σ;
    /// see [`CSS_BLUR_TO_SIGMA`] for the conversion.
    pub std_dev: f64,
    /// The shadow's own colour, drawn blurred.
    pub color: PenikoColor,
}

/// CSS's `box-shadow` blur length, expressed as a Gaussian σ.
///
/// CSS defines the blur radius as the point where the kernel's response has
/// fallen to half, and a Gaussian's σ is where it has fallen to `e^-1/2` —
/// the two are not the same number, and using one for the other makes every
/// shadow the wrong softness. `sqrt(ln 2 / 2)` is the ratio between them.
///
/// This is the one conversion the software backend does *not* make: its
/// three-pass box filter is a different approximation, and matching σ to σ
/// would not match pixel to pixel. What matches is the *visible* softness,
/// and each backend's own approximation is what produces it.
pub const CSS_BLUR_TO_SIGMA: f64 = 0.832_554_611_157_697_8; // sqrt(ln 2 / 2)

/// Folds a [`Shadow`] into vello's blur primitive.
///
/// # Where each shadow parameter goes
///
/// * `offset_x` / `offset_y` — translate the rect. The primitive blurs
///   whatever rect it is handed, at that rect's position, so moving the rect
///   is the only way to move the shadow.
/// * `spread` — grows the rect and the corner radius, keeping the centre
///   fixed. This is the CSS reading (dilate the silhouette, *then* blur), and
///   it is the one `shadow_spread.rs` pins on the software side: folding
///   spread into the blur instead produces a fatter, softer shadow, which is
///   a different effect rather than a more or less visible one.
/// * `blur` — becomes the σ, via [`CSS_BLUR_TO_SIGMA`].
/// * `color` — the blurred fill.
///
/// # What it does not do
///
/// The spread grows the rect on all four sides *including the corners*, which
/// is what CSS specifies for a box-shadow. It cannot do more than that: vello
/// blurs a rounded rectangle, so any silhouette that is not one — a path, a
/// glyph run — has no representation here, and the caller counts a skip
/// rather than drawing an approximation nobody asked for.
pub fn shadow_blur_rect(bounds: Rect, radius: f64, shadow: &floom_types::Shadow) -> ShadowBlurRect {
    // A negative spread erodes, and an erosion that closes on the centre
    // would leave a negative width. Clamping at zero keeps the primitive's
    // width/height non-negative — the same guard `radius.max(0.0)` gives the
    // shape itself — and an over-eroded silhouette legitimately vanishes.
    // The clamp is on the *extent*, not on the spread: clamping the spread
    // itself would silently turn every negative value into a no-op, and a
    // shadow asked to erode by 3px would draw at full size.
    let spread = shadow.spread;
    let cx = (bounds.x0 + bounds.x1) / 2.0 + shadow.offset_x;
    let cy = (bounds.y0 + bounds.y1) / 2.0 + shadow.offset_y;
    let half_w = ((bounds.x1 - bounds.x0) / 2.0 + spread).max(0.0);
    let half_h = ((bounds.y1 - bounds.y0) / 2.0 + spread).max(0.0);
    ShadowBlurRect {
        rect: KurboRect::new(cx - half_w, cy - half_h, cx + half_w, cy + half_h),
        radius: (radius + spread).max(0.0),
        std_dev: shadow.blur.max(0.0) * CSS_BLUR_TO_SIGMA,
        color: PenikoColor::from_rgba8(
            shadow.color.r,
            shadow.color.g,
            shadow.color.b,
            shadow.color.a,
        ),
    }
}

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
    /// Vector path nodes encoded into the last frame.
    pub paths: u64,
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
    /// Drop shadows this backend drew: a rect whose shadow was encoded into
    /// vello's blur primitive. The "did my shadow render" number — the
    /// software backend and the skia adapter have the same one.
    pub shadows_drawn: u64,
    /// Drop shadows skipped because vello has no blur primitive yet — the
    /// recorded degradation for a shadow request this backend cannot honour.
    ///
    /// Now that vello 0.7 has a blur primitive, a *rounded rect* shadow is
    /// drawn rather than skipped; what still lands here is a silhouette the
    /// primitive cannot express — a path or a glyph run carrying a shadow, or
    /// one whose parameters degenerate (zero σ, or an over-eroded spread).
    pub shadows_skipped: u64,
    /// Filter requests this backend did not apply: a filtered scope whose
    /// content was drawn sharp instead, and a backdrop filter drawn as nothing.
    ///
    /// One count rather than one per kind because the answer is the same for
    /// both — vello has no blur primitive, so neither can be honoured — and
    /// because what a caller needs from this number is "was my filter dropped",
    /// not which of the two it was.
    pub filters_skipped: u64,
    /// Custom-shader nodes this backend actually ran on the GPU — a WGSL
    /// program compiled into a pipeline and rendered to an offscreen texture.
    /// This is the "did my shader run" number; the software rasteriser and the
    /// skia adapter can never raise it.
    pub shaders_drawn: u64,
    /// Custom-shader nodes this backend could not run: a program that failed to
    /// compile, or a node whose bounds made a zero-sized target. The recorded
    /// degradation for a shader request vello could not honour — never silent.
    pub shaders_skipped: u64,
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

impl GpuFrameStats {
    /// The frame's degradations, in the vocabulary every backend shares.
    ///
    /// Vello already spells all four the same way the report does, so this is
    /// a field-for-field copy — which is the point: the report exists so a
    /// host does not have to know that vello happened to agree.
    pub fn degradation(&self) -> DegradationReport {
        DegradationReport {
            shadows_skipped: self.shadows_skipped,
            filters_skipped: self.filters_skipped,
            backdrop_filters_skipped: 0,
            shaders_skipped: self.shaders_skipped,
        }
    }
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
    paths: u64,
    glyph_runs: u64,
    images: u64,
    unsupported: u64,
    /// Shadow requests skipped: vello has no blur primitive yet, so a node
    /// carrying a drop shadow draws its fill and records the skip here.
    shadow_skips: u64,
    /// Shadow requests this fragment actually drew through the blur primitive.
    shadow_drawn: u64,
    /// Filter requests skipped on a node that reached the encoder: a
    /// `BackdropFilter` draws nothing and says so here.
    filter_skips: u64,
    /// Custom-shader requests drawn / skipped on the GPU this fragment.
    shader_drawn: u64,
    shader_skips: u64,
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
        // The group markers are consumed by the encode walk (like the clip
        // markers) and never reach a fragment fingerprint; matched anyway so
        // the enum stays exhaustively handled.
        SceneNode::GroupPush { transform } => {
            hasher.write_u8(0);
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
        SceneNode::GroupPop => hasher.write_u8(7),
        SceneNode::FilterPush { filter } => {
            hasher.write_u8(8);
            hash_filter(&mut hasher, filter);
        }
        SceneNode::FilterPop => hasher.write_u8(9),
        SceneNode::BackdropFilter { rect, filter } => {
            hasher.write_u8(10);
            for v in [rect.x0, rect.y0, rect.x1, rect.y1] {
                hasher.write_u64(v.to_bits());
            }
            hash_filter(&mut hasher, filter);
        }
        SceneNode::Rect {
            bounds,
            fill,
            radius,
            shadow,
        } => {
            hasher.write_u8(1);
            for v in [bounds.x0, bounds.y0, bounds.x1, bounds.y1] {
                hasher.write_u64(v.to_bits());
            }
            hash_brush(&mut hasher, fill);
            hasher.write_u32(radius.to_bits());
            match shadow {
                None => hasher.write_u8(0),
                Some(shadow) => {
                    hasher.write_u8(1);
                    hash_color(&mut hasher, &shadow.color);
                    // `spread` belongs in this key: it is part of the request,
                    // so two nodes differing only in it must not collide in
                    // the render cache.
                    for v in [shadow.blur, shadow.offset_x, shadow.offset_y, shadow.spread] {
                        hasher.write_u64(v.to_bits());
                    }
                }
            }
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
        SceneNode::Path { path, fill, stroke } => {
            hasher.write_u8(4);
            // Every segment, bit for bit. A path is the one payload whose
            // *shape* is data rather than a handful of numbers, so a hash over
            // "it is still a path" would freeze a cached fragment on the first
            // frame's geometry — the same class of bug the image alpha note
            // above describes, one level deeper.
            for seg in path.segments() {
                match *seg {
                    PathSeg::MoveTo(p) => {
                        hasher.write_u8(0);
                        hash_point(&mut hasher, p);
                    }
                    PathSeg::LineTo(p) => {
                        hasher.write_u8(1);
                        hash_point(&mut hasher, p);
                    }
                    PathSeg::QuadTo { ctrl, to } => {
                        hasher.write_u8(2);
                        hash_point(&mut hasher, ctrl);
                        hash_point(&mut hasher, to);
                    }
                    PathSeg::CubicTo { c1, c2, to } => {
                        hasher.write_u8(3);
                        hash_point(&mut hasher, c1);
                        hash_point(&mut hasher, c2);
                        hash_point(&mut hasher, to);
                    }
                    PathSeg::Close => hasher.write_u8(4),
                }
            }
            match fill {
                None => hasher.write_u8(0),
                Some(fill) => {
                    hasher.write_u8(1);
                    hash_brush(&mut hasher, fill);
                }
            }
            match stroke {
                None => hasher.write_u8(0),
                Some(stroke) => {
                    hasher.write_u8(1);
                    hasher.write_u64(stroke.width.to_bits());
                    hasher.write_u8(stroke.cap as u8);
                    hasher.write_u8(stroke.join as u8);
                    hasher.write_u64(stroke.miter_limit.to_bits());
                    hash_color(&mut hasher, &stroke.color);
                }
            }
        }
        SceneNode::ClipPush { region } => {
            hasher.write_u8(5);
            hash_clip_region(&mut hasher, region);
        }
        SceneNode::ClipPop => hasher.write_u8(6),
        // A shader node's pixels are dynamic (`u.time`), so it is never served
        // from the fragment cache (see the `encode` walk). The fingerprint is
        // still needed for exhaustiveness and so any future cache participant
        // keys on the *program*, not "a shader was here": hashing only a flag
        // would reuse the first node's rendered pixels for every other shader.
        SceneNode::Shader { bounds, shader } => {
            hasher.write_u8(11);
            for v in [bounds.x0, bounds.y0, bounds.x1, bounds.y1] {
                hasher.write_u64(v.to_bits());
            }
            hasher.write(shader.wgsl.as_bytes());
            for p in shader.params.iter().copied() {
                hasher.write_u32(p.to_bits());
            }
        }
    }
    hasher.finish()
}

/// Hash of a shader program's WGSL source, used as the cache key for its
/// compiled pipeline. The source is the program's identity: id is stable even
/// if naga re-validates differently under a new version, and two nodes using
/// the same source share one pipeline (which they must, so the driver can
/// hoist the compile).
fn hash_shader_wgsl(wgsl: &str) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u8(0x52);
    hasher.write(wgsl.as_bytes());
    hasher.finish()
}

/// Cache key for a shader's readback image: the program plus the render-target
/// size it was rasterised at. Two programs sharing a source but running at
/// different sizes must not collide, nor may a resize reuse the stale brush
/// (a `Shader` node re-encodes every frame, but a reused brush under the same
/// key would otherwise carry the previous frame's dimensions).
fn shader_cache_key(wgsl: &str, pw: u32, ph: u32) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    hasher.write_u8(0x53);
    hasher.write(wgsl.as_bytes());
    hasher.write_u32(pw);
    hasher.write_u32(ph);
    hasher.finish()
}

/// Hashes a clip region's geometry and identity.
///
/// Used by the fingerprint of the marker nodes themselves (which are never
/// fragmented) rather than by the fragment key of the nodes under them: vello
/// applies a clip through the *command stream*, not through each fragment's
/// contents, so a cached fragment is valid under any clip and stays reusable
/// when one moves.
fn hash_clip_region(hasher: &mut impl std::hash::Hasher, region: &ClipRegion) {
    match region {
        ClipRegion::Rect(bounds) => {
            hasher.write_u8(0);
            hash_rect(hasher, *bounds);
        }
        ClipRegion::RoundedRect { bounds, radius } => {
            hasher.write_u8(1);
            hash_rect(hasher, *bounds);
            hasher.write_u64(radius.to_bits());
        }
        ClipRegion::Path {
            bounds,
            origin,
            path,
        } => {
            hasher.write_u8(2);
            hash_rect(hasher, *bounds);
            hash_point(hasher, *origin);
            for seg in path.segments() {
                match *seg {
                    PathSeg::MoveTo(p) => {
                        hasher.write_u8(0);
                        hash_point(hasher, p);
                    }
                    PathSeg::LineTo(p) => {
                        hasher.write_u8(1);
                        hash_point(hasher, p);
                    }
                    PathSeg::QuadTo { ctrl, to } => {
                        hasher.write_u8(2);
                        hash_point(hasher, ctrl);
                        hash_point(hasher, to);
                    }
                    PathSeg::CubicTo { c1, c2, to } => {
                        hasher.write_u8(3);
                        hash_point(hasher, c1);
                        hash_point(hasher, c2);
                        hash_point(hasher, to);
                    }
                    PathSeg::Close => hasher.write_u8(4),
                }
            }
        }
    }
}

fn hash_point(hasher: &mut impl std::hash::Hasher, p: floom_types::Point) {
    hasher.write_u64(p.x.to_bits());
    hasher.write_u64(p.y.to_bits());
}

fn hash_rect(hasher: &mut impl std::hash::Hasher, r: floom_types::Rect) {
    for v in [r.x0, r.y0, r.x1, r.y1] {
        hasher.write_u64(v.to_bits());
    }
}

fn hash_color(hasher: &mut impl std::hash::Hasher, c: &floom_types::Color) {
    hasher.write_u32(
        u32::from(c.r) << 24 | u32::from(c.g) << 16 | u32::from(c.b) << 8 | u32::from(c.a),
    );
}

/// Fingerprint of a brush: kind tag, geometry, every stop. Two different
/// gradients must never share a fragment cache entry.
fn hash_brush(hasher: &mut impl std::hash::Hasher, b: &floom_types::Brush) {
    match b {
        floom_types::Brush::Solid(c) => {
            hasher.write_u8(0);
            hash_color(hasher, c);
        }
        floom_types::Brush::Linear { stops, from, to } => {
            hasher.write_u8(1);
            for p in [from.x, from.y, to.x, to.y] {
                hasher.write_u64(p.to_bits());
            }
            hash_stops(hasher, stops);
        }
        floom_types::Brush::Radial {
            stops,
            center,
            radius,
        } => {
            hasher.write_u8(2);
            for p in [center.x, center.y] {
                hasher.write_u64(p.to_bits());
            }
            hasher.write_u64(radius.to_bits());
            hash_stops(hasher, stops);
        }
    }
}

/// Hashes a filter the way [`hash_brush`] hashes a brush: field bits, so a
/// changed radius invalidates the cached fragment instead of drawing last
/// frame's blur.
fn hash_filter(hasher: &mut impl std::hash::Hasher, filter: &floom_types::Filter) {
    match filter {
        floom_types::Filter::Blur { radius_x, radius_y } => {
            hasher.write_u8(0);
            hasher.write_u64(radius_x.to_bits());
            hasher.write_u64(radius_y.to_bits());
        }
    }
}

fn hash_stops(hasher: &mut impl std::hash::Hasher, stops: &[(f32, floom_types::Color)]) {
    hasher.write_u64(stops.len() as u64);
    for (offset, color) in stops {
        hasher.write_u32(offset.to_bits());
        hash_color(hasher, color);
    }
}

/// Maps the scene's brush vocabulary onto peniko's, so a gradient encoded on
/// the GPU interpolates exactly the ramp the software rasteriser samples.
fn fill_with_brush(
    target: &mut VelloScene,
    shape: &impl vello::kurbo::Shape,
    brush: &floom_types::Brush,
) {
    // The gradient is built per call and borrowed for exactly that one `fill`:
    // vello's brush parameter takes the gradient by reference (`&Gradient`),
    // so the ramp cannot outlive the arm that built it.
    let to_peniko = |ramp: &[(f32, floom_types::Color)]| -> Vec<(f32, PenikoColor)> {
        ramp.iter()
            .map(|(offset, c)| (*offset, PenikoColor::from_rgba8(c.r, c.g, c.b, c.a)))
            .collect()
    };
    match brush {
        floom_types::Brush::Solid(c) => target.fill(
            Fill::NonZero,
            Affine::IDENTITY,
            PenikoColor::from_rgba8(c.r, c.g, c.b, c.a),
            None,
            shape,
        ),
        floom_types::Brush::Linear { stops, from, to } => {
            let mut gradient = Gradient::default();
            gradient.kind = GradientKind::Linear(LinearGradientPosition {
                start: KurboPoint::new(from.x, from.y),
                end: KurboPoint::new(to.x, to.y),
            });
            gradient.extend = Extend::Pad;
            let ramp = to_peniko(stops);
            target.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                PenikoBrush::Gradient(&gradient.with_stops(&*ramp)),
                None,
                shape,
            )
        }
        floom_types::Brush::Radial {
            stops,
            center,
            radius,
        } => {
            let mut gradient = Gradient::default();
            gradient.kind = GradientKind::Radial(RadialGradientPosition {
                start_center: KurboPoint::new(center.x, center.y),
                start_radius: 0.0,
                end_center: KurboPoint::new(center.x, center.y),
                end_radius: *radius as f32,
            });
            gradient.extend = Extend::Pad;
            let ramp = to_peniko(stops);
            target.fill(
                Fill::NonZero,
                Affine::IDENTITY,
                PenikoBrush::Gradient(&gradient.with_stops(&*ramp)),
                None,
                shape,
            )
        }
    }
}

/// Translates a path in Floom's vocabulary into kurbo's.
fn bez_path(path: &floom_types::Path) -> BezPath {
    let mut bez = BezPath::new();
    for seg in path.segments() {
        match *seg {
            PathSeg::MoveTo(p) => bez.move_to((p.x, p.y)),
            PathSeg::LineTo(p) => bez.line_to((p.x, p.y)),
            PathSeg::QuadTo { ctrl, to } => bez.quad_to((ctrl.x, ctrl.y), (to.x, to.y)),
            PathSeg::CubicTo { c1, c2, to } => {
                bez.curve_to((c1.x, c1.y), (c2.x, c2.y), (to.x, to.y));
            }
            PathSeg::Close => bez.close_path(),
        }
    }
    bez
}

fn kurbo_rect(bounds: floom_types::Rect) -> KurboRect {
    KurboRect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1)
}

fn kurbo_cap(cap: floom_types::LineCap) -> KurboLineCap {
    match cap {
        floom_types::LineCap::Butt => KurboLineCap::Butt,
        floom_types::LineCap::Round => KurboLineCap::Round,
        floom_types::LineCap::Square => KurboLineCap::Square,
    }
}

fn kurbo_join(join: floom_types::LineJoin) -> KurboLineJoin {
    match join {
        floom_types::LineJoin::Miter => KurboLineJoin::Miter,
        floom_types::LineJoin::Round => KurboLineJoin::Round,
        floom_types::LineJoin::Bevel => KurboLineJoin::Bevel,
    }
}

/// Opens a clip layer for `region`.
///
/// Vello has no scissor: a clip is a layer, which is why the renderer pushes
/// one per region rather than intersecting rectangles. The region arrives in
/// absolute logical coordinates, so a path region also carries the translate
/// that places its own coordinates — `push_clip_layer` applies its `transform`
/// to the clip shape alone and leaves the rest of the stream alone.
fn push_clip_region(scene: &mut VelloScene, region: &ClipRegion) {
    match region {
        ClipRegion::Rect(bounds) => {
            let shape = kurbo_rect(*bounds);
            scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &shape);
        }
        ClipRegion::RoundedRect { bounds, radius } => {
            let radius = radius.clamp(0.0, bounds.width().min(bounds.height()) / 2.0);
            let shape = KurboRoundedRect::from_rect(kurbo_rect(*bounds), radius);
            scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &shape);
        }
        ClipRegion::Path { origin, path, .. } => {
            let shape = bez_path(path);
            let place = Affine::translate((origin.x, origin.y));
            scene.push_clip_layer(Fill::NonZero, place, &shape);
        }
    }
}

/// WGSL prologue the host prepends to every custom-shader program. It owns the
/// vertex stage (a fullscreen triangle) and the uniform block; the caller's
/// source is only the body of `fn fs_main`. See [`floom_types::Shader`] for the
/// contract the fragment sees — `u.resolution`, `u.time`, `u.params[0..1]`
/// (two `vec4<f32>`), and `frag.uv` (0..1, y down).
const SHADER_PROLOGUE: &str = r#"
struct VsOut {
  @builtin(position) pos: vec4<f32>,
  @location(0) uv: vec2<f32>,
};
struct Uniforms {
  resolution: vec2<f32>,
  time: f32,
  _pad0: f32,
  params: array<vec4<f32>, 2>,
};
@group(0) @binding(0) var<uniform> u: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
  var positions = array<vec2<f32>, 3>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>( 3.0, -1.0),
    vec2<f32>(-1.0,  3.0)
  );
  var out: VsOut;
  let p = positions[vid];
  out.pos = vec4<f32>(p, 0.0, 1.0);
  out.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - (p.y + 1.0) * 0.5));
  return out;
}
"#;

/// A compiled custom-shader program. wgpu 27's `create_shader_module` and
/// `create_render_pipeline` are synchronous non-`Result`, so a naga/USM
/// validation error is surfaced at creation time rather than leaving a
/// recoverable "failed" state to look up later.
enum ShaderProgram {
    Pipeline(RenderPipeline),
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
    /// Retained-state budgets (fragments, resident textures, scratch) and
    /// what trimming has done about them.
    budget: GpuCacheBudget,
    trim_telemetry: CacheTrimTelemetry,
    /// Bind-group layout for custom-shader uniforms: one uniform buffer at
    /// binding 0, visible to both stages.
    shader_bind_group_layout: BindGroupLayout,
    /// Pipeline layout shared by every custom-shader program (they all use the
    /// same uniform shape).
    shader_pipeline_layout: PipelineLayout,
    /// Compiled custom-shader pipelines, keyed by a hash of the program source.
    /// A `Failed` entry records a program that did not compile so the next
    /// frame skips it without re-attempting.
    shader_programs: HashMap<u64, ShaderProgram>,
    /// Offscreen textures produced this frame (one per drawn `Shader` node),
    /// kept alive until the next frame begins, when the vello pass that sampled
    /// them has already submitted.
    shader_textures: Vec<Texture>,
    /// Seconds handed to custom shaders as `u.time`, advanced one frame's worth
    /// per `present` so an animated program actually moves.
    shader_clock: f32,
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

/// The one transform between the scene and the target.
///
/// The scene is authored in logical units; the target is in device pixels. A
/// point is therefore scaled by the display density first and then shifted by
/// the damaged region's *device-space* origin (see [`VelloGpuPresenter::damage_region`],
/// which converts the scene's logical damage into device pixels). Both halves
/// matter. Without the scale the app paints 1:1 into a `scale`-times-larger
/// target — half size in the corner of a 2× display — while hit-testing still
/// converts the pointer *into* logical units, so every tap lands at `1/scale`
/// of where the widget is drawn. With the translate composed on the other side
/// the offset would itself be scaled, and a partially rasterised frame would
/// land displaced.
fn scene_transform(offset: (f64, f64), scale: f64) -> Affine {
    // kurbo composes `a * b` as "apply `b` first", so this scales and then
    // translates — the order the two halves need.
    Affine::translate((-offset.0, -offset.1)) * Affine::scale(scale)
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

        // One bind-group / pipeline layout for every custom-shader program:
        // they all share the same uniform shape (resolution, time, params).
        let shader_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("floom-shader-bgl"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let shader_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("floom-shader-pl"),
            bind_group_layouts: &[&shader_bind_group_layout],
            push_constant_ranges: &[],
        });

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
            budget: GpuCacheBudget::for_current_platform(),
            trim_telemetry: CacheTrimTelemetry::default(),
            shader_bind_group_layout,
            shader_pipeline_layout,
            shader_programs: HashMap::new(),
            shader_textures: Vec::new(),
            shader_clock: 0.0,
        })
    }

    /// Overrides the retained-state budget (defaults come from the platform
    /// class; see `GpuCacheBudget::for_current_platform`). Takes effect on the
    /// next encode; call [`Self::trim`] to shed what is already retained.
    pub fn set_budget(&mut self, budget: GpuCacheBudget) {
        self.budget = budget;
    }

    /// The budget in force.
    pub fn budget(&self) -> GpuCacheBudget {
        self.budget
    }

    /// Trim telemetry: trims performed, cache entries shed, caching refused.
    pub fn trim_telemetry(&self) -> CacheTrimTelemetry {
        self.trim_telemetry
    }

    /// Bytes of resident image textures (registry-backed brushes only;
    /// external textures are the producer's pipeline, accounted by
    /// `external_stats`, and are governed by pruning, not by the budget).
    fn resident_resource_bytes(&self) -> u64 {
        self.image_cache
            .values()
            .map(|brush| u64::from(brush.image.width) * u64::from(brush.image.height) * 4)
            .sum()
    }

    /// How many image textures are currently resident (diagnostics).
    pub fn resident_resources(&self) -> usize {
        self.image_cache.len()
    }

    /// Sheds retained state under memory pressure. Geometry goes first (it
    /// re-encodes from the scene at the next frame's cost), resident textures
    /// second (they re-upload), the scratch texture last (it is reallocated
    /// on the next partial frame). A trim never touches external textures:
    /// those belong to their producer's pipeline and are governed by
    /// `prune_external`.
    pub fn trim(&mut self, pressure: MemoryPressure) {
        let scratch_bytes = self
            .scratch
            .as_ref()
            .map(|s| u64::from(s.width) * u64::from(s.height) * 4)
            .unwrap_or(0);
        let plan = plan_trim(
            &self.budget,
            self.fragments.len(),
            self.resident_resource_bytes(),
            scratch_bytes,
            pressure,
        );
        if plan == TrimPlan::NOTHING {
            return;
        }
        if plan.drop_fragments {
            self.trim_telemetry.fragments_dropped += self.fragments.len() as u64;
            self.fragments.clear();
        }
        if plan.drop_resources {
            self.trim_telemetry.resources_dropped += self.image_cache.len() as u64;
            self.image_cache.clear();
        }
        if plan.release_scratch {
            self.scratch = None;
        }
        self.trim_telemetry.trims += 1;
        #[cfg(debug_assertions)]
        eprintln!(
            "[floom-gpu] trim({pressure:?}): fragments_dropped={} resources_dropped={} \
             (telemetry: {:?})",
            self.trim_telemetry.fragments_dropped,
            self.trim_telemetry.resources_dropped,
            self.trim_telemetry
        );
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
    pub fn has_external_texture(&self, resource: floom_types::ResourceId) -> bool {
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
        // Scratch is budget-bound: a target whose scratch would exceed the
        // budget simply runs full-frame rasterisation (the partial path
        // falls back below), instead of holding a texture the budget forbids.
        self.scratch = if u64::from(width) * u64::from(height) * 4 <= self.budget.max_scratch_bytes
        {
            let (scratch_texture, scratch_view) = create_scratch(&self.device, width, height);
            Some(ScratchTarget {
                texture: scratch_texture,
                view: scratch_view,
                width,
                height,
            })
        } else {
            None
        };
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
        // Same budget rule as `build_surface`: oversize targets go full-frame.
        self.scratch = if u64::from(width) * u64::from(height) * 4 <= self.budget.max_scratch_bytes
        {
            let (scratch_texture, scratch_view) = create_scratch(&self.device, width, height);
            Some(ScratchTarget {
                texture: scratch_texture,
                view: scratch_view,
                width,
                height,
            })
        } else {
            None
        };
        surface.config.width = width;
        surface.config.height = height;
        surface.surface.configure(&self.device, &surface.config);
    }

    /// Translate the retained Floom scene into a vello scene, incrementally.
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
        let shift = scene_transform(offset, self.scale);
        let mut totals = GpuFrameStats {
            present_skips: 0,
            ..GpuFrameStats::default()
        };
        let mut used_fragments: HashSet<u64> = HashSet::new();
        let mut used_images: HashSet<u64> = HashSet::new();
        // The clips in force, as a stack. Vello's layers nest, and the scene's
        // `ClipPush`/`ClipPop` markers describe exactly that stack: a push opens
        // a layer, a pop closes the innermost one, and two overlapping clips
        // intersect (clip A *and* clip B) — the same thing the software
        // backend's coverage stack does. The walk therefore pushes and pops
        // without ever closing the previous layer; closing it would drop the
        // outer clip from everything the inner one covers, which is the bug a
        // "replace" reading of the markers would introduce.
        let mut clip_depth: usize = 0;
        // The transform scopes in force, as a stack of the *outer* composed
        // affines: a pop restores the top directly. A scope is not a vello
        // layer (nothing to close at the end of the stream), so a trailing
        // push is genuinely harmless here.
        let mut group_stack: Vec<Affine> = Vec::new();
        let mut group_affine = Affine::IDENTITY;
        for (id, node) in scene.iter() {
            match node {
                SceneNode::ClipPush { region } => {
                    push_clip_region(&mut self.vello_scene, region);
                    clip_depth += 1;
                    continue;
                }
                SceneNode::ClipPop => {
                    // The scene is well-formed (never pops an empty stack), so
                    // this always matches a preceding push.
                    if clip_depth > 0 {
                        self.vello_scene.pop_layer();
                        clip_depth -= 1;
                    }
                    continue;
                }
                SceneNode::GroupPush { transform } => {
                    group_stack.push(group_affine);
                    group_affine = group_affine
                        * Affine::new([
                            transform.a,
                            transform.b,
                            transform.c,
                            transform.d,
                            transform.e,
                            transform.f,
                        ]);
                    continue;
                }
                SceneNode::GroupPop => {
                    if let Some(outer) = group_stack.pop() {
                        group_affine = outer;
                    }
                    continue;
                }
                // A filter scope is consumed here and drawn as an ordinary
                // band of picture: vello has no blur primitive, so the content
                // inside it lands sharp. The alternative — dropping the
                // content too — would make a filtered card invisible, which is
                // a worse answer to "cannot blur" than "did not blur".
                SceneNode::FilterPush { filter } => {
                    if !filter.is_identity() {
                        totals.filters_skipped += 1;
                    }
                    continue;
                }
                SceneNode::FilterPop => continue,
                _ => {}
            }
            let fingerprint = node_fingerprint(node);
            let key = id.raw();
            // A shader node's pixels are dynamic (`u.time` advances each
            // present): serving it from the fragment cache would freeze the
            // first frame's output, and a stale entry under the same id (a node
            // that used to be a different kind) would draw the wrong thing. So
            // shaders never enter the cache and always re-encode. (They are
            // still fingerprinted, for exhaustiveness.)
            if !matches!(node, SceneNode::Shader { .. }) {
                if let Some(cached) = self.fragments.get_mut(&key) {
                    // A reused fragment must not outlive the textures its commands
                    // were built from: a brush that lost residency (a resource
                    // budget refusal, an external prune) means the cached commands
                    // would draw nothing, so the node re-encodes instead — which
                    // is also what rebuilds the missing brush.
                    let textures_resident = cached.stats.image_keys.iter().all(|&k| {
                        self.image_cache.contains_key(&k) || self.external_brushes.contains_key(&k)
                    });
                    if cached.fingerprint == fingerprint && textures_resident {
                        // Unchanged node: append its cached commands and report
                        // its draws (a reused image is a texture-cache hit — the
                        // pixels are already resident).
                        self.vello_scene
                            .append(&cached.scene, Some(shift * group_affine));
                        totals.rects += cached.stats.rects;
                        totals.paths += cached.stats.paths;
                        totals.glyph_runs += cached.stats.glyph_runs;
                        totals.images += cached.stats.images;
                        totals.unsupported_nodes += cached.stats.unsupported;
                        totals.shadows_skipped += cached.stats.shadow_skips;
                        totals.shadows_drawn += cached.stats.shadow_drawn;
                        totals.filters_skipped += cached.stats.filter_skips;
                        totals.texture_cache_hits += cached.stats.images;
                        totals.fragments_reused += 1;
                        for key in cached.stats.image_keys.iter().copied() {
                            used_images.insert(key);
                        }
                        used_fragments.insert(key);
                        continue;
                    }
                }
            }
            let mut fragment_scene = VelloScene::new();
            let stats = self.encode_node(&mut fragment_scene, node, resources, &mut used_images);
            // Accumulate before the stats move into the fragment.
            totals.rects += stats.rects;
            totals.paths += stats.paths;
            totals.glyph_runs += stats.glyph_runs;
            totals.images += stats.images;
            totals.unsupported_nodes += stats.unsupported;
            totals.shadows_skipped += stats.shadow_skips;
            totals.shadows_drawn += stats.shadow_drawn;
            totals.filters_skipped += stats.filter_skips;
            totals.shaders_drawn += stats.shader_drawn;
            totals.shaders_skipped += stats.shader_skips;
            totals.texture_cache_hits += stats.cache_hits;
            totals.texture_cache_misses += stats.cache_misses;
            totals.fragments_encoded += 1;
            used_fragments.insert(key);
            // Budget enforcement limits *retention*, never drawing: an
            // over-budget node still appends its freshly encoded commands,
            // they are just not kept for the next frame to reuse.
            if !matches!(node, SceneNode::Shader { .. })
                && self.budget.may_cache_fragment(self.fragments.len())
            {
                self.fragments.insert(
                    key,
                    SceneFragment {
                        fingerprint,
                        scene: fragment_scene,
                        stats,
                    },
                );
                self.vello_scene
                    .append(&self.fragments[&key].scene, Some(shift * group_affine));
            } else {
                self.trim_telemetry.caching_refused += 1;
                self.vello_scene
                    .append(&fragment_scene, Some(shift * group_affine));
            }
        }
        // Close whatever the stream left open. The scene is allowed to end with
        // regions still pushed — nothing is drawn after the last node — but a
        // vello scene with an unterminated layer is a malformed command list,
        // not a harmless trailing instruction.
        while clip_depth > 0 {
            self.vello_scene.pop_layer();
            clip_depth -= 1;
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
                shadow,
            } => {
                let rect = KurboRect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1);
                // The shadow is drawn *first*, and the order is not cosmetic.
                // Vello composites in stream order, so a blurred silhouette
                // encoded after the fill would land on top of it and the card
                // would read as a dark smudge over its own face. Drawing the
                // blur first is what makes the fill sit on its own shadow,
                // exactly as the software backend draws the pair (it composites
                // the shadow and then the caller paints over it).
                if let Some(shadow) = shadow {
                    let blur = shadow_blur_rect(*bounds, f64::from(*radius), shadow);
                    if blur.std_dev > 0.0 && blur.rect.width() > 0.0 && blur.rect.height() > 0.0 {
                        target.draw_blurred_rounded_rect(
                            Affine::IDENTITY,
                            blur.rect,
                            blur.color,
                            blur.radius,
                            blur.std_dev,
                        );
                        stats.shadow_drawn += 1;
                    } else {
                        // A zero σ is the one thing the primitive cannot
                        // express — the shader divides by it — and a zero
                        // extent is a silhouette that eroded away. Both are
                        // counted as skips rather than drawn as something
                        // else: a hard-edged offset shape would need a plain
                        // fill, and an over-eroded one is legitimately nothing.
                        stats.shadow_skips += 1;
                    }
                }
                // One fill for both shapes: a zero radius is a plain rect.
                let shape = KurboRoundedRect::from_rect(rect, f64::from(radius.max(0.0)));
                fill_with_brush(target, &shape, fill);
                stats.rects += 1;
            }
            SceneNode::Path { path, fill, stroke } => {
                let bez = bez_path(path);
                if let Some(fill) = fill {
                    fill_with_brush(target, &bez, fill);
                }
                if let Some(stroke) = stroke {
                    let pen = KurboStroke {
                        width: stroke.width,
                        join: kurbo_join(stroke.join),
                        miter_limit: stroke.miter_limit,
                        start_cap: kurbo_cap(stroke.cap),
                        end_cap: kurbo_cap(stroke.cap),
                        ..Default::default()
                    };
                    target.stroke(
                        &pen,
                        Affine::IDENTITY,
                        PenikoColor::from_rgba8(
                            stroke.color.r,
                            stroke.color.g,
                            stroke.color.b,
                            stroke.color.a,
                        ),
                        None,
                        &bez,
                    );
                }
                stats.paths += 1;
            }
            SceneNode::Text {
                origin,
                glyphs,
                color,
                ..
            } => {
                let Some(ResourceKind::GlyphRun { runs, .. }) =
                    resources.get(*glyphs).map(|r| &r.kind)
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
                let key = image.raw();
                used_images.insert(key);
                stats.image_keys.push(key);
                // Texture cache: one ImageBrush per resource id. The
                // brush's Blob shares the registry's `Arc<[u8]>` and keeps
                // a stable id, which is what vello's atlas keys on —
                // rebuilding it per frame would re-upload every frame.
                // Budget enforcement: an image that would not *fit* is still
                // drawn — built for this frame and dropped after it — so a
                // full budget degrades to re-uploads, never to missing
                // content.
                let new_bytes = u64::from(*width) * u64::from(*height) * 4;
                // The refused-by-budget path has to own its brush for the
                // rest of this arm and hand out a borrow, so it parks it in
                // a slot declared outside the chain. `get_or_insert_with`
                // rather than `= Some(..)` + `.as_ref().expect(..)`: the
                // slot's initial value is genuinely read (it decides whether
                // to build), and there is no "just built" unwrap left to lie.
                let mut uncached: Option<ImageBrush> = None;
                let brush: &ImageBrush = if let Some(brush) = self.image_cache.get(&key) {
                    stats.cache_hits += 1;
                    brush
                } else if self
                    .budget
                    .may_cache_resource(self.resident_resource_bytes(), new_bytes)
                {
                    stats.cache_misses += 1;
                    // One Arc clone, no pixel bytes copied: the Blob
                    // shares ownership with the registry. The extra
                    // Arc layer exists because `Blob` wants an
                    // `Arc<dyn AsRef<[u8]>>` and an unsized pointee
                    // cannot coerce directly — this happens once per
                    // registration (cache miss), not per frame.
                    let shared = std::sync::Arc::new(std::sync::Arc::clone(pixels));
                    self.image_cache.insert(
                        key,
                        ImageBrush {
                            image: ImageData {
                                data: Blob::new(shared),
                                format: ImageFormat::Rgba8,
                                alpha_type: ImageAlphaType::AlphaPremultiplied,
                                width: *width,
                                height: *height,
                            },
                            sampler: ImageSampler::default(),
                        },
                    );
                    self.image_cache.get(&key).expect("just inserted")
                } else {
                    stats.cache_misses += 1;
                    self.trim_telemetry.caching_refused += 1;
                    let shared = std::sync::Arc::new(std::sync::Arc::clone(pixels));
                    uncached.get_or_insert_with(|| ImageBrush {
                        image: ImageData {
                            data: Blob::new(shared),
                            format: ImageFormat::Rgba8,
                            alpha_type: ImageAlphaType::AlphaPremultiplied,
                            width: *width,
                            height: *height,
                        },
                        sampler: ImageSampler::default(),
                    })
                };
                let sx = bounds.width() / f64::from(*width);
                let sy = bounds.height() / f64::from(*height);
                let transform =
                    Affine::translate((bounds.x0, bounds.y0)) * Affine::scale_non_uniform(sx, sy);
                draw_image_at(target, brush, transform, *width, *height, *alpha);
                stats.images += 1;
            }
            SceneNode::Shader { bounds, shader } => {
                // The software rasteriser and the skia adapter can't run WGSL
                // and count a skip; here the program is compiled and run on the
                // GPU, the output read back, and drawn back into the retained
                // scene as an image. vello 0.7 cannot import a GPU texture, so
                // the readback is the one way to keep a shader's z-order, clip,
                // transform and damage behaviour identical to a rect's — the
                // alternative (a separate pass over the finished raster) would
                // paint over occlusion and break the scene contract.
                let pw = (f64::from(self.scale) * bounds.width()).ceil().max(1.0) as u32;
                let ph = (f64::from(self.scale) * bounds.height()).ceil().max(1.0) as u32;
                let prog_key = hash_shader_wgsl(&shader.wgsl);
                if !self.shader_programs.contains_key(&prog_key) {
                    let module = self
                        .device
                        .create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: Some("floom-shader"),
                            source: wgpu::ShaderSource::Wgsl(
                                format!("{SHADER_PROLOGUE}\n{}", shader.wgsl).into(),
                            ),
                        });
                    let pipeline =
                        self.device
                            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                                label: Some("floom-shader-pipeline"),
                                layout: Some(&self.shader_pipeline_layout),
                                vertex: wgpu::VertexState {
                                    module: &module,
                                    entry_point: "vs_main".into(),
                                    compilation_options: wgpu::PipelineCompilationOptions::default(
                                    ),
                                    buffers: &[],
                                },
                                fragment: Some(wgpu::FragmentState {
                                    module: &module,
                                    entry_point: "fs_main".into(),
                                    compilation_options: wgpu::PipelineCompilationOptions::default(
                                    ),
                                    targets: &[Some(wgpu::ColorTargetState {
                                        format: wgpu::TextureFormat::Rgba8Unorm,
                                        blend: Some(wgpu::BlendState::REPLACE),
                                        write_mask: wgpu::ColorWrites::ALL,
                                    })],
                                }),
                                primitive: wgpu::PrimitiveState {
                                    topology: wgpu::PrimitiveTopology::TriangleList,
                                    strip_index_format: None,
                                    front_face: wgpu::FrontFace::Ccw,
                                    cull_mode: None,
                                    unclipped_depth: false,
                                    polygon_mode: wgpu::PolygonMode::Fill,
                                    conservative: false,
                                },
                                depth_stencil: None,
                                multisample: wgpu::MultisampleState {
                                    count: 1,
                                    mask: !0,
                                    alpha_to_coverage_enabled: false,
                                },
                                multiview: None,
                                cache: None,
                            });
                    // A failed program is never stored (wgpu 27 surfaces the
                    // error synchronously), so re-lookup below simply finds the
                    // cached pipeline and runs it.
                    self.shader_programs
                        .insert(prog_key, ShaderProgram::Pipeline(pipeline));
                }
                if let Some(ShaderProgram::Pipeline(pipeline)) = self.shader_programs.get(&prog_key)
                {
                    // The pipeline is cloned out so the `&mut self` call below
                    // does not fight the `&self` lookup borrow.
                    let pipeline = pipeline.clone();
                    if let Some(skey) =
                        self.run_shader_to_image(target, bounds, pw, ph, shader, &pipeline)
                    {
                        used_images.insert(skey);
                        stats.image_keys.push(skey);
                        stats.shader_drawn += 1;
                    } else {
                        stats.shader_skips += 1;
                    }
                } else {
                    stats.shader_skips += 1;
                }
            }
            SceneNode::BackdropFilter { .. } => {
                // Nothing to draw: what this node asks for is a filter over
                // pixels that have already been rasterised, which vello cannot
                // reach into. The skip is the record — a silent no-op would
                // leave a frosted panel reading as plain transparency.
                stats.filter_skips += 1;
            }
            // Markers are consumed by the walk in `encode`, before it reaches
            // here; reaching this arm would mean a marker slipped past the
            // clip/transform bookkeeping.
            SceneNode::ClipPush { .. } | SceneNode::ClipPop => {}
            SceneNode::GroupPush { .. } | SceneNode::GroupPop => {}
            SceneNode::FilterPush { .. } | SceneNode::FilterPop => {}
        }
        stats
    }

    /// Renders a custom-shader program into an offscreen `pw x ph` texture,
    /// reads the pixels back to the CPU, turns them into an [`ImageBrush`] that
    /// vello can composite, and draws that image into `target` at `bounds`.
    ///
    /// vello 0.7 has no way to import a GPU texture as a scene input, so the
    /// readback is the only route that keeps the shader's z-order, clip,
    /// transform and damage behaviour identical to a rect's — the pixels are
    /// an ordinary image node in the live scene. Returns the brush's cache key
    /// on success, `None` when the readback could not be mapped (never silently
    ///: the caller counts a skip so the degradation is visible).
    fn run_shader_to_image(
        &mut self,
        target: &mut VelloScene,
        bounds: &Rect,
        pw: u32,
        ph: u32,
        shader: &Shader,
        pipeline: &RenderPipeline,
    ) -> Option<u64> {
        if pw == 0 || ph == 0 {
            return None;
        }
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("floom-shader-output"),
            size: wgpu::Extent3d {
                width: pw,
                height: ph,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.shader_textures.push(texture);

        // Uniform block: resolution, time, the two param vec4s, packed into
        // 16 f32 so the WGSL prologue's layout is a straight slice.
        let mut uniforms = [0.0f32; 16];
        uniforms[0] = pw as f32;
        uniforms[1] = ph as f32;
        uniforms[2] = self.shader_clock;
        let mut i = 4;
        for v in shader.params.iter().copied().take(8) {
            uniforms[i] = v;
            i += 1;
        }
        let uniform = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("floom-shader-uniforms"),
            size: (uniforms.len() * 4) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.queue.write_buffer(&uniform, 0, {
            let ptr = uniforms.as_ptr().cast::<u8>();
            // # Safety: uniforms is an owned array living for this block, the
            // slice is exactly its byte length. Writing a f32 vector as bytes
            // to a uniform buffer matches how `write_buffer` consumes any
            // plain data.
            unsafe { std::slice::from_raw_parts(ptr, uniforms.len() * 4) }
        });

        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("floom-shader-bind"),
            layout: &self.shader_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("floom-shader-encode"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("floom-shader-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // The prologue's fullscreen triangle: vertex_index 0..3.
            pass.draw(0..3, 0..1);
        }

        // Read back the rendered RGBA into a CPU buffer. `bytes_per_row` for a
        // texel copy must be a multiple of 256, so pad the stride and copy each
        // row out of the padded buffer afterwards.
        let row_bytes = pw * 4;
        let stride = 256u32 * row_bytes.div_ceil(256);
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("floom-shader-staging"),
            size: u64::from(stride) * u64::from(ph),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.shader_textures.last().expect("pushed above"),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(ph),
                },
            },
            wgpu::Extent3d {
                width: pw,
                height: ph,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);

        let slice = staging.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res.is_ok());
        });
        // Block until the readback lands so the mapping below is valid; a
        // timeout/error means the map callback never fired happy.
        match self.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        }) {
            Err(_) => return None,
            Ok(_) => {}
        }
        if !rx.recv().unwrap_or(false) {
            return None;
        }
        let mapped = slice.get_mapped_range();
        let mut pixels = Vec::with_capacity(pw as usize * ph as usize * 4);
        for y in 0..ph {
            let row_start = y as usize * stride as usize;
            let row = &mapped[row_start..row_start + (pw as usize) * 4];
            pixels.extend_from_slice(row);
        }
        drop(mapped);
        staging.unmap();

        // Composite the readback as an ordinary image node, scaled to its
        // logical bounds (the scene is later transformed by `shift`, so the
        // draw must stay in logical units — matching the brush `Image` arm).
        // `Arc::new(pixels)` yields `Arc<Vec<u8>>`, which unsizes to the
        // `Arc<dyn AsRef<[u8]>>` Blob wants.
        let shared: std::sync::Arc<dyn AsRef<[u8]> + Send + Sync> = std::sync::Arc::new(pixels);
        let brush = ImageBrush {
            image: ImageData {
                data: Blob::new(std::sync::Arc::clone(&shared)),
                format: ImageFormat::Rgba8,
                alpha_type: ImageAlphaType::AlphaPremultiplied,
                width: pw,
                height: ph,
            },
            sampler: ImageSampler::default(),
        };
        let sx = bounds.width() / f64::from(pw);
        let sy = bounds.height() / f64::from(ph);
        let transform =
            Affine::translate((bounds.x0, bounds.y0)) * Affine::scale_non_uniform(sx, sy);
        draw_image_at(target, &brush, transform, pw, ph, u8::MAX);
        let key = shader_cache_key(&shader.wgsl, pw, ph);
        self.image_cache.insert(key, brush);
        Some(key)
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
        damage: &floom_scene::DamageRegion,
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
        label: Some("floom-vello-target"),
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
        label: Some("floom-vello-scratch"),
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

    fn import_external_texture(&mut self, frame: ExternalFrame) {
        // One call per snapshot: the app runner drains the host's pending
        // captures and imports each exactly once, before the frame that first
        // draws the frozen texture. The default no-op is for backends with no
        // texture path; vello resolves the participant's `SceneNode::Image`
        // from the brush keyed on `frame.resource`.
        let _ = self.import_texture(frame);
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
            self.stats.shadows_skipped = 0;
            self.stats.shadows_drawn = 0;
            self.stats.filters_skipped = 0;
            self.stats.shaders_drawn = 0;
            self.stats.shaders_skipped = 0;
            return;
        }
        // Any non-skipped present refreshes the screen (or would, with a
        // surface): the "stale screen" reason to present is consumed. It also
        // decides the rasterisation area — after an attach/resize the target
        // is a brand-new texture with undefined content, so only a full-frame
        // rasterisation can fill it.
        let stale_screen = self.needs_present;
        self.needs_present = false;

        // A non-skipped present is what drives an animated shader: advance the
        // clock handed to custom programs, and drop last frame's offscreen
        // shader textures (the vello pass they fed has already submitted).
        self.shader_clock += 1.0 / 60.0;
        self.shader_textures.clear();

        // Physical extent of the target, whether or not a surface is attached.
        let (pw, ph) = match self.surface.as_ref() {
            Some(surface) => (surface.config.width, surface.config.height),
            None => (
                (self.size.width.max(0.0) * self.scale).ceil() as u32,
                (self.size.height.max(0.0) * self.scale).ceil() as u32,
            ),
        };
        let mut region = if stale_screen {
            None
        } else {
            Self::damage_region(scene.damage(), pw, ph, self.scale)
        };
        // Partial rasterisation needs a live surface and a scratch view big
        // enough for the region; if the budget refused (or trimmed) the
        // scratch, fall back to a full-frame pass rather than dropping the
        // frame.
        if self.surface.is_some() {
            if let Some((_, _, w, h)) = region {
                if self.scratch_for(w, h).is_none() {
                    region = None;
                }
            }
        }
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
        self.stats.shadows_skipped = encoded.shadows_skipped;
        self.stats.shadows_drawn = encoded.shadows_drawn;
        self.stats.filters_skipped = encoded.filters_skipped;
        self.stats.shaders_drawn = encoded.shaders_drawn;
        self.stats.shaders_skipped = encoded.shaders_skipped;
        // Prepare the scratch view up front: both it and the surface live on
        // `self`, and the surface is borrowed mutably below.
        let scratch = if self.surface.is_some() {
            region.and_then(|(_, _, w, h)| self.scratch_for(w, h))
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
                            label: Some("floom damage copy"),
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
                label: Some("floom surface blit"),
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

    /// Shadows on rects are drawn, filters are not, shaders are.
    ///
    /// vello 0.7 grew a blur primitive (`draw_blurred_rounded_rect`), so a
    /// rect's drop shadow is drawn rather than skipped. The bit is `true`
    /// because the *shadow* is honoured, not because every shadow is: the
    /// primitive blurs a rounded rectangle, so a path or a glyph run carrying
    /// a shadow still has no representation and still lands in
    /// `shadows_skipped`. A caller that needs to know which shapes get
    /// shadows reads that counter, not this bit — a bit cannot carry a
    /// per-shape answer, and one that claimed to would be lying.
    ///
    /// Filters remain `false` for the same reason they always were: a filter
    /// is an operation on pixels that have *already* been drawn, and vello
    /// has no way to reach into its own raster. A custom shader it *can* run:
    /// the WGSL program is compiled into a pipeline and rendered.
    fn capabilities(&self) -> RendererCapabilities {
        RendererCapabilities {
            shadows: true,
            filters: false,
            backdrop_filters: false,
            shaders: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The raw draw-tag word for vello's blurred-rounded-rect command.
    ///
    /// `vello_encoding::DrawTag::BLUR_RECT` is not re-exported by `vello` —
    /// only `Glyph` and `NormalizedCoord` are — so a test that wants to
    /// *name* the command has to either add a dependency on a transitive
    /// crate or spell the value. Spelling it is the smaller coupling: this is
    /// a wire-format constant that changes only when the encoding changes,
    /// and if it ever does, these tests are exactly where the breakage
    /// should surface.
    const BLUR_RECT_TAG: u32 = 0x2d4;

    /// GPU tests are opportunistic: CI machines and containers frequently have
    /// no usable adapter, and "no GPU" is a supported configuration (the app
    /// falls back to the software presenter), so these tests skip rather than
    /// fail in that case.
    fn presenter() -> Option<VelloGpuPresenter> {
        VelloGpuPresenter::new()
    }

    /// A shadow, folded into the blur primitive's own parameters.
    ///
    /// The arithmetic is pure, so these run without an adapter — which is the
    /// point: the mapping from CSS's four shadow lengths to vello's three
    /// numbers is where a plausible-looking wrong answer hides, and it is
    /// exactly the part that needs no GPU to be wrong.
    fn shadow(blur: f64, off_x: f64, off_y: f64, spread: f64) -> floom_types::Shadow {
        floom_types::Shadow {
            color: floom_types::Color::rgba(0, 0, 0, 160),
            blur,
            offset_x: off_x,
            offset_y: off_y,
            spread,
        }
    }

    #[test]
    fn a_shadows_offset_moves_the_blurred_rect() {
        // Vello's primitive has no offset of its own: it blurs whatever rect
        // it is handed, where that rect is. So the offset has to be folded
        // into the rect, and the centre is what moves.
        let bounds = Rect {
            x0: 10.0,
            y0: 10.0,
            x1: 30.0,
            y1: 30.0,
        };
        let b = shadow_blur_rect(bounds, 0.0, &shadow(0.0, 4.0, -2.0, 0.0));
        assert_eq!(b.rect.center(), KurboPoint::new(24.0, 18.0));
        // The size is untouched: an offset moves a shadow, it does not grow it.
        assert_eq!(b.rect.width(), 20.0);
        assert_eq!(b.rect.height(), 20.0);
    }

    #[test]
    fn spread_grows_the_silhouette_not_the_blur() {
        // CSS's fourth `box-shadow` length dilates the silhouette *before*
        // the blur. The plausible-looking wrong implementation widens the
        // blur instead, which reads as "a fatter, softer shadow" — a
        // different effect, and the one `shadow_spread.rs` pins down on the
        // software side. Here the discriminator is that the rect grows by
        // exactly the spread on every side while the sigma stays put.
        let bounds = Rect {
            x0: 10.0,
            y0: 10.0,
            x1: 30.0,
            y1: 30.0,
        };
        let b = shadow_blur_rect(bounds, 3.0, &shadow(6.0, 0.0, 0.0, 5.0));
        assert_eq!(b.rect.width(), 30.0, "spread dilates on both sides");
        assert_eq!(b.rect.height(), 30.0);
        assert_eq!(
            b.radius, 8.0,
            "the corner radius grows to match, so the silhouette stays a rounded box"
        );
        // The blur is the blur: 6.0 CSS px, not 6.0 + the spread.
        assert!((b.std_dev - 6.0 * CSS_BLUR_TO_SIGMA).abs() < 1e-12);
    }

    #[test]
    fn a_negative_spread_erodes_and_cannot_close_past_the_centre() {
        // A negative spread erodes, and one larger than the shape would leave
        // a negative width — which the primitive cannot take. Clamped to zero
        // rather than wrapped: an over-eroded silhouette legitimately
        // vanishes, and the caller's zero-extent guard skips it.
        let bounds = Rect {
            x0: 10.0,
            y0: 10.0,
            x1: 30.0,
            y1: 30.0,
        };
        let b = shadow_blur_rect(bounds, 2.0, &shadow(0.0, 0.0, 0.0, -3.0));
        assert_eq!(b.rect.width(), 14.0, "3px in from each side");

        let gone = shadow_blur_rect(bounds, 2.0, &shadow(0.0, 0.0, 0.0, -100.0));
        assert!(gone.rect.width() >= 0.0, "never a negative extent");
        assert_eq!(
            gone.rect.width(),
            0.0,
            "an over-eroded silhouette is nothing"
        );
    }

    #[test]
    fn the_blur_is_a_sigma_not_a_radius() {
        // CSS's blur length and a Gaussian's sigma are different numbers, and
        // using one for the other makes every shadow the wrong softness. The
        // ratio is sqrt(ln 2 / 2).
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 20.0,
            y1: 20.0,
        };
        let b = shadow_blur_rect(bounds, 0.0, &shadow(10.0, 0.0, 0.0, 0.0));
        assert!((b.std_dev - 10.0 * CSS_BLUR_TO_SIGMA).abs() < 1e-12);
        assert!(
            (b.std_dev - 10.0).abs() > 1.0,
            "10 CSS px of blur is not 10 of sigma — that would be visibly too soft"
        );
    }

    #[test]
    fn the_shadow_colour_travels_with_the_fold() {
        // The primitive takes a plain colour, so the shadow's own tint and
        // alpha are what gets blurred. A shadow that lost its alpha here
        // would draw as a hard dark silhouette instead.
        let bounds = Rect {
            x0: 0.0,
            y0: 0.0,
            x1: 10.0,
            y1: 10.0,
        };
        let mut s = shadow(4.0, 0.0, 0.0, 0.0);
        s.color = floom_types::Color::rgba(20, 40, 60, 128);
        let b = shadow_blur_rect(bounds, 0.0, &s);
        assert_eq!(b.color, PenikoColor::from_rgba8(20, 40, 60, 128));
    }

    /// A shadow must reach vello's command stream, not just the counters.
    ///
    /// `shadows_drawn` going up is the *claim*; this is the *evidence*. The
    /// encoding is public (`Scene::encoding`), so the test drives the same
    /// call the encoder makes and then walks the draw tags looking for
    /// `BLUR_RECT`. A backend that counted a shadow and encoded nothing would
    /// pass every counter test and fail here.
    ///
    /// No GPU needed, which is what makes it worth having: the GPU path is
    /// otherwise unverified on every machine without an adapter.
    #[test]
    fn a_rects_shadow_reaches_the_command_stream() {
        let bounds = Rect {
            x0: 10.0,
            y0: 10.0,
            x1: 30.0,
            y1: 30.0,
        };
        let s = shadow(6.0, 4.0, 4.0, 5.0);
        let folded = shadow_blur_rect(bounds, 3.0, &s);

        // The exact call `encode_node`'s Rect arm makes.
        let mut scene = VelloScene::new();
        scene.draw_blurred_rounded_rect(
            Affine::IDENTITY,
            folded.rect,
            folded.color,
            folded.radius,
            folded.std_dev,
        );

        let tags: Vec<u32> = scene.encoding().draw_tags.iter().map(|tag| tag.0).collect();
        let blur_rects = tags.iter().filter(|&&raw| raw == BLUR_RECT_TAG).count();
        assert_eq!(
            blur_rects, 1,
            "exactly one blurred rect reached the stream: the tags were {tags:?}"
        );
    }

    /// A shadowless rect must not put a blur command in the stream.
    ///
    /// The control for the test above. "A blur tag is present" only means
    /// something next to "the same scene without a shadow has none" —
    /// otherwise the tag could be coming from anything the encoder does
    /// anyway.
    #[test]
    fn a_shadowless_rect_emits_no_blur_command() {
        let mut scene = VelloScene::new();
        let shape = KurboRoundedRect::from_rect(KurboRect::new(10.0, 10.0, 30.0, 30.0), 3.0);
        fill_with_brush(
            &mut scene,
            &shape,
            &floom_types::Brush::Solid(floom_types::Color::rgb(200, 200, 200)),
        );

        let has_blur = scene
            .encoding()
            .draw_tags
            .iter()
            .any(|tag| tag.0 == BLUR_RECT_TAG);
        assert!(
            !has_blur,
            "a plain fill is not a blur: no shadow was asked for"
        );
    }

    /// The fragment cache must not serve a shadow across a change in it.
    ///
    /// A fragment is keyed on the node fingerprint and bakes its commands, so
    /// a shadow left out of the fingerprint would keep serving the fragment
    /// built with the old one — a card whose shadow stopped following its
    /// `box-shadow`, arriving through the cache rather than through the
    /// scene. No GPU needed: the fingerprint is pure.
    #[test]
    fn a_shadow_is_part_of_a_nodes_fingerprint() {
        let node = |shadow: Option<floom_types::Shadow>| SceneNode::Rect {
            bounds: Rect {
                x0: 0.0,
                y0: 0.0,
                x1: 20.0,
                y1: 10.0,
            },
            fill: floom_types::Brush::Solid(floom_types::Color::rgb(200, 200, 200)),
            radius: 4.0,
            shadow,
        };
        let with = node(Some(shadow(6.0, 2.0, 2.0, 1.0)));
        let without = node(None);
        assert_ne!(
            node_fingerprint(&with),
            node_fingerprint(&without),
            "a shadow is a visible difference, so it must be part of the key"
        );
        // And a *different* shadow is a different picture too.
        let other = node(Some(shadow(6.0, 2.0, 2.0, 4.0)));
        assert_ne!(
            node_fingerprint(&with),
            node_fingerprint(&other),
            "spread is part of the silhouette, so it must be part of the key"
        );
    }

    /// The scene→target transform must scale *and* shift, in that order.
    ///
    /// Pure arithmetic, so it runs without an adapter — which is the point:
    /// the bug it pins (a logical scene painted 1:1 into a device-pixel target)
    /// is invisible in any test whose surface is 1×, and CI machines and
    /// emulators are frequently not.
    #[test]
    fn the_scene_transform_scales_then_translates() {
        use vello::kurbo::Point;

        // Logical (3,5) on a 2× display whose damaged region starts at device
        // (4,8): scale to (6,10), then subtract the region origin.
        let t = scene_transform((4.0, 8.0), 2.0);
        assert_eq!(t * Point::new(3.0, 5.0), Point::new(2.0, 2.0));

        // Translating first would give (3-4)*2, (5-8)*2: the offset scaled
        // along with the scene, which is what a partial rasterisation must not
        // do.
        assert_ne!(t * Point::new(3.0, 5.0), Point::new(-2.0, -6.0));

        // At 1× with no damage offset it is the identity, so every existing 1×
        // expectation keeps meaning what it did.
        let identity = scene_transform((0.0, 0.0), 1.0);
        assert_eq!(identity * Point::new(7.0, 9.0), Point::new(7.0, 9.0));
    }

    /// Two image nodes differing only in alpha must not share a fragment.
    ///
    /// The fragment cache keys on this fingerprint and bakes the brush into the
    /// encoded commands, so an alpha left out of the key would keep serving the
    /// fragment built at the old alpha — a picture frozen mid-fade, arriving
    /// through the cache rather than through the clock. No GPU needed: the
    /// fingerprint is pure.
    /// A filter's radii are part of a node's fingerprint.
    ///
    /// The fragment cache keys on this hash, so radii left out of it would keep
    /// serving the fragment encoded at the old blur — a frost that never
    /// deepens, arriving through the cache rather than through the value.
    #[test]
    fn a_filters_radii_are_part_of_its_fingerprint() {
        let push = |filter| node_fingerprint(&SceneNode::FilterPush { filter });
        assert_ne!(
            push(floom_types::Filter::blur(3.0)),
            push(floom_types::Filter::blur(6.0)),
            "a deeper blur is a different encoding"
        );
        assert_ne!(
            push(floom_types::Filter::blur_xy(6.0, 1.0)),
            push(floom_types::Filter::blur_xy(1.0, 6.0)),
            "and the two axes are not interchangeable"
        );
        assert_ne!(
            push(floom_types::Filter::blur(4.0)),
            node_fingerprint(&SceneNode::FilterPop),
            "a push and a pop are different nodes"
        );
    }

    #[test]
    fn an_images_alpha_is_part_of_its_fingerprint() {
        let bounds = floom_types::Rect::from_origin_size(
            floom_types::Point::new(1.0, 2.0),
            Size::new(8.0, 4.0),
        );
        let image = floom_types::ResourceId::new(0, 1);
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
            bounds: floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(4.0, 4.0),
            ),
            fill: floom_types::Brush::from(floom_types::Color::rgb(1, 2, 3)),
            radius: 0.0,

            shadow: None,
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
        let img = registry.insert(floom_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![255u8; 16].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(1.0, 2.0),
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
        let img = registry.insert(floom_scene::Resource {
            kind: ResourceKind::Image {
                width: 4,
                height: 4,
                pixels: std::sync::Arc::from(vec![7u8; 64].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
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
        let img_a = registry.insert(floom_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![1u8; 16].into_boxed_slice()),
            },
        });
        let img_b = registry.insert(floom_scene::Resource {
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
                floom_types::Rect::from_origin_size(
                    floom_types::Point::new(0.0, 0.0),
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
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
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
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(8.0, 4.0),
            ),
            floom_types::ResourceId::new(99, 1),
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
            SceneNode::GroupPush {
                transform: floom_types::Transform::IDENTITY,
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
            bounds: floom_types::Rect::from_origin_size(
                floom_types::Point::new(x, 0.0),
                Size::new(4.0, 4.0),
            ),
            fill: floom_types::Brush::from(floom_types::Color::rgb(1, 2, 3)),
            radius: 0.0,

            shadow: None,
        }
    }

    #[test]
    fn empty_damage_skips_present_until_resize() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut registry = ResourceRegistry::new();
        let img = registry.insert(floom_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![9u8; 16].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
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

    // --- Phase 28.2: cache budgets -----------------------------------------

    /// A fragment budget caps *retention*, not drawing: an over-budget scene
    /// still reports every node's draws, and the refusals are counted.
    #[test]
    fn a_fragment_budget_limits_retention_but_not_drawing() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        presenter.set_budget(crate::budget::GpuCacheBudget {
            max_fragments: 1,
            ..crate::budget::GpuCacheBudget::for_platform(crate::budget::PlatformClass::Desktop)
        });
        let mut scene = Scene::new();
        scene.begin_update();
        for x in 0..3u32 {
            let _ = scene.insert(rect_node(f64::from(x) * 8.0));
        }
        scene.finish_update();
        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.rects, 3, "every node is drawn");
        assert!(
            presenter.fragment_count() <= 1,
            "only the budgeted number of fragments is retained"
        );
        assert!(
            presenter.trim_telemetry().caching_refused >= 2,
            "refused caching is counted"
        );
        // The scene still re-encodes what it did not retain: presenting again
        // re-encodes everything, never draws from a phantom cache.
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        presenter.present(&scene, &ResourceRegistry::new());
        assert_eq!(presenter.stats().rects, 3);
    }

    /// A resource budget refuses to *retain* an oversized image but still
    /// draws it, and a critical trim sheds every derived cache.
    #[test]
    fn resource_budget_and_critical_trim_behave() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        presenter.set_budget(crate::budget::GpuCacheBudget {
            max_resource_bytes: 8, // one 2×2 image is 16 bytes: does not fit
            ..crate::budget::GpuCacheBudget::for_platform(crate::budget::PlatformClass::Desktop)
        });
        let mut registry = ResourceRegistry::new();
        let img = registry.insert(floom_scene::Resource {
            kind: ResourceKind::Image {
                width: 2,
                height: 2,
                pixels: std::sync::Arc::from(vec![9u8; 16].into_boxed_slice()),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(6.0, 6.0),
            ),
            img,
        ));
        scene.finish_update();
        presenter.present(&scene, &registry);
        let stats = presenter.stats();
        assert_eq!(stats.images, 1, "the image is drawn regardless");
        assert!(
            presenter.trim_telemetry().caching_refused >= 1,
            "the oversized retention is refused and counted"
        );

        // Back to a fitting budget: two image nodes cache, then Critical
        // sheds them along with the encode fragments.
        presenter.set_budget(crate::budget::GpuCacheBudget::for_platform(
            crate::budget::PlatformClass::Desktop,
        ));
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        presenter.present(&scene, &registry);
        assert_eq!(presenter.resident_resources(), 1);
        assert!(presenter.fragment_count() >= 1);

        presenter.trim(MemoryPressure::Critical);
        let telemetry = presenter.trim_telemetry();
        assert!(telemetry.trims >= 1);
        assert!(
            telemetry.fragments_dropped >= 1,
            "geometry is the first thing to go"
        );
        assert_eq!(
            telemetry.resources_dropped, 1,
            "resident textures are dropped by a critical trim"
        );
        assert_eq!(presenter.fragment_count(), 0);
        assert_eq!(presenter.resident_resources(), 0);
        // The scene still renders after the trim: caches are derived state.
        presenter.resize(Size::new(64.0, 48.0), 1.0);
        presenter.present(&scene, &registry);
        assert_eq!(presenter.stats().images, 1);
    }

    // --- external textures (16.3) ------------------------------------------

    /// A scene whose only node draws `texture` — the shape a `Texture`-mode
    /// participant has.
    fn scene_drawing(texture: floom_types::ResourceId) -> Scene {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::image(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(8.0, 8.0),
            ),
            texture,
        ));
        scene.finish_update();
        scene
    }

    /// A producer's frame: the pixels are the producer's, so the registry holds
    /// no `Image` for this id at all.
    fn external(texture: floom_types::ResourceId, sequence: u64) -> ExternalFrame {
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
        let texture = registry.insert(floom_scene::Resource {
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
        let texture = registry.insert(floom_scene::Resource {
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
        let texture = registry.insert(floom_scene::Resource {
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

    /// A `Shader` node's fingerprint must carry the WGSL and both parameter
    /// vecs, so a changed program (or a changed scalar) re-encodes instead of
    /// serving a stale fragment. Pure arithmetic — no adapter needed.
    #[test]
    fn a_shaders_source_and_params_are_part_of_its_fingerprint() {
        let bounds = floom_types::Rect::from_origin_size(
            floom_types::Point::new(0.0, 0.0),
            Size::new(8.0, 4.0),
        );
        let at = |wgsl: &str, params: Vec<f32>| {
            node_fingerprint(&SceneNode::Shader {
                bounds,
                shader: Shader::new(wgsl, params),
            })
        };
        let src = "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
                   return vec4<f32>(in.uv, 0.0, 1.0); }";
        assert_ne!(
            at(src, vec![0.5]),
            at(src, vec![0.75]),
            "a changed parameter is a different encoding"
        );
        assert_ne!(
            at(src, vec![0.5]),
            at("let d = 3.0;", vec![0.5]),
            "a changed source is a different encoding"
        );
        assert_eq!(at(src, vec![0.5]), at(src, vec![0.5]), "stable payload");
    }

    /// Shader readback cache keys must separate the program from the target
    /// size: a resize must never reuse the earlier frame's brush.
    #[test]
    fn a_shader_cache_key_separates_source_from_size() {
        let src = "fn red() { }";
        assert_ne!(
            shader_cache_key(src, 4, 4),
            shader_cache_key(src, 8, 4),
            "a larger target is a different rasterisation"
        );
        assert_ne!(
            shader_cache_key(src, 4, 4),
            shader_cache_key(src, 4, 8),
            "a taller target is also a different rasterisation"
        );
        assert_ne!(
            shader_cache_key(src, 4, 4),
            shader_cache_key("fn blue() { }", 4, 4),
            "a different program never collides"
        );
        assert_eq!(
            shader_cache_key(src, 4, 4),
            shader_cache_key(src, 4, 4),
            "the same program and size is stable"
        );
    }

    /// A `Shader` node must encode through the GPU backend's offscreen path,
    /// counting a draw rather than a skip. Adapter-gated like every other GPU
    /// test: machines without a usable adapter skip, because "no GPU" is a
    /// supported configuration (the host falls back to the software backend).
    #[test]
    fn shader_nodes_encode_through_the_gpu_backend() {
        let Some(mut presenter) = presenter() else {
            return;
        };
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::shader(
            floom_types::Rect::from_origin_size(
                floom_types::Point::new(0.0, 0.0),
                Size::new(4.0, 4.0),
            ),
            Shader::new(
                "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
                 return vec4<f32>(in.uv, 0.5, 1.0); }",
                [0.5],
            ),
        ));
        scene.finish_update();

        presenter.present(&scene, &ResourceRegistry::new());
        let stats = presenter.stats();
        assert_eq!(stats.unsupported_nodes, 0, "the shader is understood");
        assert_eq!(stats.shaders_drawn, 1, "an offscreen readback happened");
        assert_eq!(stats.shaders_skipped, 0, "nothing was degraded");
        assert_eq!(presenter.frames_presented(), 0, "nothing presented");
    }
}
