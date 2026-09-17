//! Skia-oriented Scene adapter (command recording; GPU submit stays in legacy crate).

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub enum SkiaCommand {
    FillRect { bounds: Rect, color: Color },
    DrawImage { bounds: Rect, alpha: u8 },
    DrawText { bounds: Rect },
}

#[derive(Debug, Default)]
pub struct SkiaRendererAdapter {
    pub commands: Vec<SkiaCommand>,
    pub frames: u64,
}

impl Renderer for SkiaRendererAdapter {
    fn begin_frame(&mut self, _info: &FrameInfo) {
        self.commands.clear();
    }

    fn render_scene(&mut self, scene: &Scene, _damage: &DamageRegion) {
        for (_id, node) in scene.iter() {
            match node {
                SceneNode::Rect { bounds, fill, .. } => self.commands.push(SkiaCommand::FillRect {
                    bounds: *bounds,
                    color: *fill,
                }),
                SceneNode::Image {
                    bounds,
                    image: _,
                    alpha,
                } => self.commands.push(SkiaCommand::DrawImage {
                    bounds: *bounds,
                    alpha: *alpha,
                }),
                SceneNode::Text { origin, size, .. } => self.commands.push(SkiaCommand::DrawText {
                    bounds: Rect::from_origin_size(*origin, *size),
                }),
                SceneNode::Group { .. } => {}
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
    use floem_renderer_software::SoftwareRenderer;
    use floem_scene::{Point, SceneNode, Size};
    use floem_types::Color;

    #[test]
    fn skia_and_software_see_same_rect_scene() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(2.0, 2.0)),
            fill: Color::rgb(1, 2, 3),
            radius: 0.0,
        });
        scene.finish_update();

        let mut skia = SkiaRendererAdapter::default();
        render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);
        assert_eq!(skia.commands.len(), 1);

        let mut sw = SoftwareRenderer::new(16, 16);
        render_once(&mut sw, &FrameInfo::software(16, 16, 1), &scene);
        assert_eq!(sw.rects_drawn, 1);
        assert_eq!(sw.pixel_rgba(1, 1), Some([1, 2, 3, 255]));
    }
}
