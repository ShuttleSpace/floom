
use floom_core::{ElementId, Widget};
use floom_types::{Color, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;

use floom_widget_md_text::{to_engine_weight, MaterialText};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn text_widget(
    t: MaterialText,
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
    let style = MaterialText::new("Hello")
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
    let style = MaterialText::new("Hello")
        .with_theme(theme.clone())
        .resolved_style();
    let (color, size, weight, content) =
        text_widget(MaterialText::new("Hello").with_theme(theme.clone()));
    assert_eq!(content.as_deref(), Some("Hello"));
    assert_eq!(color, Some(to_paint(style.color)));
    assert_eq!(size, Some(style.font_size));
    assert_eq!(
        weight,
        Some(to_engine_weight(style.typography.font_weight)),
        "and the weight is the theme's, crossed into the engine's own enum"
    );
}

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

#[test]
fn max_lines_and_selectable_are_carried_but_not_applied() {
    let text = MaterialText::new("Hello").max_lines(2).selectable(true);
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
