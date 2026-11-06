//! The ported Fluent radio group -- a column of options, each a ring with a

use floom_core::{ElementId, Rect, Widget};
use floom_types::Color;
use floom_ui_composites::SelectGroup;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_radio_group::{FluentRadioGroup, FluentRadioGroupStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Fluent);
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn kit() -> FluentRadioGroup {
    FluentRadioGroup::new().with_theme(theme())
}

fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

fn row_of(group: &SelectGroup, index: usize) -> Box<dyn Widget> {
    let pressable = group.children()[index].build().children().remove(0).build();
    let surface = pressable.children().remove(0).build();
    surface.children().remove(0).build()
}

fn ring_of(row: &dyn Widget) -> Box<dyn Widget> {
    row.children().remove(0).build()
}

#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: FluentRadioGroupStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::RadioGroup) {
            floom_core_theme::component_tokens::ComponentTokens::RadioGroup(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.radio_size, tokens.radio_size, "the token's ring size");
    assert_eq!(
        style.item_height, tokens.item_height,
        "the token's row height"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap between rows");
    assert_eq!(style.indicator_color, theme().colors().primary, "the dot");
    assert_eq!(style.label_color, theme().colors().on_surface, "the label");
    assert_eq!(
        style.unselected_color,
        theme().colors().outline,
        "the unchosen ring"
    );
    assert_eq!(
        style.radius,
        theme().radius().full,
        "the theme's full radius"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "fluent");
}

#[test]
fn the_ring_and_the_dot_are_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let group = kit().option("One").option("Two").selected(0).view();

    let row = row_of(&group, 0);
    let ring = ring_of(&*row);
    let ring_ops = ops_of(&*ring);

    let (width, colour) = ring_ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Path {
                stroke: Some((width, colour)),
                ..
            } => Some((*width, *colour)),
            _ => None,
        })
        .expect("the ring is outlined");
    assert_eq!(width, 2.0, "v1's ring is two points thick");
    assert_eq!(
        colour,
        paint(resolved.indicator_color),
        "in v1's ring colour"
    );

    let dot = ring.children().remove(0).build();
    let dot_ops = ops_of(&*dot);
    let fill = dot_ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        fill,
        Some(paint(resolved.indicator_color)),
        "the dot's fill"
    );
    let request = dot
        .layout_request()
        .expect("the dot asks for its own space");
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(resolved.radio_size * 0.5)),
        "and the dot's size"
    );
}

#[test]
fn the_chosen_option_is_the_one_with_a_dot() {
    let group = kit().option("One").option("Two").selected(0).view();

    let chosen = ring_of(&*row_of(&group, 0));
    assert_eq!(
        chosen.children().len(),
        1,
        "the chosen option has the dot inside its ring"
    );

    let other = ring_of(&*row_of(&group, 1));
    assert_eq!(
        other.children().len(),
        0,
        "and the one that is not chosen has no dot at all"
    );
}

#[test]
fn the_label_is_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let group = kit().option("One").selected(0).view();
    let row = row_of(&group, 0);
    let label = row.children().remove(1).build();
    let ops = ops_of(&*label);
    let (text, size, ink) = ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the label is painted");
    assert_eq!(text, "One");
    assert_eq!(size, resolved.label_font_size, "v1's label size");
    assert_eq!(
        ink,
        paint(resolved.indicator_color),
        "and the chosen label takes the indicator's colour"
    );
}

#[test]
fn the_option_chosen_before_mount_is_the_chosen_one() {
    let group = kit().option("One").option("Two").selected(1).view();
    assert!(group.is_selected(1), "v1 seeded the choice from the props");
    assert!(!group.is_selected(0));
}

#[test]
fn a_disabled_group_does_not_answer() {
    let group = kit().option("One").option("Two").disabled(true).view();
    assert!(group.is_disabled(), "the group is turned off");
    assert!(
        !group.is_selected(0),
        "and nothing is chosen, because nothing can be chosen"
    );
}

#[test]
fn the_props_report_the_options() {
    let kit = kit().option("One").option("Two").selected(1);
    assert_eq!(
        kit.props().options,
        vec!["One".to_string(), "Two".to_string()]
    );
    assert_eq!(kit.props().selected, Some(1));
}

#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    assert_ne!(
        FluentRadioGroup::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Fluent radio group took a snapshot, not the app-wide signal"
    );
}
