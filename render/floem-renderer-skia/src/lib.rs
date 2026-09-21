//! Skia-oriented Scene adapter (command recording; GPU submit stays in legacy crate).

use floem_renderer::{FrameInfo, Renderer};
use floem_scene::{DamageRegion, Scene, SceneNode};
use floem_types::{Color, Rect};

#[derive(Debug, Clone, PartialEq)]
pub enum SkiaCommand {
    FillRect {
        bounds: Rect,
        /// The fill as the scene's own `Brush` vocabulary: the real submit
        /// path maps a gradient to an `SkShader`, a solid to a plain paint
        /// colour. Carried structurally so the contract tests can compare
        /// fills without linking Skia.
        brush: floem_types::Brush,
        /// Drop shadow requested behind the fill; the submit path turns it
        /// into an `SkBlurImageFilter`-based draw (see the type's docs).
        shadow: Option<floem_types::Shadow>,
    },
    /// A path, in Skia's own shape vocabulary: `SkPath` holds the segments and
    /// a `SkPaint` decides fill or stroke. Recorded as the segment count plus
    /// the style so the adapter's contract test can tell a fill from a stroke
    /// and a line from a circle without linking Skia.
    DrawPath {
        bounds: Rect,
        segments: usize,
        fill: Option<floem_types::Brush>,
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
    /// `canvas.save()` + `canvas.concat(matrix)` for a `GroupPush` marker.
    /// Recorded as the six matrix coefficients so the contract test can
    /// compare scopes without linking Skia.
    PushTransform {
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    },
    /// `canvas.restore()` for a `GroupPop`.
    PopTransform,
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
                SceneNode::Rect {
                    bounds,
                    fill,
                    shadow,
                    ..
                } => self.commands.push(SkiaCommand::FillRect {
                    bounds: *bounds,
                    brush: fill.clone(),
                    shadow: *shadow,
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
                        fill: fill.clone(),
                        stroke: stroke.as_ref().map(|s| (s.width, s.color)),
                    })
                }
                SceneNode::ClipPush { region } => self.commands.push(SkiaCommand::PushClip {
                    bounds: region.bounds(),
                    rounded: !matches!(region, floem_types::ClipRegion::Rect(_)),
                }),
                SceneNode::ClipPop => self.commands.push(SkiaCommand::PopClip),
                SceneNode::GroupPush { transform } => {
                    self.commands.push(SkiaCommand::PushTransform {
                        a: transform.a,
                        b: transform.b,
                        c: transform.c,
                        d: transform.d,
                        e: transform.e,
                        f: transform.f,
                    })
                }
                SceneNode::GroupPop => self.commands.push(SkiaCommand::PopTransform),
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
    use floem_types::{Color, Transform};

    #[test]
    fn skia_and_software_see_same_rect_scene() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(2.0, 2.0)),
            fill: floem_types::Brush::from(Color::rgb(1, 2, 3)),
            radius: 0.0,

            shadow: None,

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

    /// A transform scope reaches Skia as a save/concat pair bracketing the
    /// draws it wraps — the same stream position the scene carries.
    #[test]
    fn skia_records_a_transform_scope_bracketing_its_draws() {
        let mut scene = Scene::new();
        scene.begin_update();
        let _ = scene.insert(SceneNode::GroupPush {
            transform: Transform::rotate_around(0.5, Point::new(8.0, 8.0)),
        });
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(4.0, 4.0)),
            fill: floem_types::Brush::from(Color::rgb(9, 9, 9)),
            radius: 0.0,

            shadow: None,

        });
        let _ = scene.insert(SceneNode::GroupPop);
        scene.finish_update();

        let mut skia = SkiaRendererAdapter::default();
        render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);

        assert!(
            matches!(
                skia.commands.as_slice(),
                [
                    SkiaCommand::PushTransform { a, b, c, d, e, f },
                    SkiaCommand::FillRect { .. },
                    SkiaCommand::PopTransform,
                ] if (*a - 0.5f64.cos()).abs() < 1e-9
                    && (*b - 0.5f64.sin()).abs() < 1e-9
                    && (*c + 0.5f64.sin()).abs() < 1e-9
                    && (*d - 0.5f64.cos()).abs() < 1e-9
                    && e.is_finite()
                    && f.is_finite()
            ),
            "the scope brackets its draws and carries the rotation matrix: {:?}",
            skia.commands
        );
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
