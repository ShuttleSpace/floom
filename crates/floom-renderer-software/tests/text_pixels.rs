//! Pixel-level evidence for text, at the **rendering** level.
//!
//! `floom-text` already has a regression test for the macOS-gate finding: CJK
//! text must not shape to `.notdef` glyphs. That test is about *shaping* —
//! the right glyph id comes out of the shaper.
//!
//! This one is about *drawing*: the shaped glyphs reach a backend and come out
//! as ink on a surface. The two fail differently. A backend that resolves a
//! glyph but never rasterises it produces a scene that looks right and a
//! screen that is empty, and no shaping test can see that.
//!
//! # Why the text is Latin here and the CJK case is not
//!
//! The CJK case needs a font that is installed, which is a property of the
//! machine rather than of the code — the same reason
//! `floom-text`'s own CJK test is macOS-gated. So the *portable* evidence
//! here is Latin text, and the CJK evidence stays where the font is known to
//! exist.
//!
//! The check that matters is that **ink appears where the glyph is**, and
//! that the ink is the *shape* of a letter rather than a filled box: a
//! backend that drew every glyph as a solid rectangle would pass a
//! "something was drawn" test and fail this one.

use floom_renderer::{Renderer, render_once};
use floom_renderer_software::SoftwareRenderer;
use floom_scene::{Rect, ResourceRegistry, Scene, SceneNode};
use floom_text::{TextLayoutOptions, TextShaper};
use floom_types::{Color, Point, Size};

/// The size of the surface the text is drawn on.
const SIZE: (u32, u32) = (64, 24);

/// A scene with one line of text at the origin.
fn text_scene(text: &str) -> (Scene, ResourceRegistry) {
    let mut shaper = TextShaper::new();
    let laid = shaper.shape(text, &TextLayoutOptions::default());
    let runs = laid.runs.clone();
    let width = laid.lines.first().map(|l| l.width).unwrap_or(0.0f32);

    let mut registry = ResourceRegistry::new();
    // The string the runs were shaped from travels with the resource: shaping
    // is lossy, so a backend that needs characters rather than ink cannot
    // recover them later.
    let glyphs = registry.insert(floom_scene::Resource {
        kind: floom_scene::ResourceKind::glyph_runs_with_text(runs, text),
    });

    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Text {
        origin: Point::new(2.0, 2.0),
        size: Size::new(width as f64, 16.0),
        glyphs,
        color: Color::rgb(0, 0, 0),
    });
    scene.finish_update();
    (scene, registry)
}

fn render(text: &str) -> SoftwareRenderer {
    let (scene, registry) = text_scene(text);
    let mut sw = SoftwareRenderer::new(SIZE.0, SIZE.1);
    // `render_scene_with_resources`, not `render_scene`: the glyph runs live
    // in the resource registry, and the plain call has no way to reach them.
    // A backend that skipped the registry would draw nothing at all — which
    // is exactly what this test is here to notice.
    sw.begin_frame(&floom_renderer::FrameInfo::software(SIZE.0, SIZE.1, 1));
    sw.render_scene_with_resources(&scene, &scene.damage(), &registry);
    sw.end_frame();
    sw
}

/// How many pixels carry any ink at all.
fn inked(sw: &SoftwareRenderer) -> usize {
    let mut count = 0;
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            if sw.pixel_rgba(x, y).is_some_and(|p| p[3] != 0) {
                count += 1;
            }
        }
    }
    count
}

#[test]
fn text_reaches_the_surface_as_ink() {
    let sw = render("Hello");
    let ink = inked(&sw);
    assert!(
        ink > 0,
        "a shaped glyph run must reach the surface as ink: the scene was right \
         but nothing was drawn"
    );
}

#[test]
fn an_empty_scene_draws_nothing() {
    // The control for the test above, and the reason it means something.
    //
    // "Some pixels are inked" only says something next to "the same surface
    // with no text has none": otherwise a backend that filled the whole
    // buffer would pass.
    let mut scene = Scene::new();
    scene.begin_update();
    scene.finish_update();
    let mut sw = SoftwareRenderer::new(SIZE.0, SIZE.1);
    render_once(
        &mut sw,
        &floom_renderer::FrameInfo::software(SIZE.0, SIZE.1, 1),
        &scene,
    );
    assert_eq!(inked(&sw), 0, "an empty scene draws nothing");
}

#[test]
fn the_ink_is_a_glyph_shape_not_a_filled_box() {
    // A letter is a shape with holes; a filled rectangle is not.
    //
    // The check: the *middle* of the line's bounding box has fewer inked
    // pixels than the whole box does. A backend that filled each glyph's
    // advance rectangle would make the interior as dense as the edges, and
    // this would fail.
    let sw = render("Hello");
    let mut min_x = u32::MAX;
    let mut max_x = 0u32;
    let mut min_y = u32::MAX;
    let mut max_y = 0u32;
    for y in 0..SIZE.1 {
        for x in 0..SIZE.0 {
            if sw.pixel_rgba(x, y).is_some_and(|p| p[3] != 0) {
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    assert!(
        max_x > min_x && max_y > min_y,
        "there is a shape to measure"
    );

    let total = ((max_x - min_x + 1) * (max_y - min_y + 1)) as usize;
    let ink = inked(&sw);
    assert!(
        ink * 3 < total * 2,
        "the ink covers {ink} of {total} pixels in the bounding box: that is \
         a filled shape, not a letter"
    );
}

#[test]
fn the_text_lands_where_the_scene_put_it() {
    // The origin is (2, 2), so no ink may appear to its left or above.
    let sw = render("Hello");
    for y in 0..2 {
        for x in 0..SIZE.0 {
            let alpha = sw.pixel_rgba(x, y).map(|p| p[3]).unwrap_or(0);
            assert_eq!(alpha, 0, "no ink above the origin at ({x}, {y})");
        }
    }
    for x in 0..2 {
        for y in 0..SIZE.1 {
            let alpha = sw.pixel_rgba(x, y).map(|p| p[3]).unwrap_or(0);
            assert_eq!(alpha, 0, "no ink to the left of the origin at ({x}, {y})");
        }
    }
}

#[test]
fn a_longer_string_draws_more_ink() {
    // A crude but real check that the glyphs are being *placed*, not stamped
    // in the same spot: more characters means more ink.
    let short = inked(&render("Hi"));
    let long = inked(&render("Hello world"));
    assert!(
        long > short,
        "more text should mean more ink: {short} for two characters, {long} for eleven"
    );
}

#[test]
fn a_rect_is_not_mistaken_for_text() {
    // The two must not be confused: a filled rect is all ink, text is not.
    // This is the check that would catch a backend that drew a placeholder
    // instead of the glyphs.
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect {
            x0: 2.0,
            y0: 2.0,
            x1: 30.0,
            y1: 18.0,
        },
        fill: floom_types::Brush::Solid(Color::rgb(0, 0, 0)),
        radius: 0.0,
        shadow: None,
    });
    scene.finish_update();
    let mut sw = SoftwareRenderer::new(SIZE.0, SIZE.1);
    render_once(
        &mut sw,
        &floom_renderer::FrameInfo::software(SIZE.0, SIZE.1, 1),
        &scene,
    );

    let box_ink = inked(&sw);
    let text_ink = inked(&render("Hello"));
    assert!(
        box_ink > text_ink,
        "a filled box ({box_ink}) has more ink than a word ({text_ink}): the two \
         are not the same thing, and a backend that drew a box for the text \
         would be caught here"
    );
}
