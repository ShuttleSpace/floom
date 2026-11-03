//! Fluent avatar: the shape, the palette, and the type.

use floom_core::Widget;
use floom_types::{Color, FontWeight, Rect};
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{set_design_system, DesignSystem};
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_avatar::{FluentAvatar, FluentAvatarStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 64.0,
    y1: 64.0,
};

fn theme() -> MultiTheme {
    set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::AvatarTokens {
    let theme = theme();
    match theme.component(ComponentId::Avatar) {
        ComponentTokens::Avatar(t) => t,
        _ => unreachable!(),
    }
}

fn painted(
    b: FluentAvatar,
) -> (
    Option<Color>,
    Option<Color>,
    Option<floom_types::FontWeight>,
) {
    let mut surface = b.view();
    let mut create = AllocatingCreateContext::new();
    let id = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();

    let fill = recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        _ => None,
    });

    let mut text = surface.children()[0].build();
    let mut create = AllocatingCreateContext::new();
    let text_id = text.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    text.paint(&mut recorder, text_id);
    recorder.finish();
    let (ink, weight) = recorder
        .ops()
        .iter()
        .find_map(|op| match op {
            PaintOp::Text { color, weight, .. } => Some((*color, *weight)),
            _ => None,
        })
        .unzip();

    (fill, ink, weight)
}

#[test]
fn the_style_resolves_this_kits_own_palette() {
    let theme = theme();
    let style: FluentAvatarStyle = FluentAvatar::new()
        .with_theme(theme.clone())
        .resolved_style();
    assert_eq!(style.design_system, "fluent");
    assert_eq!(style.background, theme.colors().primary_container);
    assert_eq!(style.text_color, theme.colors().on_primary_container);
    assert_eq!(style.size, 40.0);
    assert_eq!(style.font_size, tokens().font_size);
}

#[test]
fn the_props_size_wins_over_the_tokens() {
    let style = FluentAvatar::new()
        .with_theme(theme())
        .size(64.0)
        .resolved_style();
    assert_eq!(style.size, 64.0, "the props' size wins");

    let style = FluentAvatar::new().with_theme(theme()).resolved_style();
    assert_eq!(
        style.size, 40.0,
        "the props' own default, which is why the token's size is never read"
    );
}

#[test]
fn the_shape_is_square() {
    let b = FluentAvatar::new().with_theme(theme()).content("AB");
    let style = b.resolved_style();
    let (fill, ink, weight) = painted(b);

    assert_eq!(
        fill,
        Some(to_engine(style.background)),
        "the fill is this kit's background"
    );
    assert_eq!(
        ink,
        Some(to_engine(style.text_color)),
        "and the ink is this kit's text colour"
    );
    assert_eq!(
        weight,
        Some(FontWeight::SemiBold),
        "and the text is set in FontWeight::SemiBold"
    );
}

fn to_engine(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
