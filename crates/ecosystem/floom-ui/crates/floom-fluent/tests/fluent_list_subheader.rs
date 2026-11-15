#![cfg(feature = "list_subheader")]
//! Fluent list subheader: the row, the text in it, and the two paddings.

use floom_core::{ElementId, Widget};
use floom_types::Rect;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};
use floom_widgets::{Align, Surface};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_fluent::widgets::list_subheader::FluentListSubheader;

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 48.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn tokens() -> floom_core_theme::component_tokens::ListSubheaderTokens {
    let theme = theme();
    match theme.component(ComponentId::ListSubheader) {
        ComponentTokens::ListSubheader(t) => t,
        _ => unreachable!(),
    }
}

#[test]
fn the_row_declares_the_tokens_height_and_padding() {
    let surface = FluentListSubheader::new("Section")
        .with_theme(theme())
        .view();
    let request = surface
        .layout_request()
        .expect("a row asks for its own space");
    assert_eq!(
        request.height,
        Some(floom_types::Length::Px(tokens().height)),
        "the token's height"
    );
    assert_eq!(
        request.padding,
        Some(floom_types::Insets {
            left: f64::from(tokens().padding_horizontal),
            right: f64::from(tokens().padding_horizontal),
            top: 0.0,
            bottom: 0.0,
        }),
        "and the token's horizontal padding"
    );
    assert_eq!(
        request.cross_align,
        Some(Align::Center),
        "v1's `items_center()`"
    );
}

#[test]
fn the_inset_flag_chooses_the_other_padding() {
    let surface = FluentListSubheader::new("Section")
        .with_theme(theme())
        .inset(true)
        .view();
    let request = surface
        .layout_request()
        .expect("a row asks for its own space");
    assert_eq!(
        request.padding,
        Some(floom_types::Insets {
            left: f64::from(tokens().inset_padding),
            right: f64::from(tokens().inset_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: `inset_padding` when inset, `padding_horizontal` otherwise"
    );
    assert_ne!(
        tokens().inset_padding,
        tokens().padding_horizontal,
        "CONTROL: the two paddings really are different"
    );
}

#[test]
fn the_text_is_the_muted_ink_at_the_token_size() {
    let style = FluentListSubheader::new("Section")
        .with_theme(theme())
        .resolved_style();
    let surface = FluentListSubheader::new("Section")
        .with_theme(theme())
        .view();
    let mut child = surface.children()[0].build();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = child.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    child.paint(&mut recorder, id);
    recorder.finish();

    let painted = recorder.ops().iter().find_map(|op| match op {
        PaintOp::Text {
            color, size, text, ..
        } => Some((*color, *size, text.clone())),
        _ => None,
    });
    let (color, size, content) = painted.expect("the subheader draws its text");
    assert_eq!(content, "Section");
    assert_eq!(size, style.font_size);
    assert_eq!(color.r, style.color.r);
    assert_eq!(color.g, style.color.g);
    assert_eq!(color.b, style.color.b);
    assert_eq!(
        style.color,
        theme().colors().on_surface_variant,
        "and the ink is the theme's muted one"
    );
}

#[test]
fn the_surface_holds_the_text() {
    let surface = FluentListSubheader::new("Section")
        .with_theme(theme())
        .view();
    assert_eq!(surface.child_count(), 1, "the row holds the text");
    assert!(surface.children()[0]
        .build()
        .as_any()
        .is::<floom_widgets::Text>());
}
