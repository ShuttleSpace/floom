#![cfg(feature = "timeline")]
use floom_core::{ElementId, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_material::widgets::timeline::{MaterialTimeline, MaterialTimelineStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Material);
    MultiTheme::new_for(DesignSystem::Material)
}

fn kit() -> MaterialTimeline {
    MaterialTimeline::new().with_theme(theme())
}

/// Theme colour -> engine colour.
fn paint(c: floom_core_theme::tokens::color::Color) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// One widget's paint ops.
fn ops_of(widget: &dyn Widget) -> Vec<PaintOp> {
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = widget.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    widget.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().to_vec()
}

/// The tokens and the colours reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: MaterialTimelineStyle = kit().resolved_style();
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Timeline)
    {
        floom_core_theme::component_tokens::ComponentTokens::Timeline(tokens) => tokens,
        _ => unreachable!(),
    };
    assert_eq!(style.dot_size, tokens.dot_size, "the token's dot size");
    assert_eq!(
        style.line_width, tokens.line_width,
        "the token's line width"
    );
    assert_eq!(style.padding, tokens.padding, "the token's padding");
    assert_eq!(
        style.dot_color,
        theme().colors().primary,
        "the dot's colour"
    );
    assert_eq!(
        style.line_color,
        theme().colors().outline_variant,
        "the line's colour"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "material");
}

#[test]
fn the_dot_and_the_line_are_painted_in_v1_s_colours() {
    let resolved = kit().resolved_style();
    let column = kit()
        .item("Started", "The first entry")
        .item("Finished", "The second entry")
        .view();

    let row = column.children().remove(0).build();
    let dot_column = row.children().remove(0).build();

    let dot = dot_column.children().remove(0).build();
    let dot_ops = ops_of(&*dot);
    let dot_fill = dot_ops.iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(dot_fill, Some(paint(resolved.dot_color)), "the dot's fill");
    let dot_request = dot
        .layout_request()
        .expect("the dot asks for its own space");
    assert_eq!(
        dot_request.width,
        Some(floom_core::Length::Px(resolved.dot_size)),
        "and the dot's size"
    );

    let line = dot_column.children().remove(1).build();
    let line_ops = ops_of(&*line);
    let line_fill = line_ops.iter().find_map(|op| match op {
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    });
    assert_eq!(
        line_fill,
        Some(paint(resolved.line_color)),
        "the line's fill"
    );
    let line_request = line
        .layout_request()
        .expect("the line asks for its own space");
    assert_eq!(
        line_request.width,
        Some(floom_core::Length::Px(resolved.line_width)),
        "and the line's thickness"
    );
}

#[test]
fn the_title_and_the_description_are_painted_in_v1_s_ink() {
    let column = kit()
        .item("Started", "The first entry")
        .item("Finished", "The second entry")
        .view();
    let row = column.children().remove(0).build();
    let text_column = row.children().remove(1).build();

    let title = text_column.children().remove(0).build();
    let title_ops = ops_of(&*title);
    let (title_text, title_size, title_ink) = title_ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the title is painted");
    assert_eq!(title_text, "Started");
    assert_eq!(title_size, 15.0, "v1's title size");
    assert_eq!(
        title_ink,
        paint(theme().colors().on_surface),
        "v1's title ink"
    );

    let description = text_column.children().remove(1).build();
    let description_ops = ops_of(&*description);
    let (text, size, ink) = description_ops
        .iter()
        .find_map(|op| match op {
            PaintOp::Text {
                text, size, color, ..
            } => Some((text.clone(), *size, *color)),
            _ => None,
        })
        .expect("the description is painted");
    assert_eq!(text, "The first entry");
    assert_eq!(size, 13.0, "v1's description size");
    assert_eq!(
        ink,
        paint(theme().colors().on_surface_variant),
        "v1's description ink"
    );
}

#[test]
fn the_last_entry_has_no_line() {
    let column = kit().item("One", "First").item("Two", "Second").view();
    assert_eq!(column.children().len(), 2, "two entries");

    let last = column.children().remove(1).build();
    let dot_column = last.children().remove(0).build();
    let line = dot_column.children().remove(1).build();
    let request = line
        .layout_request()
        .expect("the line asks for its own space");
    assert_eq!(
        request.width,
        Some(floom_core::Length::Px(0.0)),
        "v1 gave the last entry's line a width of zero"
    );
}

#[test]
fn the_entries_have_no_gap_between_them() {
    let column = kit().item("One", "First").item("Two", "Second").view();
    let request = column
        .layout_request()
        .expect("the column asks for its own space");
    assert_eq!(
        request.gap,
        Some(0.0),
        "the gap v1 wrote never had two children to separate"
    );
}

/// The props report the entries.
#[test]
fn the_props_report_the_entries() {
    let kit = kit().item("One", "First").item("Two", "Second");
    assert_eq!(kit.props().items.len(), 2);
    assert_eq!(kit.props().items[0].title, "One");
    assert_eq!(kit.props().items[1].description, "Second");
}

#[test]
fn the_theme_is_the_app_wide_one() {
    assert_eq!(
        MaterialTimeline::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Material timeline read the app-wide signal, not a snapshot"
    );
}
