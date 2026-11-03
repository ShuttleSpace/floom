//! Cupertino text: the theme's type, and the text it draws.
//!

use floom_core::{ElementId, Widget};
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

use floom_widget_cupertino_text::{to_engine_weight, CupertinoText};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

/// The Cupertino theme, with Cupertino selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Cupertino's own values has to
/// select Cupertino, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn text_widget(
    t: CupertinoText,
) -> (
    Option<Color>,
    Option<f32>,
    Option<floom_types::FontWeight>,
    Option<String>,
) {
    let mut w = t.view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = w.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    w.paint(&mut recorder, id);
    recorder.finish();
    let mut color = None;
    let mut size = None;
    let mut weight = None;
    let mut content = None;
    for op in recorder.ops() {
        if let PaintOp::Text {
            color: c,
            size: s,
            weight: w,
            text,
            ..
        } = op
        {
            color = Some(*c);
            size = Some(*s);
            weight = Some(*w);
            content = Some(text.clone());
        }
    }
    (color, size, weight, content)
}

/// The style resolves from the theme's own body type.
#[test]
fn the_style_resolves_from_the_themes_body_type() {
    let theme = theme();
    let style = CupertinoText::new("Hello")
        .with_theme(theme.clone())
        .resolved_style();
    let typography = theme.typography().body.medium.clone();
    assert_eq!(style.color, theme.colors().on_surface);
    assert_eq!(style.font_size, typography.font_size);
    assert_eq!(style.line_height, typography.line_height);
    assert_eq!(style.typography, typography);
}

/// The painted text carries the theme's ink, size and weight.
#[test]
fn the_painted_text_carries_the_themes_ink_and_type() {
    let theme = theme();
    let style = CupertinoText::new("Hello")
        .with_theme(theme.clone())
        .resolved_style();
    let (color, size, weight, content) =
        text_widget(CupertinoText::new("Hello").with_theme(theme.clone()));
    assert_eq!(content.as_deref(), Some("Hello"));
    assert_eq!(color, Some(to_paint(style.color)));
    assert_eq!(size, Some(style.font_size));
    assert_eq!(
        weight,
        Some(to_engine_weight(style.typography.font_weight)),
        "and the weight is the theme's, crossed into the engine's own enum"
    );
}

/// The weight crossing names all nine variants, and v1's one difference is
/// kept: v1's `to_floom()` sent `Black` to 800, because parley's table has no
/// 900. Here `Black` is the engine's 900.
#[test]
fn the_weight_crossing_names_all_nine_variants() {
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Thin),
        floom_types::FontWeight::Thin
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::ExtraLight),
        floom_types::FontWeight::ExtraLight
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Light),
        floom_types::FontWeight::Light
    );
    // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Regular),
        floom_types::FontWeight::Normal
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Medium),
        floom_types::FontWeight::Medium
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::SemiBold),
        floom_types::FontWeight::SemiBold
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Bold),
        floom_types::FontWeight::Bold
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::ExtraBold),
        floom_types::FontWeight::ExtraBold
    );
    assert_eq!(
        to_engine_weight(ThemeFontWeight::Black),
        floom_types::FontWeight::Black
    );
}

/// `max_lines` and `selectable` are carried on the props and not applied.
///
/// v1 set both and neither reached the view -- it drew a plain `Label` -- and
/// 2.0's `Text` has no knob for either. So the props keep reporting them and
/// no kit claims a truncation or a selection it does not perform.
#[test]
fn max_lines_and_selectable_are_carried_but_not_applied() {
    let text = CupertinoText::new("Hello").max_lines(2).selectable(true);
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);
}

/// Design-system colour -> engine colour, the same crossing the kit does.
fn to_paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
