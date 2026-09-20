//! CPU rasterizer adapter: fills scene rects into an RGBA8 buffer.
//!
//! The buffer holds **straight (non-premultiplied)** RGBA, matching what the
//! plain fill path writes. Rounded rects get analytic anti-aliasing: coverage
//! comes from the rounded-rect signed distance evaluated at the pixel centre,
//! and edge pixels blend src-over so curves land between pixels instead of
//! being chopped to the integer grid.
//!
//! ## Partial redraw
//!
//! The renderer is damage-aware: `begin_frame` no longer wipes the buffer.
//! Each damage rect is cleared and fully repainted (every scene node is drawn
//! again, clipped to that rect), regions outside the damage are left
//! untouched, and a frame with empty damage is skipped entirely. A buffer
//! resize forces one full clear, and so does a full-frame damage report.
//!
//! ## Text and images
//!
//! With a [`ResourceRegistry`] (via `render_scene_with_resources`), `Text`
//! nodes rasterise their glyph runs through swash and `Image` nodes blit
//! premultiplied RGBA8 (the registry's format) converted to the buffer's
//! straight alpha. Without resources those nodes are skipped, never fatal.

mod presenter;
mod raster;

pub use presenter::SoftwarePresenter;

use floem_renderer::{FrameInfo, RenderTarget, Renderer};
use floem_scene::{DamageRegion, Rect, ResourceKind, ResourceRegistry, Scene, SceneNode};
use floem_types::{Color, Point, Polyline};
use raster::{ClipStack, Scissor};
use std::collections::HashMap;

/// Pixel-space clip region: `(x0, y0, x1, y1)`, exclusive right/bottom.
type Clip = (u32, u32, u32, u32);

/// Cache key for one rasterised glyph: (font-bytes pointer, font collection
/// index, font size bits, glyph id). The cached entry keeps a clone of the
/// font data alive, so the pointer cannot be recycled while the entry exists
/// and can never alias a different font.
type GlyphCacheKey = (usize, u32, u32, u32);

/// Default ceiling on cached glyph masks. UI text repeats a small working set
/// of glyphs, but dynamic content (scrolling document text, many font sizes)
/// can grow the set without bound — the cache evicts least-recently-used
/// entries beyond this, trading a re-rasterisation for a bounded footprint.
const GLYPH_CACHE_LIMIT: usize = 4096;

/// Default ceiling on the mask *bytes* the cache holds. Masks vary wildly in
/// size — a 10px glyph is tens of bytes, a 96px one is tens of kilobytes — so
/// an entry count alone says nothing about the memory actually retained. This
/// is the real budget; the entry ceiling above is a backstop against a
/// pathological pile of tiny glyphs.
const GLYPH_CACHE_BYTES: usize = 4 * 1024 * 1024;

/// Software renderer backend implementing the Floem 2.0 [`Renderer`] ABI.
pub struct SoftwareRenderer {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pub frames: u64,
    pub rects_drawn: u64,
    /// Glyph mask pixels blended (across all text nodes).
    pub glyphs_drawn: u64,
    /// Image nodes blitted.
    pub images_drawn: u64,
    /// Frames skipped because the damage region was empty.
    pub damage_skips: u64,
    /// Glyph rasterisations served from the bitmap cache.
    pub glyph_cache_hits: u64,
    /// Glyph rasterisations that had to run swash (first sighting, or after
    /// a cache clear).
    pub glyph_cache_misses: u64,
    /// Scene nodes culled because their bounds miss every damaged rect.
    pub nodes_skipped: u64,
    /// Set after (re)creation/resize: the next render must clear everything,
    /// since the buffer content cannot be trusted.
    needs_full_clear: bool,
    /// Rasterised alpha masks, keyed by (font, size, glyph id). Each entry
    /// holds a clone of the font's `FontData` so the key's byte pointer stays
    /// valid and unique for as long as the entry lives, plus the logical
    /// clock stamp of its most recent use — the LRU order. UI glyphs repeat
    /// massively across frames and nodes — without this every frame pays a
    /// full outline-to-mask pass per visible glyph.
    glyph_cache: HashMap<GlyphCacheKey, (peniko::Blob<u8>, std::sync::Arc<GlyphMask>, u64)>,
    /// Mask bytes currently held by `glyph_cache` (sum of every entry's mask).
    glyph_cache_bytes: usize,
    /// Maximum number of cached masks; the least-recently-used entries are
    /// evicted beyond it.
    glyph_cache_limit: usize,
    /// Ceiling on retained mask bytes; see [`GLYPH_CACHE_BYTES`].
    glyph_cache_byte_limit: usize,
    /// Monotonic access clock feeding the LRU order.
    glyph_cache_clock: u64,
    /// Device pixels per logical unit, as reported by the frame. Scene geometry
    /// — and glyph pen positions, and the size glyphs rasterise at — is in
    /// logical units; the buffer is in device pixels, so every coordinate is
    /// multiplied by this on the way in. A renderer that ignores it paints the
    /// scene into a `scale`-times-larger buffer at 1:1, i.e. the app drawn at
    /// half size on a 2× display.
    scale: f64,
    /// Clip regions pushed by the scene's `ClipPush` markers. Cleared at the
    /// start of every walk, because the marker stream is allowed to end with
    /// regions still pushed and nothing drawn after them.
    clips: ClipStack,
    /// Flattened device-space contours for the path node currently being
    /// drawn, kept between calls so a chart that draws every frame does not
    /// re-allocate its outline every frame.
    contours: Vec<Vec<Point>>,
    /// Coverage buffer shared by every path fill. Sized to the shape, not the
    /// frame.
    coverage: Vec<f32>,
    /// Path nodes drawn (fills and strokes), for the frame stats.
    pub paths_drawn: u64,
    /// `ClipPush` markers that resolved to a real window this frame, for the
    /// frame stats and backend-parity contract tests. A push whose region is
    /// empty (a clip to nothing) is not counted, because nothing under it is
    /// ever visible — that is the same "no clip" the host would have produced.
    pub clips_resolved: u64,
}

impl SoftwareRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        let len = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0; len],
            frames: 0,
            rects_drawn: 0,
            glyphs_drawn: 0,
            images_drawn: 0,
            damage_skips: 0,
            nodes_skipped: 0,
            glyph_cache_hits: 0,
            glyph_cache_misses: 0,
            needs_full_clear: true,
            glyph_cache: HashMap::new(),
            glyph_cache_limit: GLYPH_CACHE_LIMIT,
            glyph_cache_bytes: 0,
            glyph_cache_byte_limit: GLYPH_CACHE_BYTES,
            glyph_cache_clock: 0,
            scale: 1.0,
            clips: ClipStack::new(),
            contours: Vec::new(),
            coverage: Vec::new(),
            paths_drawn: 0,
            clips_resolved: 0,
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Number of glyph masks currently cached.
    pub fn glyph_cache_entries(&self) -> usize {
        self.glyph_cache.len()
    }

    /// The configured ceiling on cached glyph masks.
    pub fn glyph_cache_limit(&self) -> usize {
        self.glyph_cache_limit
    }

    /// Mask bytes currently held by the cache.
    pub fn glyph_cache_bytes(&self) -> usize {
        self.glyph_cache_bytes
    }

    /// The configured ceiling on cached mask bytes.
    pub fn glyph_cache_byte_limit(&self) -> usize {
        self.glyph_cache_byte_limit
    }

    /// Changes the cache ceiling, evicting least-recently-used entries until
    /// the cache fits. A limit of zero is clamped to one: the cache always
    /// holds the most recently used mask.
    pub fn set_glyph_cache_limit(&mut self, limit: usize) {
        self.glyph_cache_limit = limit.max(1);
        while self.glyph_cache.len() > self.glyph_cache_limit {
            self.evict_glyph_cache_one();
        }
    }

    /// Changes the byte budget, evicting least-recently-used entries until the
    /// cache fits. A mask costs its own bytes, so the budget is floored at
    /// what a single large glyph needs — below that the cache could not hold
    /// anything and every frame would re-rasterise everything.
    pub fn set_glyph_cache_byte_limit(&mut self, limit: usize) {
        self.glyph_cache_byte_limit = limit.max(64 * 1024);
        while self.glyph_cache_bytes > self.glyph_cache_byte_limit {
            self.evict_glyph_cache_one();
        }
    }

    /// Drops all cached glyph masks (e.g. under memory pressure). The next
    /// frame re-rasterises on demand.
    pub fn clear_glyph_cache(&mut self) {
        self.glyph_cache.clear();
        self.glyph_cache_bytes = 0;
    }

    /// Evicts the single least-recently-used mask, releasing its bytes from the
    /// budget. No-op on an empty cache.
    fn evict_glyph_cache_one(&mut self) {
        let oldest = self
            .glyph_cache
            .iter()
            .min_by_key(|(_, value)| value.2)
            .map(|(key, value)| (*key, value.1.data.len()));
        if let Some((key, bytes)) = oldest {
            self.glyph_cache.remove(&key);
            self.glyph_cache_bytes = self.glyph_cache_bytes.saturating_sub(bytes);
        }
    }

    /// Makes room for a mask of `bytes`, then charges it to the budget.
    ///
    /// Both ceilings apply: the byte budget is what bounds memory, the entry
    /// count is a backstop against a pathological pile of tiny glyphs.
    fn admit_glyph_cache_entry(&mut self, bytes: usize) {
        while (self.glyph_cache_bytes + bytes > self.glyph_cache_byte_limit
            || self.glyph_cache.len() >= self.glyph_cache_limit)
            && !self.glyph_cache.is_empty()
        {
            self.evict_glyph_cache_one();
        }
        self.glyph_cache_bytes += bytes;
    }

    pub fn pixel_rgba(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let i = ((y * self.width + x) as usize) * 4;
        Some([
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ])
    }

    /// Intersects a clip (or the whole buffer) with this renderer's bounds.
    ///
    /// The clip arrives in scene units — logical pixels — and the buffer is in
    /// device pixels, so it is scaled on the way in. The `max`/`min` against the
    /// buffer bounds has to come *after* that: a logical edge can sit far
    /// outside the device extent (`x = 400` on a 2× display is device 800), and
    /// clamping first would silently clip the app to half its width.
    fn clip_region(&self, clip: Option<Rect>) -> Clip {
        let (mut x0, mut y0, mut x1, mut y1) = (0u32, 0u32, self.width, self.height);
        if let Some(clip) = clip {
            x0 = x0.max((clip.x0 * self.scale).floor().max(0.0) as u32);
            y0 = y0.max((clip.y0 * self.scale).floor().max(0.0) as u32);
            x1 = x1.min((clip.x1 * self.scale).ceil().max(0.0) as u32);
            y1 = y1.min((clip.y1 * self.scale).ceil().max(0.0) as u32);
        }
        (x0.min(x1), y0.min(y1), x1, y1)
    }

    fn clear_region(&mut self, (x0, y0, x1, y1): Clip) {
        for y in y0..y1 {
            let row = ((y * self.width + x0) as usize) * 4;
            let span = ((x1 - x0) as usize) * 4;
            self.pixels[row..row + span].fill(0);
        }
    }

    /// src-over blend of `color` at fractional `coverage`, scaled by whatever
    /// the clip stack admits at this pixel.
    ///
    /// Every drawing routine goes through here rather than calling
    /// `blend_pixel` directly: a curved or path clip cannot be expressed as a
    /// scissor, so its coverage has to reach the pixel no matter which
    /// primitive is being drawn.
    fn blend(&mut self, x: u32, y: u32, color: Color, coverage: f32) {
        let clip = self.clips.coverage(x, y, self.scale);
        if clip <= 0.0 {
            return;
        }
        self.blend_pixel(x, y, color, coverage * clip);
    }

    /// src-over blend of `color` at fractional `coverage` onto one pixel.
    fn blend_pixel(&mut self, x: u32, y: u32, color: Color, coverage: f32) {
        let i = ((y * self.width + x) as usize) * 4;
        let sa = f32::from(color.a) / 255.0 * coverage;
        if sa >= 1.0 {
            self.pixels[i] = color.r;
            self.pixels[i + 1] = color.g;
            self.pixels[i + 2] = color.b;
            self.pixels[i + 3] = color.a;
            return;
        }
        let da = f32::from(self.pixels[i + 3]) / 255.0;
        let out_a = sa + da * (1.0 - sa);
        if out_a <= 0.0 {
            self.pixels[i..i + 4].fill(0);
            return;
        }
        for (c, &src8) in [color.r, color.g, color.b].iter().enumerate() {
            let src = f32::from(src8) / 255.0;
            let dst = f32::from(self.pixels[i + c]) / 255.0;
            let out = (src * sa + dst * da * (1.0 - sa)) / out_a;
            self.pixels[i + c] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        self.pixels[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color, clip: Clip) {
        if !self.clips.is_rect_only() {
            // A curved or path clip admits a fraction of some pixels, so the
            // opaque store below would paint straight over the clip's edge.
            let (cx0, cy0, cx1, cy1) = clip;
            let x0 = (x0.clamp(0, self.width as i32) as u32).max(cx0);
            let y0 = (y0.clamp(0, self.height as i32) as u32).max(cy0);
            let x1 = (x1.clamp(0, self.width as i32) as u32).min(cx1);
            let y1 = (y1.clamp(0, self.height as i32) as u32).min(cy1);
            for y in y0..y1 {
                for x in x0..x1 {
                    self.blend(x, y, color, 1.0);
                }
            }
            self.rects_drawn += 1;
            return;
        }
        let (cx0, cy0, cx1, cy1) = clip;
        let x0 = (x0.clamp(0, self.width as i32) as u32).max(cx0);
        let y0 = (y0.clamp(0, self.height as i32) as u32).max(cy0);
        let x1 = (x1.clamp(0, self.width as i32) as u32).min(cx1);
        let y1 = (y1.clamp(0, self.height as i32) as u32).min(cy1);
        for y in y0..y1 {
            for x in x0..x1 {
                let i = ((y * self.width + x) as usize) * 4;
                self.pixels[i] = color.r;
                self.pixels[i + 1] = color.g;
                self.pixels[i + 2] = color.b;
                self.pixels[i + 3] = color.a;
            }
        }
        self.rects_drawn += 1;
    }

    /// Fills a rounded rectangle with analytic anti-aliasing.
    ///
    /// Coverage per pixel is `clamp(0.5 - d, 0, 1)` where `d` is the
    /// rounded-rect signed distance at the pixel centre (Inigo Quilez's SDF;
    /// negative inside, in pixels). The 0.5px band across `d = 0` is the AA
    /// ramp — the same coverage a 16-sample box filter would converge to for a
    /// straight edge, at one distance evaluation per pixel instead of 16.
    fn fill_rounded_rect(&mut self, bounds: Rect, radius: f64, color: Color, clip: Clip) {
        let (cx0, cy0, cx1, cy1) = clip;
        let radius = radius.clamp(0.0, bounds.width().min(bounds.height()) / 2.0);
        let x0 = (bounds.x0.floor().max(0.0) as u32).max(cx0);
        let y0 = (bounds.y0.floor().max(0.0) as u32).max(cy0);
        let x1 = ((bounds.x1.ceil() as u32).min(self.width)).min(cx1);
        let y1 = ((bounds.y1.ceil() as u32).min(self.height)).min(cy1);
        let cx = (bounds.x0 + bounds.x1) / 2.0;
        let cy = (bounds.y0 + bounds.y1) / 2.0;
        let half_w = (bounds.x1 - bounds.x0) / 2.0;
        let half_h = (bounds.y1 - bounds.y0) / 2.0;
        // Corner box: rectangle of the "inner core" the corner arcs attach to.
        let qx_extent = half_w - radius;
        let qy_extent = half_h - radius;

        for y in y0..y1 {
            for x in x0..x1 {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let qxp = (px - cx).abs() - qx_extent;
                let qyp = (py - cy).abs() - qy_extent;
                let outside_x = qxp.max(0.0);
                let outside_y = qyp.max(0.0);
                let d = outside_x.hypot(outside_y) + qxp.max(qyp).min(0.0) - radius;
                let coverage = (0.5 - d).clamp(0.0, 1.0);
                if coverage <= 0.0 {
                    continue;
                }
                self.blend(x, y, color, coverage as f32);
            }
        }
        self.rects_drawn += 1;
    }

    /// Draws a text node: rasterises each glyph run's alpha mask with swash
    /// and blends the fill colour through it.
    fn draw_text(
        &mut self,
        origin: floem_types::Point,
        glyph_resource: floem_types::ResourceId,
        color: Color,
        resources: &ResourceRegistry,
        clip: Clip,
    ) {
        let Some(ResourceKind::GlyphRun { runs }) = resources.get(glyph_resource).map(|r| &r.kind)
        else {
            return;
        };
        // Logical → device happens here, once, for the whole run: a glyph run
        // is shaped in logical units (the laid-out box the widget measured
        // against) and the buffer is in device pixels. Scaling the pen
        // positions but rasterising at the logical size would space the
        // letters out and shrink the glyphs — the size has to scale too, which
        // is also why it belongs in the cache key.
        let scale = self.scale;
        let font_scale = scale as f32;
        for run in runs {
            if run.font.data.as_ref().is_empty() {
                continue;
            }
            let Some(font_ref) =
                swash::FontRef::from_index(run.font.data.as_ref(), run.font.index as usize)
            else {
                continue;
            };
            // Baseline where this run lands, in buffer pixels.
            let bx = (origin.x as f32 + run.origin.x as f32) * font_scale;
            let by = (origin.y as f32 + run.origin.y as f32) * font_scale;
            let font_size = run.font_size * font_scale;
            for glyph in &run.glyphs {
                // Cache lookup/insert must finish before the buffer is
                // borrowed mutably, so clone the (cheap Arc) mask out first.
                let cache_key: GlyphCacheKey = (
                    run.font.data.as_ref().as_ptr() as usize,
                    run.font.index,
                    font_size.to_bits(),
                    glyph.id,
                );
                let mask = if let Some((_, mask, stamp)) = self.glyph_cache.get_mut(&cache_key) {
                    self.glyph_cache_clock += 1;
                    *stamp = self.glyph_cache_clock;
                    self.glyph_cache_hits += 1;
                    std::sync::Arc::clone(mask)
                } else {
                    self.glyph_cache_misses += 1;
                    let Some(mask) = rasterize_glyph(font_ref, font_size, glyph.id) else {
                        // Colour glyphs (emoji) have no alpha-mask form;
                        // not cached, retried (and skipped) each frame.
                        continue;
                    };
                    let mask = std::sync::Arc::new(mask);
                    // The FontData clone pins the font bytes, so the
                    // key's pointer stays valid and un-aliasable while
                    // the entry lives.
                    self.admit_glyph_cache_entry(mask.data.len());
                    self.glyph_cache_clock += 1;
                    self.glyph_cache.insert(
                        cache_key,
                        (
                            run.font.data.clone(),
                            std::sync::Arc::clone(&mask),
                            self.glyph_cache_clock,
                        ),
                    );
                    mask
                };
                // swash placement: left is origin→image-left; top is
                // image-top→baseline (y-down), so the image's top-left lands
                // at (baseline_x + left, baseline_y - top). Same convention
                // tiny-skia's adapter uses (`new_y.floor() - cached.top`).
                let px = (bx + glyph.x as f32 * font_scale).floor() as i32
                    + i32::from(mask.placement.left);
                let py = (by + glyph.y as f32 * font_scale).floor() as i32
                    - i32::from(mask.placement.top);
                let (cx0, cy0, cx1, cy1) = clip;
                for row in 0..mask.placement.height as i32 {
                    let dy = py + row;
                    if dy < 0 || dy < cy0 as i32 || dy >= cy1 as i32 || dy >= self.height as i32 {
                        continue;
                    }
                    for col in 0..mask.placement.width as i32 {
                        let dx = px + col;
                        if dx < 0 || dx < cx0 as i32 || dx >= cx1 as i32 || dx >= self.width as i32
                        {
                            continue;
                        }
                        let alpha = mask.data[(row * mask.placement.width as i32 + col) as usize];
                        if alpha == 0 {
                            continue;
                        }
                        let coverage = f32::from(alpha) / 255.0;
                        self.blend(dx as u32, dy as u32, color, coverage);
                        self.glyphs_drawn += 1;
                    }
                }
            }
        }
    }

    /// Blits a registry image (RGBA8 **premultiplied**) into `bounds` with
    /// nearest-neighbour sampling, converting to the buffer's straight alpha.
    fn draw_image(
        &mut self,
        bounds: Rect,
        image_resource: floem_types::ResourceId,
        alpha: u8,
        resources: &ResourceRegistry,
        clip: Clip,
    ) {
        let Some(ResourceKind::Image {
            width,
            height,
            pixels,
        }) = resources.get(image_resource).map(|r| &r.kind)
        else {
            return;
        };
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 || *width == 0 || *height == 0 {
            return;
        }
        let (cx0, cy0, cx1, cy1) = clip;
        let x0 = (bounds.x0.floor().max(0.0) as u32).max(cx0);
        let y0 = (bounds.y0.floor().max(0.0) as u32).max(cy0);
        let x1 = ((bounds.x1.ceil() as u32).min(self.width)).min(cx1);
        let y1 = ((bounds.y1.ceil() as u32).min(self.height)).min(cy1);
        for y in y0..y1 {
            let sy = (((y as f64 + 0.5 - bounds.y0) / bounds.height()).clamp(0.0, 0.999_999_9)
                * f64::from(*height)) as u32;
            for x in x0..x1 {
                let sx = (((x as f64 + 0.5 - bounds.x0) / bounds.width()).clamp(0.0, 0.999_999_9)
                    * f64::from(*width)) as u32;
                let si = ((sy * *width + sx) as usize) * 4;
                let (pr, pg, pb, pa) = (
                    f32::from(pixels[si]),
                    f32::from(pixels[si + 1]),
                    f32::from(pixels[si + 2]),
                    f32::from(pixels[si + 3]),
                );
                if pa == 0.0 {
                    continue;
                }
                // Premultiplied → straight: rgb / alpha.
                let straight = Color::rgba(
                    ((pr / pa) * 255.0).round().clamp(0.0, 255.0) as u8,
                    ((pg / pa) * 255.0).round().clamp(0.0, 255.0) as u8,
                    ((pb / pa) * 255.0).round().clamp(0.0, 255.0) as u8,
                    pa as u8,
                );
                self.blend(x, y, straight, f32::from(alpha) / f32::from(u8::MAX));
            }
        }
        self.images_drawn += 1;
    }

    /// Flattens a polyline set into `self.contours`, in device pixels.
    ///
    /// The outer vector is reused, and each slot's `Vec<Point>` is refilled in
    /// place, so a scene that draws a chart every frame does not re-allocate
    /// the outline every frame. The flattening itself still allocates (one
    /// vector per subpath); that is the price of the path being a value rather
    /// than a view, and it is bounded by the path's own complexity.
    fn build_contours(&mut self, polylines: Vec<Polyline>, scale: f64) {
        let count = polylines.len();
        if self.contours.len() < count {
            self.contours.resize_with(count, Vec::new);
        }
        for (slot, poly) in self.contours.iter_mut().zip(polylines) {
            slot.clear();
            slot.extend(
                poly.points
                    .into_iter()
                    .map(|p| Point::new(p.x * scale, p.y * scale)),
            );
        }
        self.contours.truncate(count);
    }

    /// Rasterises the contours currently in `self.contours` and blends `color`
    /// through the coverage.
    fn rasterize_contours(&mut self, color: Color, clip: Scissor) {
        let Some(shape) = raster::contour_bounds(&self.contours) else {
            return;
        };
        // The shape's own extent, not the frame's: a coverage buffer the size
        // of the window would make every path node cost a snapshot of the
        // window.
        let region = raster::intersect_scissor(shape, clip);
        if raster::scissor_is_empty(region) {
            return;
        }
        let stride = raster::rasterize(&self.contours, region, &mut self.coverage);
        if stride == 0 {
            return;
        }
        let (x0, y0, x1, y1) = region;
        for y in y0..y1 {
            let base = (y - y0) as usize * stride;
            for x in x0..x1 {
                let coverage = self.coverage[base + (x - x0) as usize];
                if coverage > 0.0 {
                    self.blend(x, y, color, coverage);
                }
            }
        }
    }

    /// Redraws the nodes that touch `region` (`None` = whole buffer), clipped
    /// to it.
    ///
    /// Partial redraw still repaints whatever overlapped the damage — the
    /// buffer there was cleared — but a node whose bounds miss every damaged
    /// rect cannot have contributed a single pixel to it, so it is culled
    /// before any per-pixel work. That is what keeps a large scene cheap when
    /// only a small area changes: the walk is still O(nodes), the rasterising
    /// is not.
    fn draw_all(
        &mut self,
        scene: &Scene,
        resources: Option<&ResourceRegistry>,
        region: Option<Rect>,
    ) {
        let damage_clip = self.clip_region(region);
        let scale = self.scale;
        // The marker stream may end with regions still pushed — nothing is
        // drawn after the last node, so a trailing push has no effect on this
        // walk. Inheriting it into the next one would, and the next walk starts
        // at the first node again.
        self.clips.clear();
        for (_id, node) in scene.iter() {
            // Clip markers change the stack rather than the pixels, so they are
            // handled *before* culling: a marker whose own box misses the
            // damaged rect still has to move the stack for the nodes that
            // follow it, or those nodes would be clipped by the wrong region.
            match node {
                SceneNode::ClipPush { region } => {
                    self.clips.push(region.clone(), scale);
                    self.clips_resolved += 1;
                    continue;
                }
                SceneNode::ClipPop => {
                    self.clips.pop();
                    continue;
                }
                _ => {}
            }
            let clip = match self.clips.scissor(scale) {
                Some(scissor) => raster::intersect_scissor(damage_clip, scissor),
                None => damage_clip,
            };
            if raster::scissor_is_empty(clip) {
                // Everything from here on is clipped away. Not counted as a
                // node skip: `nodes_skipped` means "culled by damage", and
                // conflating the two would make the counter lie.
                continue;
            }
            // Groups and clip markers report no bounds (they force a full
            // frame), so they are never culled — only nodes with a known box
            // are.
            if let Some(bounds) = node.bounds_hint() {
                if !hits_clip(scale_rect(bounds, scale), clip) {
                    self.nodes_skipped += 1;
                    continue;
                }
            }
            // Every arm below crosses from the scene's logical units into the
            // device-pixel buffer, so each one scales its own geometry.
            match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius: 0.0,
                } => {
                    let b = scale_rect(*bounds, scale);
                    self.fill_rect(
                        b.x0.floor() as i32,
                        b.y0.floor() as i32,
                        b.x1.ceil() as i32,
                        b.y1.ceil() as i32,
                        *fill,
                        clip,
                    );
                }
                // A non-zero radius takes the AA path, which works in the
                // node's own (sub-pixel) geometry instead of the integer grid.
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                } => self.fill_rounded_rect(
                    scale_rect(*bounds, scale),
                    f64::from(*radius) * scale,
                    *fill,
                    clip,
                ),
                SceneNode::Text {
                    origin,
                    glyphs,
                    color,
                    ..
                } => {
                    if let Some(resources) = resources {
                        // Unscaled: `draw_text` scales the block origin, the
                        // run offset, every pen position and the size the
                        // glyphs rasterise at, which only stays consistent if
                        // it is handed the scene's own units.
                        self.draw_text(*origin, *glyphs, *color, resources, clip);
                    }
                }
                SceneNode::Image {
                    bounds,
                    image,
                    alpha,
                } => {
                    if let Some(resources) = resources {
                        self.draw_image(
                            scale_rect(*bounds, scale),
                            *image,
                            *alpha,
                            resources,
                            clip,
                        );
                    }
                }
                SceneNode::Path { path, fill, stroke } => {
                    if let Some(color) = *fill {
                        let polylines = path.flatten(floem_types::DEFAULT_FLATTEN_TOLERANCE);
                        self.build_contours(polylines, scale);
                        self.rasterize_contours(color, clip);
                    }
                    if let Some(stroke) = stroke {
                        // A stroke is a fill of its own outline, so both halves
                        // of the path vocabulary go through one scanline
                        // filler — the caps and joins are exercised by the same
                        // code the fill is.
                        let outline = stroke
                            .outline_with_tolerance(path, floem_types::DEFAULT_FLATTEN_TOLERANCE);
                        self.build_contours(outline, scale);
                        self.rasterize_contours(stroke.color, clip);
                    }
                    self.paths_drawn += 1;
                }
                SceneNode::Group { .. } => {}
                // Unreachable: the markers were consumed above, before any
                // node could be culled. Listed anyway so adding a variant to
                // the scene IR without teaching this backend about it is a
                // compile error rather than a silently missing feature.
                SceneNode::ClipPush { .. } | SceneNode::ClipPop => {}
            }
        }
    }

    /// The render path shared by the trait entry points: decide the damaged
    /// regions, clear them, and repaint every node into each.
    fn render_with_resources(
        &mut self,
        scene: &Scene,
        damage: &DamageRegion,
        resources: Option<&ResourceRegistry>,
    ) {
        if self.needs_full_clear {
            // After creation/resize the buffer holds nothing trustworthy.
            self.pixels.fill(0);
            self.needs_full_clear = false;
            self.draw_all(scene, resources, None);
            return;
        }
        if damage.is_empty() {
            // Nothing changed since the last present: leave the buffer alone.
            self.damage_skips += 1;
            return;
        }
        if damage.is_full_frame() {
            self.pixels.fill(0);
            self.draw_all(scene, resources, None);
            return;
        }
        for region in damage.rects() {
            let clip = self.clip_region(Some(*region));
            self.clear_region(clip);
            // Repaint *every* node into the damaged rect: whatever overlapped
            // it before (and was destroyed by the clear) is drawn again, and
            // pixels outside stay untouched.
            self.draw_all(scene, resources, Some(*region));
        }
    }
}

/// True when `bounds` (logical space) overlaps `clip` (pixel space) at all.
///
/// Node bounds are inclusive-exclusive rects that may start negative or run
/// past the buffer, so both sides are compared in floating point before any
/// clamping: `ceil` on the upper edge keeps a node that merely grazes the
/// region, which matters for the anti-aliased boundary pixel.
/// Logical scene rect → device-pixel rect. The identity when `scale` is 1,
/// which is why a 1× display has never needed it.
fn scale_rect(bounds: Rect, scale: f64) -> Rect {
    if scale == 1.0 {
        return bounds;
    }
    Rect {
        x0: bounds.x0 * scale,
        y0: bounds.y0 * scale,
        x1: bounds.x1 * scale,
        y1: bounds.y1 * scale,
    }
}

fn hits_clip(bounds: Rect, clip: Clip) -> bool {
    let (cx0, cy0, cx1, cy1) = clip;
    bounds.x1.ceil() > f64::from(cx0)
        && bounds.x0.floor() < f64::from(cx1)
        && bounds.y1.ceil() > f64::from(cy0)
        && bounds.y0.floor() < f64::from(cy1)
}

struct GlyphMask {
    placement: swash::zeno::Placement,
    data: Vec<u8>,
}

/// Rasterises one glyph as an 8-bit alpha mask at the given font size.
fn rasterize_glyph(font: swash::FontRef<'_>, font_size: f32, glyph_id: u32) -> Option<GlyphMask> {
    use swash::scale::image::Content;
    use swash::scale::{Render, ScaleContext, Source, StrikeWith};
    use swash::zeno::Format;

    thread_local! {
        static SCALE_CONTEXT: std::cell::RefCell<ScaleContext> =
            std::cell::RefCell::new(ScaleContext::new());
    }

    SCALE_CONTEXT.with_borrow_mut(|context| {
        let mut scaler = context.builder(font).size(font_size).build();
        let mut render = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ]);
        render.format(Format::Alpha);
        let image = render.render(&mut scaler, glyph_id as u16)?;
        if image.content != Content::Mask {
            // Colour glyphs (emoji) have no representation in an alpha mask.
            return None;
        }
        Some(GlyphMask {
            placement: image.placement,
            data: image.data,
        })
    })
}

impl Renderer for SoftwareRenderer {
    fn begin_frame(&mut self, info: &FrameInfo) {
        if let RenderTarget::Software { width, height } = info.target {
            if width != self.width || height != self.height {
                self.width = width;
                self.height = height;
                self.pixels
                    .resize((width as usize) * (height as usize) * 4, 0);
                // A resized buffer cannot be patched incrementally.
                self.needs_full_clear = true;
            }
        }
        // A density change moves every logical coordinate on screen without
        // touching a single node, so the scene is stale even though the damage
        // region — computed in logical units — is empty.
        let scale = if info.scale_factor > 0.0 {
            info.scale_factor
        } else {
            1.0
        };
        if scale != self.scale {
            self.scale = scale;
            self.needs_full_clear = true;
            // Cached masks were rasterised at the old density; the size is part
            // of the cache key, so they would never be hit again — release them
            // rather than hold the memory.
            self.glyph_cache.clear();
            self.glyph_cache_bytes = 0;
        }
        // No bulk clear here: damage decides what gets wiped in render_scene.
    }

    fn render_scene(&mut self, scene: &Scene, damage: &DamageRegion) {
        self.render_with_resources(scene, damage, None);
    }

    fn render_scene_with_resources(
        &mut self,
        scene: &Scene,
        damage: &DamageRegion,
        resources: &ResourceRegistry,
    ) {
        self.render_with_resources(scene, damage, Some(resources));
    }

    fn end_frame(&mut self) {
        self.frames += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_renderer::{FrameInfo, render_once};
    use floem_scene::{Point, SceneNode, Size};
    use floem_types::Color;

    #[test]
    fn software_draws_rect_pixels() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(3.0, 3.0)),
            fill: Color::rgb(10, 20, 30),
            radius: 0.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(8, 8);
        render_once(&mut sw, &FrameInfo::software(8, 8, 1), &scene);
        assert_eq!(sw.frames, 1);
        assert_eq!(sw.rects_drawn, 1);
        assert_eq!(sw.pixel_rgba(3, 3), Some([10, 20, 30, 255]));
        assert_eq!(sw.pixel_rgba(0, 0), Some([0, 0, 0, 0]));
    }

    /// A 4×4 rect at (2,2) with radius 2: the interior is fully opaque, the
    /// top-left corner pixel is partially covered by the arc, and pixels the
    /// SDF puts fully outside are untouched.
    #[test]
    fn rounded_rect_interior_is_solid_and_corner_is_antialiased() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(4.0, 4.0)),
            fill: Color::rgb(200, 10, 10),
            radius: 2.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(8, 8);
        render_once(&mut sw, &FrameInfo::software(8, 8, 1), &scene);

        // Interior pixel: full coverage, straight overwrite.
        assert_eq!(sw.pixel_rgba(4, 4), Some([200, 10, 10, 255]));
        // (3,3) sits well inside the arc: centre (3.5,3.5) is √(0.5²+0.5²)
        // from the arc centre — full coverage.
        assert_eq!(sw.pixel_rgba(3, 3), Some([200, 10, 10, 255]));

        // Corner pixel (2,2): centre at (2.5, 2.5), arc centre (4,4), r=2 →
        // d = √(1.5²+1.5²) − 2 ≈ 0.121 → partial coverage.
        let corner = sw.pixel_rgba(2, 2).expect("in bounds");
        assert!(
            corner[3] > 0 && corner[3] < 255,
            "corner pixel should be an AA blend, got {corner:?}"
        );
        assert!(corner[0] > 0, "corner pixel picked up fill colour");

        // Fully outside the arc: unchanged.
        assert_eq!(sw.pixel_rgba(1, 1), Some([0, 0, 0, 0]));
        assert_eq!(sw.pixel_rgba(0, 0), Some([0, 0, 0, 0]));
    }

    /// The SDF path clamps the radius to half the shorter side, so a
    /// larger-than-legal radius renders a pill instead of panicking or
    /// producing garbage.
    #[test]
    fn oversized_radius_clamps_to_a_pill() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 3.0), Size::new(6.0, 2.0)),
            fill: Color::rgb(0, 255, 0),
            radius: 99.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(10, 10);
        render_once(&mut sw, &FrameInfo::software(10, 10, 1), &scene);

        // Pill middle row: fully covered.
        assert_eq!(sw.pixel_rgba(4, 3), Some([0, 255, 0, 255]));
        assert_eq!(sw.pixel_rgba(4, 4), Some([0, 255, 0, 255]));
        // Beyond either cap: empty.
        assert_eq!(sw.pixel_rgba(8, 3), Some([0, 0, 0, 0]));
        assert_eq!(sw.pixel_rgba(0, 3), Some([0, 0, 0, 0]));
    }

    /// A translucent rounded fill blends with what is already in the buffer
    /// instead of punching through it.
    #[test]
    fn translucent_rounded_rect_blends_src_over() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: Color::rgb(255, 0, 0),
            radius: 0.0,
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: Color::rgba(0, 0, 255, 128),
            radius: 1.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(4, 4);
        render_once(&mut sw, &FrameInfo::software(4, 4, 1), &scene);

        // Interior: 50% blue over opaque red → ~127 blue channel, red at ~128.
        let mixed = sw.pixel_rgba(1, 1).expect("in bounds");
        assert!(
            (100..=155).contains(&mixed[0]) && (100..=155).contains(&mixed[2]),
            "expected a red/blue blend, got {mixed:?}"
        );
        assert!(mixed[1] < 30, "green must stay out of the blend");
    }

    /// The image path converts the registry's premultiplied RGBA8 into the
    /// buffer's straight alpha and blends over the existing buffer.
    #[test]
    fn image_nodes_blit_premultiplied_pixels_as_straight_alpha() {
        let mut registry = ResourceRegistry::new();
        // 1×1 half-alpha red, premultiplied: (128, 0, 0, 128) → straight
        // (255, 0, 0, 128).
        let img = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 1,
                height: 1,
                pixels: std::sync::Arc::from(vec![128, 0, 0, 128].into_boxed_slice()),
            },
        });
        // One scene with a blue backdrop and the image on top, so the damaged
        // second frame repaints the backdrop too (damage redraws the whole
        // scene, clipped to the damage).
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: Color::rgb(0, 0, 255),
            radius: 0.0,
        });
        let _ = scene.insert(SceneNode::image(
            Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(2.0, 2.0)),
            img,
        ));
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(4, 4);
        render_once(&mut sw, &FrameInfo::software(4, 4, 1), &scene);
        // render_once has no registry: only the rect is drawn so far.
        assert_eq!(sw.pixel_rgba(1, 1), Some([0, 0, 255, 255]));

        // Re-present with resources, damaged only around the image.
        let mut damage = DamageRegion::new();
        damage.union_rect(Rect::from_origin_size(
            Point::new(1.0, 1.0),
            Size::new(2.0, 2.0),
        ));
        sw.render_scene_with_resources(&scene, &damage, &registry);
        sw.end_frame();

        // Half-alpha red over opaque blue → red and blue both mid-range.
        let px = sw.pixel_rgba(1, 1).expect("in bounds");
        assert!(
            (110..=145).contains(&px[0]) && (110..=145).contains(&px[2]) && px[1] < 30,
            "expected a red/blue blend from premultiplied source, got {px:?}"
        );
        assert_eq!(sw.images_drawn, 1);
    }

    /// A node alpha dims the image it is drawn with.
    ///
    /// The scene's `Image` variant carried no alpha for a long time, so an
    /// image inside a fading overlay stayed opaque over a dimming background —
    /// the composite had nowhere to write. Halving the alpha has to halve the
    /// image's contribution, not do nothing.
    #[test]
    fn a_node_alpha_dims_the_image_it_draws() {
        let mut registry = ResourceRegistry::new();
        // Opaque white, so the only thing between it and the backdrop is the
        // node alpha.
        let img = registry.insert(floem_scene::Resource {
            kind: ResourceKind::Image {
                width: 1,
                height: 1,
                pixels: std::sync::Arc::from(vec![255, 255, 255, 255].into_boxed_slice()),
            },
        });

        let mut solids = Vec::new();
        for alpha in [u8::MAX, u8::MAX / 2] {
            let mut scene = Scene::new();
            scene.begin_update();
            let _ = scene.insert(SceneNode::Rect {
                bounds: Rect::from_origin_size(Point::ZERO, Size::new(2.0, 2.0)),
                fill: Color::rgb(0, 0, 0),
                radius: 0.0,
            });
            let _ = scene.insert(SceneNode::Image {
                bounds: Rect::from_origin_size(Point::ZERO, Size::new(2.0, 2.0)),
                image: img,
                alpha,
            });
            scene.finish_update();

            let mut sw = SoftwareRenderer::new(2, 2);
            let mut damage = DamageRegion::new();
            damage.union_rect(Rect::from_origin_size(Point::ZERO, Size::new(2.0, 2.0)));
            sw.render_scene_with_resources(&scene, &damage, &registry);
            sw.end_frame();
            solids.push(sw.pixel_rgba(0, 0).expect("in bounds")[0]);
        }

        let (opaque, half) = (solids[0], solids[1]);
        assert!(
            opaque > 200,
            "opaque white over black stays white: {opaque}"
        );
        assert!(
            (100..=155).contains(&half),
            "half alpha must land near half brightness, got {half}"
        );
    }

    /// Text nodes rasterise through swash when the registry is available:
    /// some pixels inside the text bounds must pick up the fill colour.
    #[test]
    fn text_nodes_rasterize_glyphs_from_their_resource() {
        use floem_text::{TextAlign, TextLayoutOptions};

        let mut shaper = floem_text::TextShaper::new();
        let laid = shaper.shape(
            "HI",
            &TextLayoutOptions {
                max_width: f64::INFINITY,
                font_size: 24.0,
                align: TextAlign::Start,
            },
        );
        assert!(!laid.runs.is_empty(), "shaper produced runs");

        let mut registry = ResourceRegistry::new();
        let glyphs = registry.insert(floem_scene::Resource {
            kind: ResourceKind::GlyphRun {
                runs: laid.runs.clone(),
            },
        });

        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Text {
            origin: Point::new(2.0, 2.0),
            size: laid.size,
            glyphs,
            color: Color::rgb(255, 255, 255),
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(96, 40);
        render_once(&mut sw, &FrameInfo::software(96, 40, 1), &scene);
        // render_once passes no resources: nothing drawn, nothing crashes.
        assert_eq!(sw.glyphs_drawn, 0);

        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        assert!(sw.glyphs_drawn > 0, "glyph pixels were rasterised");

        // At least one pixel inside the text area carries white ink.
        let width = (laid.size.width.ceil() as u32).min(90);
        let height = (laid.size.height.ceil() as u32).min(36);
        let mut inked = 0;
        for y in 0..height {
            for x in 0..width {
                if let Some(px) = sw.pixel_rgba(2 + x, 2 + y) {
                    if px[0] > 100 {
                        inked += 1;
                    }
                }
            }
        }
        assert!(inked > 0, "text left no ink inside its bounds");
    }

    /// Glyphs must be spread *along* the run, not stacked at its origin.
    ///
    /// This is the failure mode of reading Parley's per-glyph offset as a
    /// position: every glyph lands on the run origin, so the ink hugs the start
    /// of a box the layout sized correctly for the whole string — a smudge
    /// sitting inside exactly the right amount of space. "Some ink was drawn"
    /// passes either way (see the test above, which is why it did not catch
    /// this); ink in the far end of the run is what tells them apart.
    #[test]
    fn glyphs_spread_along_the_run_instead_of_stacking() {
        use floem_text::{TextAlign, TextLayoutOptions};

        let text = "clicked 2 times";
        let mut shaper = floem_text::TextShaper::new();
        let laid = shaper.shape(
            text,
            &TextLayoutOptions {
                max_width: f64::INFINITY,
                font_size: 20.0,
                align: TextAlign::Start,
            },
        );
        let width = laid.size.width.ceil() as u32;
        assert!(width > 100, "the string is wide enough to tell: {width}");

        let mut registry = ResourceRegistry::new();
        let glyphs = registry.insert(floem_scene::Resource {
            kind: ResourceKind::GlyphRun {
                runs: laid.runs.clone(),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Text {
            origin: Point::new(2.0, 2.0),
            size: laid.size,
            glyphs,
            color: Color::rgb(255, 255, 255),
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(width + 8, 40);
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();

        // Rightmost inked column, relative to the text origin.
        let mut rightmost = 0i64;
        for y in 0..laid.size.height.ceil() as u32 {
            for x in 0..width {
                if let Some(px) = sw.pixel_rgba(2 + x, 2 + y) {
                    if px[0] > 100 {
                        rightmost = rightmost.max(i64::from(x));
                    }
                }
            }
        }
        assert!(
            rightmost as f64 > f64::from(width) * 0.8,
            "the last glyph should sit near the end of the {width}px run, \
             but the ink stops at column {rightmost} — the run is stacked at \
             its origin"
        );
    }

    /// Damage-driven partial redraw: pixels outside the damaged region
    /// survive untouched, pixels inside are cleared and repainted from the
    /// current scene.
    #[test]
    fn damage_limits_which_pixels_are_touched() {
        let scene_with = |x: f64| {
            let mut scene = Scene::new();
            scene.begin_update();
            let _ = scene.insert(SceneNode::Rect {
                bounds: Rect::from_origin_size(Point::new(x, 1.0), Size::new(3.0, 3.0)),
                fill: Color::rgb(200, 0, 0),
                radius: 0.0,
            });
            let _ = scene.insert(SceneNode::Rect {
                bounds: Rect::from_origin_size(Point::new(6.0, 1.0), Size::new(3.0, 3.0)),
                fill: Color::rgb(0, 0, 200),
                radius: 0.0,
            });
            scene.finish_update();
            scene
        };

        let mut sw = SoftwareRenderer::new(12, 6);
        let first = scene_with(1.0);
        render_once(&mut sw, &FrameInfo::software(12, 6, 1), &first);
        assert_eq!(sw.pixel_rgba(2, 2), Some([200, 0, 0, 255]));
        assert_eq!(sw.pixel_rgba(7, 2), Some([0, 0, 200, 255]));

        // Next frame: damage covers only the red rect's side of the buffer.
        let second = scene_with(1.0);
        let mut damage = DamageRegion::new();
        damage.union_rect(Rect::from_origin_size(
            Point::new(0.0, 0.0),
            Size::new(6.0, 5.0),
        ));
        sw.render_scene(&second, &damage);
        sw.end_frame();

        // Inside damage: cleared and repainted from the scene.
        assert_eq!(sw.pixel_rgba(2, 2), Some([200, 0, 0, 255]));
        // The gap inside the damaged region (no node there) is cleared.
        assert_eq!(sw.pixel_rgba(5, 2), Some([0, 0, 0, 0]));
        // Outside damage: last frame's content survives untouched.
        assert_eq!(sw.pixel_rgba(7, 2), Some([0, 0, 200, 255]));
    }

    /// An empty damage region skips the frame entirely: the buffer stays
    /// exactly as the previous frame left it.
    #[test]
    fn empty_damage_skips_the_frame() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
            fill: Color::rgb(255, 0, 0),
            radius: 0.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(8, 8);
        render_once(&mut sw, &FrameInfo::software(8, 8, 1), &scene);
        assert_eq!(sw.pixel_rgba(1, 1), Some([255, 0, 0, 255]));

        sw.render_scene(&scene, &DamageRegion::new());
        assert_eq!(sw.damage_skips, 1, "empty damage skipped the repaint");
        assert_eq!(sw.pixel_rgba(1, 1), Some([255, 0, 0, 255]));
    }

    /// Same glyph at the same size across frames rasterises once: the second
    /// paint serves every mask from the cache, and the drawn ink is
    /// identical. A different font size is a different cache entry.
    #[test]
    fn glyph_masks_are_cached_across_frames() {
        use floem_text::{TextAlign, TextLayoutOptions};

        let shape = |size: f32| {
            let mut shaper = floem_text::TextShaper::new();
            shaper.shape(
                "HI",
                &TextLayoutOptions {
                    max_width: f64::INFINITY,
                    font_size: size,
                    align: TextAlign::Start,
                },
            )
        };

        let make_scene = |runs| {
            let mut registry = ResourceRegistry::new();
            let glyphs = registry.insert(floem_scene::Resource {
                kind: ResourceKind::GlyphRun { runs },
            });
            let mut scene = Scene::new();
            scene.begin_update();
            let _ = scene.insert(SceneNode::Text {
                origin: Point::new(2.0, 2.0),
                size: Size::new(90.0, 30.0),
                glyphs,
                color: Color::rgb(255, 255, 255),
            });
            scene.finish_update();
            (scene, registry)
        };

        let mut sw = SoftwareRenderer::new(96, 40);

        let (scene, registry) = make_scene(shape(24.0).runs);
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        let ink_first = sw.pixel_rgba(10, 12);
        let misses_first = sw.glyph_cache_misses;
        assert!(misses_first > 0, "first paint rasterises (and caches)");
        assert_eq!(sw.glyph_cache_hits, 0);
        assert_eq!(sw.glyph_cache_entries(), misses_first as usize);

        // Second frame, same glyphs: every mask comes from the cache and the
        // ink lands in exactly the same place.
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        assert_eq!(sw.glyph_cache_hits, misses_first, "all masks cached");
        assert_eq!(sw.glyph_cache_misses, misses_first, "no new rasterising");
        assert_eq!(sw.pixel_rgba(10, 12), ink_first, "identical output");

        // Same glyph at a different size: new cache entries, not the old ones.
        let (scene, registry) = make_scene(shape(32.0).runs);
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        assert!(
            sw.glyph_cache_misses > misses_first,
            "a new size rasterises fresh masks"
        );
    }

    /// Partial redraw culls whole nodes: a node whose bounds miss every
    /// damaged rect is not rasterised at all, while its pixels (outside the
    /// damage) survive exactly as the previous frame left them.
    #[test]
    fn damage_culls_nodes_that_miss_it() {
        // Two far-apart rects: a red one on the left, a blue one on the right.
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(3.0, 3.0)),
            fill: Color::rgb(200, 0, 0),
            radius: 0.0,
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(8.0, 1.0), Size::new(3.0, 3.0)),
            fill: Color::rgb(0, 0, 200),
            radius: 0.0,
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(12, 6);
        render_once(&mut sw, &FrameInfo::software(12, 6, 1), &scene);
        assert_eq!(sw.pixel_rgba(2, 2), Some([200, 0, 0, 255]));
        assert_eq!(sw.pixel_rgba(9, 2), Some([0, 0, 200, 255]));
        assert_eq!(sw.nodes_skipped, 0, "a full frame culls nothing");

        // Damage only the left side: the right-hand node misses it entirely.
        let mut damage = DamageRegion::new();
        damage.union_rect(Rect::from_origin_size(
            Point::new(0.0, 0.0),
            Size::new(5.0, 5.0),
        ));
        let skipped_before = sw.nodes_skipped;
        sw.render_scene(&scene, &damage);
        sw.end_frame();

        assert_eq!(
            sw.nodes_skipped - skipped_before,
            1,
            "the off-damage node is culled"
        );
        // Inside the damage: still repainted. Outside: untouched.
        assert_eq!(sw.pixel_rgba(2, 2), Some([200, 0, 0, 255]));
        assert_eq!(sw.pixel_rgba(9, 2), Some([0, 0, 200, 255]));
    }

    /// Renders one frame of `runs` at the given box, the way the app layer
    /// would: a text scene node pointing at a glyph-run resource.
    fn draw_text_frame(sw: &mut SoftwareRenderer, runs: &[floem_text::GlyphRun]) {
        let mut registry = ResourceRegistry::new();
        let glyphs = registry.insert(floem_scene::Resource {
            kind: ResourceKind::GlyphRun {
                runs: runs.to_vec(),
            },
        });
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Text {
            origin: Point::new(2.0, 2.0),
            size: Size::new(240.0, 120.0),
            glyphs,
            color: Color::rgb(255, 255, 255),
        });
        scene.finish_update();
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
    }

    /// The byte budget — not the entry count — is what bounds retained memory:
    /// a 24px glyph costs tens of bytes, a 120px one tens of kilobytes, so a
    /// fixed number of entries says nothing about how much is held.
    #[test]
    fn glyph_cache_byte_budget_bounds_retained_memory() {
        use floem_text::{TextAlign, TextLayoutOptions};

        let sizes = [24.0, 48.0, 72.0, 96.0, 120.0];
        // Shape once per size and reuse: each `shape()` allocates fresh font
        // bytes, which are part of the cache key.
        let shaped: std::collections::HashMap<u32, Vec<floem_text::GlyphRun>> = sizes
            .iter()
            .map(|&size| {
                let mut shaper = floem_text::TextShaper::new();
                let laid = shaper.shape(
                    "MM",
                    &TextLayoutOptions {
                        max_width: f64::INFINITY,
                        font_size: size,
                        align: TextAlign::Start,
                    },
                );
                (size.to_bits(), laid.runs.clone())
            })
            .collect();

        let mut sw = SoftwareRenderer::new(256, 128);
        sw.set_glyph_cache_byte_limit(64 * 1024);
        assert_eq!(sw.glyph_cache_byte_limit(), 64 * 1024);

        // One large glyph shows the granularity the budget can be overshot by:
        // a single entry is never split.
        draw_text_frame(&mut sw, &shaped[&96.0f32.to_bits()]);
        let biggest = sw.glyph_cache_bytes();
        assert!(biggest > 0, "a rasterised mask costs bytes");

        // Ten distinct masks through a budget that holds a few large ones:
        // bytes stay bounded instead of growing with every new size.
        for size in sizes {
            draw_text_frame(&mut sw, &shaped[&size.to_bits()]);
            assert!(
                sw.glyph_cache_bytes() <= 64 * 1024 + biggest,
                "bytes stay bounded by the budget, got {}",
                sw.glyph_cache_bytes()
            );
        }
        assert!(
            sw.glyph_cache_entries() < sizes.len() * 2,
            "large glyphs are evicted rather than all retained, got {} entries",
            sw.glyph_cache_entries()
        );
        assert!(
            sw.glyph_cache_hits > 0,
            "what does fit is still served from the cache"
        );

        // Clearing releases the budget, not just the entries.
        sw.clear_glyph_cache();
        assert_eq!(sw.glyph_cache_entries(), 0);
        assert_eq!(sw.glyph_cache_bytes(), 0, "clearing releases the budget");
    }

    /// Beyond the cache limit the least-recently-used masks are evicted: the
    /// cache never grows past the ceiling, a long-untouched size must
    /// re-rasterise when it comes back, and the most recently used size is
    /// still served from the cache.
    #[test]
    fn glyph_cache_evicts_least_recently_used() {
        use floem_text::{TextAlign, TextLayoutOptions};

        // Shape every size exactly once and reuse those runs across frames:
        // each shape() call allocates fresh font bytes, which are part of
        // the cache key — only same-byte glyphs can ever hit the cache.
        let sizes = [10.0, 12.0, 14.0, 16.0, 18.0];
        let shaped: std::collections::HashMap<u32, Vec<floem_text::GlyphRun>> = sizes
            .iter()
            .map(|&size| {
                let mut shaper = floem_text::TextShaper::new();
                let laid = shaper.shape(
                    "HI",
                    &TextLayoutOptions {
                        max_width: f64::INFINITY,
                        font_size: size,
                        align: TextAlign::Start,
                    },
                );
                (size.to_bits(), laid.runs.clone())
            })
            .collect();
        let make_scene = |runs: &[floem_text::GlyphRun]| {
            let mut registry = ResourceRegistry::new();
            let glyphs = registry.insert(floem_scene::Resource {
                kind: ResourceKind::GlyphRun {
                    runs: runs.to_vec(),
                },
            });
            let mut scene = Scene::new();
            scene.begin_update();
            let _ = scene.insert(SceneNode::Text {
                origin: Point::new(2.0, 2.0),
                size: Size::new(90.0, 30.0),
                glyphs,
                color: Color::rgb(255, 255, 255),
            });
            scene.finish_update();
            (scene, registry)
        };

        let mut sw = SoftwareRenderer::new(96, 40);
        sw.set_glyph_cache_limit(4);
        assert_eq!(sw.glyph_cache_limit(), 4);

        // Five sizes x two glyphs = ten distinct masks through a 4-slot cache.
        for size in sizes {
            let (scene, registry) = make_scene(&shaped[&size.to_bits()]);
            sw.render_scene_with_resources(&scene, scene.damage(), &registry);
            sw.end_frame();
            assert!(
                sw.glyph_cache_entries() <= 4,
                "cache stays within the limit, got {}",
                sw.glyph_cache_entries()
            );
        }

        // The oldest size was evicted long ago: coming back re-rasterises.
        let misses_before = sw.glyph_cache_misses;
        let (scene, registry) = make_scene(&shaped[&10.0f32.to_bits()]);
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        assert!(
            sw.glyph_cache_misses > misses_before,
            "evicted size re-rasterises on its return"
        );

        // The most recent size is still warm: no new rasterisation.
        let misses_before = sw.glyph_cache_misses;
        let hits_before = sw.glyph_cache_hits;
        let (scene, registry) = make_scene(&shaped[&18.0f32.to_bits()]);
        sw.render_scene_with_resources(&scene, scene.damage(), &registry);
        sw.end_frame();
        assert_eq!(
            sw.glyph_cache_misses, misses_before,
            "recent size stays cached"
        );
        assert!(sw.glyph_cache_hits > hits_before, "served from the cache");
    }
}
