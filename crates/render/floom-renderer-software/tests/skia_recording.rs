// SPDX-License-Identifier: MIT
//! Contract tests for the Skia-side recording adapter (`floom_renderer::skia_abi`).
//!
//! These live in `floom-renderer-software` rather than inside `floom-renderer`'s
//! own test build because they drive BOTH the skia recorder and the software
//! backend through the same `render_once`, and a crate cannot depend on its own
//! dev-dependency's copy of itself without splitting `floom_renderer` into two
//! instances. Here `SoftwareRenderer`, `render_once` and `SkiaRendererAdapter`
//! all resolve against one `floom_renderer`, so the trait bounds line up.

use floom_renderer::skia_abi::{SkiaCommand, SkiaRecording, SkiaRendererAdapter};
use floom_renderer::{FrameInfo, Scene, SceneNode, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_types::{Brush, Color, Filter, Point, Rect, Shader, Size, Transform};

#[test]
fn skia_and_software_see_same_rect_scene() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(2.0, 2.0)),
        fill: Brush::from(Color::rgb(1, 2, 3)),
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
        fill: Brush::from(Color::rgb(9, 9, 9)),
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

/// A filter request reaches the skia command list as the save-layer it
/// maps to, and a backdrop filter keeps its own rect.
#[test]
fn records_a_filter_scope_bracketing_its_draws() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::FilterPush {
        filter: Filter::blur(4.0),
    });
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(6.0, 6.0)),
        fill: Brush::from(Color::BLACK),
        radius: 0.0,
        shadow: None,
    });
    let _ = scene.insert(SceneNode::FilterPop);
    let _ = scene.insert(SceneNode::BackdropFilter {
        rect: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(8.0, 8.0)),
        filter: Filter::blur_xy(2.0, 7.0),
    });
    scene.finish_update();

    let mut skia = SkiaRendererAdapter::default();
    render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);
    assert_eq!(
        skia.commands,
        vec![
            SkiaCommand::PushFilter {
                filter: Filter::blur(4.0)
            },
            SkiaCommand::FillRect {
                bounds: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(6.0, 6.0)),
                brush: Brush::from(Color::BLACK),
                shadow: None,
            },
            SkiaCommand::PopFilter,
            SkiaCommand::BackdropFilter {
                rect: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(8.0, 8.0)),
                filter: Filter::blur_xy(2.0, 7.0),
            },
        ],
        "a filter is a layer, not a folded-in flag on the draws beneath it"
    );
}

/// The three backends do not share pixels, but they do share an *IR*: a
/// clipped, self-drawn scene must reach every adapter as the same marker
/// stream in the same order, or one backend would be clipping something
/// the others do not.
#[test]
fn skia_records_the_same_clip_brackets_and_paths_the_scene_carries() {
    use floom_types::{ClipRegion, Path, Point as P};

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

/// A custom-shader request reaches the headless command list intact.
///
/// Only the GPU submit path can actually run WGSL; the adapter cannot, so
/// it carries the request (the same "degradation is never silent" half of
/// A5 a `FilterPush` gets). A backend dropping it would leave the effect
/// reading as plain transparency with nothing to explain why.
#[test]
fn skia_records_a_shader_request_instead_of_dropping_it() {
    let shader = Shader::new(
        "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
         return vec4<f32>(in.uv, 0.0, 1.0); }",
        [0.5, 0.25],
    );
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::shader(
        Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
        shader.clone(),
    ));
    scene.finish_update();

    let mut skia = SkiaRendererAdapter::default();
    render_once(&mut skia, &FrameInfo::software(32, 32, 1), &scene);
    assert_eq!(skia.commands.len(), 1);
    let SkiaCommand::Shader {
        bounds,
        shader: carried,
    } = &skia.commands[0]
    else {
        panic!("expected a shader command, got {:?}", skia.commands[0]);
    };
    assert_eq!(
        *bounds,
        Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(12.0, 12.0)),
        "the bounds survive"
    );
    assert_eq!(carried, &shader, "and so does the program");

    // The command says what was asked for; the counter says that a Skia
    // submit path could not do it. Both, because a caller reading only
    // the commands would think the shader was drawn.
    assert_eq!(
        skia.shaders_skipped, 1,
        "a shader is a degradation for Skia and must be counted"
    );
}

/// A degradation is counted, never silent.
///
/// The rule the whole renderer layer follows, and the one this crate
/// exists to serve: a backend that cannot honour a request says so in a
/// number, so the number can be surfaced. A silent skip reads as a
/// feature that quietly does nothing.
#[test]
fn a_degradation_is_counted_rather_than_silent() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::shader(
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(4.0, 4.0)),
        Shader::new(
            "@fragment fn fs_main(in: VsOut) -> @location(0) vec4<f32> { \
             return vec4<f32>(in.uv, 0.0, 1.0); }",
            [0.0; 2],
        ),
    ));
    scene.finish_update();

    let mut skia = SkiaRecording::default();
    render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);

    assert_eq!(skia.shaders_skipped, 1, "counted");
    assert_eq!(
        skia.shaders_skipped + skia.filters_skipped + skia.shadows_skipped,
        1,
        "and only the shader was a degradation: Skia has an image filter and \
         a save layer, so the other two are things it can do"
    );
}

/// A frame with nothing to degrade has nothing to report.
///
/// The other half of the rule: a counter that always reads zero is as
/// useless as one that is never read.
#[test]
fn a_plain_frame_reports_no_degradation() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(2.0, 2.0)),
        fill: Brush::from(Color::rgb(1, 2, 3)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();

    let mut skia = SkiaRecording::default();
    render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);

    assert_eq!(skia.shaders_skipped, 0);
    assert_eq!(skia.filters_skipped, 0);
    assert_eq!(skia.shadows_skipped, 0);
    assert_eq!(skia.frames, 1);
}

/// The recording is not a renderer, and says so.
///
/// The name of the crate says it, this says it in code: there is no
/// surface, no pixels, and no framebuffer. A reader who reaches for this
/// crate expecting a backend has to be told.
#[test]
fn the_recorder_produces_no_pixels() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(2.0, 2.0)),
        fill: Brush::from(Color::rgb(255, 0, 0)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();

    let mut skia = SkiaRecording::default();
    render_once(&mut skia, &FrameInfo::software(16, 16, 1), &scene);

    // The command is there — the request was recorded faithfully.
    assert!(matches!(skia.commands[0], SkiaCommand::FillRect { .. }));
    // And there is nothing anywhere that holds a pixel: the type has no
    // buffer, no surface, and no framebuffer. What it has is commands.
    assert_eq!(skia.commands.len(), 1);
}
