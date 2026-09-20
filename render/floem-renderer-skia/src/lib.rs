//! Skia-oriented Scene adapter (command recording; GPU submit stays in legacy crate).

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub enum SkiaCommand {
    FillRect {
        bounds: Rect,
        color: Color,
    },
    /// A path, in Skia's own shape vocabulary: `SkPath` holds the segments and
    /// a `SkPaint` decides fill or stroke. Recorded as the segment count plus
    /// the style so the adapter's contract test can tell a fill from a stroke
    /// and a line from a circle without linking Skia.
    DrawPath {
        bounds: Rect,
        segments: usize,
        fill: Option<Color>,
        stroke: Option<(f64, Color)>,
    },
    /// `canvas.clipPath` / `clipRRect`. Skia keeps the same push/pop stack the
    /// scene does, so a clip is recorded rather than folded into the commands
    /// under it — folding is what the scene stack exists to avoid.
    PushClip {
        bounds: Rect,
        rounded: bool,
    },
    PopClip,
    DrawImage {
        bounds: Rect,
        alpha: u8,
    },
    DrawText {
        bounds: Rect,
    },
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
                SceneNode::Path { path, fill, stroke } => {
                    self.commands.push(SkiaCommand::DrawPath {
                        bounds: path.bounds(),
                        segments: path.segments().len(),
                        fill: *fill,
                        stroke: stroke.as_ref().map(|s| (s.width, s.color)),
                    })
                }
                SceneNode::ClipPush { region } => self.commands.push(SkiaCommand::PushClip {
                    bounds: region.bounds(),
                    rounded: !matches!(region, floem_types::ClipRegion::Rect(_)),
                }),
                SceneNode::ClipPop => self.commands.push(SkiaCommand::PopClip),
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

    /// The three backends do not share pixels, but they do share an *IR*: a
    /// clipped, self-drawn scene must reach every adapter as the same marker
    /// stream in the same order, or one backend would be clipping something
    /// the others do not.
    #[test]
    fn skia_records_the_same_clip_brackets_and_paths_the_scene_carries() {
        use floem_types::{ClipRegion, Path, Point as P};

        let mut scene = Scene::new();
        scene.begin_update();
        let mut path = Path::new();
        path.move_to(P::new(0.0, 0.0)).line_to(P::new(8.0, 8.0));
        let _ = scene.insert(SceneNode::ClipPush {
            region: ClipRegion::Rect(Rect::from_origin_size(
                P::new(0.0, 0.0),
                Size::new(8.0, 8.0),
            )),
        });
        let _ = scene.insert(SceneNode::filled_path(path, Color::rgb(9, 9, 9)));
        let _ = scene.insert(SceneNode::ClipPop);
        scene.finish_update();

        let mut skia = SkiaRendererAdapter::default();
        render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);

        // Order matters: the pop after the path is what keeps the clip off the
        // nodes that follow it.
        assert!(
            matches!(
                skia.commands.as_slice(),
                [
                    SkiaCommand::PushClip { rounded: false, .. },
                    SkiaCommand::DrawPath {
                        fill: Some(_),
                        stroke: None,
                        ..
                    },
                    SkiaCommand::PopClip
                ]
            ),
            "unexpected command stream: {:?}",
            skia.commands
        );

        // Control: the same walk in the software backend reports one clip and
        // one path too, so this is the scene's shape and not one adapter's.
        let mut sw = SoftwareRenderer::new(16, 16);
        render_once(&mut sw, &FrameInfo::software(16, 16, 1), &scene);
        assert_eq!(sw.clips_resolved, 1);
        assert_eq!(sw.paths_drawn, 1);
    }
}
