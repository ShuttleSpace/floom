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

pub use presenter::SoftwarePresenter;

use floem_renderer::{FrameInfo, RenderTarget, Renderer};
use floem_scene::{DamageRegion, Rect, ResourceKind, ResourceRegistry, Scene, SceneNode};
use floem_types::Color;

/// Pixel-space clip region: `(x0, y0, x1, y1)`, exclusive right/bottom.
type Clip = (u32, u32, u32, u32);

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
    /// Set after (re)creation/resize: the next render must clear everything,
    /// since the buffer content cannot be trusted.
    needs_full_clear: bool,
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
            needs_full_clear: true,
        }
    }

    pub fn pixels(&self) -> &[u8] {
        &self.pixels
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
    fn clip_region(&self, clip: Option<Rect>) -> Clip {
        let (mut x0, mut y0, mut x1, mut y1) = (0u32, 0u32, self.width, self.height);
        if let Some(clip) = clip {
            x0 = x0.max(clip.x0.floor() as u32);
            y0 = y0.max(clip.y0.floor() as u32);
            x1 = x1.min(clip.x1.ceil() as u32);
            y1 = y1.min(clip.y1.ceil() as u32);
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
                self.blend_pixel(x, y, color, coverage as f32);
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
            let bx = origin.x as f32 + run.origin.x as f32;
            let by = origin.y as f32 + run.origin.y as f32;
            for glyph in &run.glyphs {
                let Some(mask) = rasterize_glyph(font_ref, run.font_size, glyph.id) else {
                    continue;
                };
                // swash placement: left is origin→image-left; top is
                // image-top→baseline (y-down), so the image's top-left lands
                // at (baseline_x + left, baseline_y - top). Same convention
                // tiny-skia's adapter uses (`new_y.floor() - cached.top`).
                let px = (bx + glyph.x).floor() as i32 + i32::from(mask.placement.left);
                let py = (by + glyph.y).floor() as i32 - i32::from(mask.placement.top);
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
                        self.blend_pixel(dx as u32, dy as u32, color, coverage);
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
                self.blend_pixel(x, y, straight, 1.0);
            }
        }
        self.images_drawn += 1;
    }

    /// Redraws every node, clipped to `region` (`None` = whole buffer).
    fn draw_all(
        &mut self,
        scene: &Scene,
        resources: Option<&ResourceRegistry>,
        region: Option<Rect>,
    ) {
        let clip = self.clip_region(region);
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius: 0.0,
                } => self.fill_rect(
                    bounds.x0.floor() as i32,
                    bounds.y0.floor() as i32,
                    bounds.x1.ceil() as i32,
                    bounds.y1.ceil() as i32,
                    *fill,
                    clip,
                ),
                // A non-zero radius takes the AA path, which works in the
                // node's own (sub-pixel) geometry instead of the integer grid.
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                } => self.fill_rounded_rect(*bounds, f64::from(*radius), *fill, clip),
                SceneNode::Text {
                    origin,
                    glyphs,
                    color,
                    ..
                } => {
                    if let Some(resources) = resources {
                        self.draw_text(*origin, *glyphs, *color, resources, clip);
                    }
                }
                SceneNode::Image { bounds, image } => {
                    if let Some(resources) = resources {
                        self.draw_image(*bounds, *image, resources, clip);
                    }
                }
                SceneNode::Group { .. } => {}
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
        let _ = scene.insert(SceneNode::Image {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(2.0, 2.0)),
            image: img,
        });
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
}
