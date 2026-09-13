//! CPU rasterizer adapter: fills scene rects into an RGBA8 buffer.

mod presenter;

pub use presenter::SoftwarePresenter;

use floem_renderer::{FrameInfo, RenderTarget, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
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
            if let SceneNode::Rect { bounds, fill } = node {
                self.fill_rect(
                    bounds.x0.floor() as i32,
                    bounds.y0.floor() as i32,
                    bounds.x1.ceil() as i32,
                    bounds.y1.ceil() as i32,
                    *fill,
                );
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
        });
        scene.finish_update();

        let mut sw = SoftwareRenderer::new(8, 8);
        render_once(&mut sw, &FrameInfo::software(8, 8, 1), &scene);
        assert_eq!(sw.frames, 1);
        assert_eq!(sw.rects_drawn, 1);
        assert_eq!(sw.pixel_rgba(3, 3), Some([10, 20, 30, 255]));
        assert_eq!(sw.pixel_rgba(0, 0), Some([0, 0, 0, 0]));
    }
}
