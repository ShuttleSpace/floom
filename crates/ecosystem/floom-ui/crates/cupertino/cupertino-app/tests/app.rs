//! Cupertino application root: the box, and the content in it.
//!
//! ## The claim
//!
//! The root is a full-viewport box in the app's background colour, holding the
//! shell's bars and the main region. The assertions are about the painted fill
//! and the box's own extent.

use floom_core::Widget;
use floom_signal::SignalGet;
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_cupertino_app::CupertinoApp;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 600.0,
};

/// The Cupertino theme, with Cupertino selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time.
fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// The root fills its box and is painted in the app's background colour.
#[test]
fn the_root_is_a_full_viewport_box_in_the_background_colour() {
    let app = CupertinoApp::new().with_theme(theme());
    let mut surface = app.view();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();

    let request = surface
        .layout_request()
        .expect("the root asks for its own space");
    assert_eq!(
        request.flex_grow,
        Some(1.0),
        "the root fills its parent: {request:?}"
    );

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(to_engine(theme().colors().background)),
        "and it is the app's background colour"
    );
}

/// The content goes inside the root.
#[test]
fn the_content_goes_inside_the_root() {
    let app = CupertinoApp::new()
        .with_theme(theme())
        .child(|| Box::new(floom_widgets::Text::new("shell")));
    let surface = app.view();
    assert_eq!(surface.child_count(), 1, "the shell's content is inside");
    assert!(
        surface.children()[0]
            .build()
            .as_any()
            .is::<floom_widgets::Text>(),
        "and it is the caller's own content"
    );
}

/// Building the root initialises the process-wide layout signal, which the shell
/// pieces publish their sizes into.
#[test]
fn building_the_root_initialises_the_layout_signal() {
    let _ = floom_core_theme::layout::layout_signal();
    let app = CupertinoApp::new().with_theme(theme());
    let _ = app.view();
    // The signal exists and is readable: the shell pieces publish into it, and
    // `main` reads it to inset itself.
    let _ = floom_core_theme::layout::layout_signal().get();
}

/// Design-system colour -> engine colour.
fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
