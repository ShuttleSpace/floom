//! CPU rasterizer adapter: fills scene rects into an RGBA8 buffer.
//!
//! The buffer holds **straight (non-premultiplied)** RGBA, matching what the
//! plain fill path writes. Rounded rects get analytic anti-aliasing: coverage
//! comes from the rounded-rect signed distance evaluated at the pixel centre,
//! and edge pixels blend src-over so curves land between pixels instead of
//! being chopped to the integer grid.

mod presenter;

pub use presenter::SoftwarePresenter;

use floem_renderer::{FrameInfo, RenderTarget, Renderer};
use floem_scene::{DamageRegion, Rect, Scene, SceneNode};
use floem_types::Color;

/// Software renderer backend implementing the Floem 2.0 [`Renderer`] ABI.
pub struct SoftwareRenderer {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
    pub frames: u64,
    pub rects_drawn: u64,
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

    fn fill_rect(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let x0 = x0.clamp(0, self.width as i32) as u32;
        let y0 = y0.clamp(0, self.height as i32) as u32;
        let x1 = x1.clamp(0, self.width as i32) as u32;
        let y1 = y1.clamp(0, self.height as i32) as u32;
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
    fn fill_rounded_rect(&mut self, bounds: Rect, radius: f64, color: Color) {
        let radius = radius.clamp(0.0, bounds.width().min(bounds.height()) / 2.0);
        let x0 = bounds.x0.floor().max(0.0) as u32;
        let y0 = bounds.y0.floor().max(0.0) as u32;
        let x1 = (bounds.x1.ceil() as u32).min(self.width);
        let y1 = (bounds.y1.ceil() as u32).min(self.height);
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
                let i = ((y * self.width + x) as usize) * 4;
                if coverage >= 1.0 && color.a == u8::MAX {
                    // Fully covered opaque source: plain overwrite, identical
                    // to the axis-aligned path.
                    self.pixels[i] = color.r;
                    self.pixels[i + 1] = color.g;
                    self.pixels[i + 2] = color.b;
                    self.pixels[i + 3] = color.a;
                    continue;
                }
                // src-over in straight alpha: the source is scaled by
                // (alpha × coverage), the destination survives behind it.
                let sa = f32::from(color.a) / 255.0 * coverage as f32;
                let da = f32::from(self.pixels[i + 3]) / 255.0;
                let out_a = sa + da * (1.0 - sa);
                if out_a <= 0.0 {
                    self.pixels[i..i + 4].fill(0);
                    continue;
                }
                for (c, &src8) in [color.r, color.g, color.b].iter().enumerate() {
                    let src = f32::from(src8) / 255.0;
                    let dst = f32::from(self.pixels[i + c]) / 255.0;
                    let out = (src * sa + dst * da * (1.0 - sa)) / out_a;
                    self.pixels[i + c] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
                }
                self.pixels[i + 3] = (out_a * 255.0).round().clamp(0.0, 255.0) as u8;
            }
        }
        self.rects_drawn += 1;
    }
}

impl Renderer for SoftwareRenderer {
    fn begin_frame(&mut self, info: &FrameInfo) {
        if let RenderTarget::Software { width, height } = info.target {
            if width != self.width || height != self.height {
                self.width = width;
                self.height = height;
                self.pixels
                    .resize((width as usize) * (height as usize) * 4, 0);
            }
        }
        self.pixels.fill(0);
    }

    fn render_scene(&mut self, scene: &Scene, _damage: &DamageRegion) {
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
                ),
                // A non-zero radius takes the AA path, which works in the
                // node's own (sub-pixel) geometry instead of the integer grid.
                SceneNode::Rect {
                    bounds,
                    fill,
                    radius,
                } => self.fill_rounded_rect(*bounds, f64::from(*radius), *fill),
                // Text and image nodes have no software path yet; the buffer
                // keeps the rect coverage only.
                _ => {}
            }
        }
    }

    fn end_frame(&mut self) {
        self.frames += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use floem_renderer::{FrameInfo, render_once};
    use floem_scene::{Point, Rect, SceneNode, Size};
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
        // Red dominates the blend but the channel is scaled down from 200 by
        // coverage — the alpha ramp keeps rgb == a ratio consistent-ish; just
        // assert it moved toward the fill colour from transparent black.
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
        // Rounded end caps: (1,3) is the left cap corner → partial or none,
        // (6,4) the right cap corner → partial or none. Neither may exceed the
        // shape, and the far columns stay empty.
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
}
