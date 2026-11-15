#![cfg(feature = "empty_state")]
//! The ported Cupertino empty state -- an icon, a title, a description, and
//! whatever the caller wants under them.

use floom_core::{Child, ChildKey, ElementId, LayoutRequest, Rect, Widget};
use floom_types::Color;
use floom_widgets::testkit::{AllocatingCreateContext, PaintOp, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_cupertino::widgets::empty_state::{CupertinoEmptyState, CupertinoEmptyStateStyle};

const BOX: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 400.0,
    y1: 400.0,
};

fn theme() -> MultiTheme {
    // `MultiTheme::new_for` builds all three palettes and reads the *current*
    // one, and `set_design_system` writes a thread-local, so the selection is
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

fn kit() -> CupertinoEmptyState {
    CupertinoEmptyState::new().with_theme(theme())
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

/// The surface's fill.
fn surface_fill(kit: CupertinoEmptyState) -> Option<Color> {
    let surface = kit.view();
    let mut create = AllocatingCreateContext::new();
    let id: ElementId = surface.create(&mut create);
    let mut recorder = SceneRecorder::new(BOX);
    surface.paint(&mut recorder, id);
    recorder.finish();
    recorder.ops().iter().find_map(|op| match op {
        PaintOp::RoundedRect { color, .. } => Some(*color),
        PaintOp::Rect { color, .. } => Some(*color),
        _ => None,
    })
}

/// The lines the column holds: their text, their size and their ink.
fn lines(kit: CupertinoEmptyState) -> Vec<(String, f32, Color)> {
    let surface = kit.view();
    let column = surface.children().remove(0).build();
    let mut out = Vec::new();
    for child in column.children() {
        let widget = child.build();
        let mut create = AllocatingCreateContext::new();
        let id: ElementId = widget.create(&mut create);
        let mut recorder = SceneRecorder::new(BOX);
        widget.paint(&mut recorder, id);
        recorder.finish();
        for op in recorder.ops() {
            if let PaintOp::Text {
                text, size, color, ..
            } = op
            {
                out.push((text.clone(), *size, *color));
            }
        }
    }
    out
}

// --- the translation ------------------------------------------------------

/// The tokens and the ink reach the style.
#[test]
fn the_tokens_and_colours_reach_the_style() {
    let style: CupertinoEmptyStateStyle = kit().resolved_style();
    let tokens =
        match theme().component(floom_core_theme::component_tokens::ComponentId::EmptyState) {
            floom_core_theme::component_tokens::ComponentTokens::EmptyState(tokens) => tokens,
            _ => unreachable!(),
        };
    assert_eq!(style.icon_size, tokens.icon_size, "the token's icon size");
    assert_eq!(
        style.title_font_size, tokens.title_font_size,
        "the title's size"
    );
    assert_eq!(
        style.description_font_size, tokens.description_font_size,
        "the description's size"
    );
    assert_eq!(style.gap, tokens.gap, "the token's gap");
    assert_eq!(
        style.max_width, tokens.max_width,
        "the column's maximum width"
    );
    assert_eq!(
        style.on_surface,
        theme().colors().on_surface,
        "the title's ink"
    );
    assert_eq!(
        style.on_surface_variant,
        theme().colors().on_surface_variant,
        "the icon's and the description's ink"
    );
    assert_eq!(
        style.surface,
        theme().colors().surface,
        "the surface it sits on"
    );
}

#[test]
fn the_design_system_is_named_in_the_style() {
    assert_eq!(kit().resolved_style().design_system, "cupertino");
}

// --- the pixels -----------------------------------------------------------

/// The surface is painted the colour v1 painted.
#[test]
fn the_surface_is_painted_v1_s_colour() {
    let resolved = kit().resolved_style();
    assert_eq!(
        surface_fill(kit()),
        Some(paint(resolved.surface)),
        "the surface the column sits on"
    );
}

/// The three lines are painted in v1's ink at v1's sizes.
#[test]
fn the_three_lines_are_painted_in_v1_s_ink() {
    let resolved = kit().resolved_style();
    let lines = lines(
        kit()
            .icon("!")
            .title("Nothing here")
            .description("Try something else"),
    );
    assert_eq!(lines.len(), 3, "the icon, the title and the description");
    assert_eq!(lines[0].0, "!");
    assert_eq!(lines[0].1, resolved.icon_size, "the icon's size");
    assert_eq!(
        lines[0].2,
        paint(resolved.on_surface_variant),
        "the icon's ink"
    );
    assert_eq!(lines[1].0, "Nothing here");
    assert_eq!(lines[1].1, resolved.title_font_size, "the title's size");
    assert_eq!(lines[1].2, paint(resolved.on_surface), "the title's ink");
    assert_eq!(lines[2].0, "Try something else");
    assert_eq!(
        lines[2].1, resolved.description_font_size,
        "the description's size"
    );
    assert_eq!(
        lines[2].2,
        paint(resolved.on_surface_variant),
        "the description's ink"
    );
}

/// v1 left out the icon and the description when they were empty, and so does
/// this.
#[test]
fn an_empty_icon_and_description_leave_no_line() {
    let lines = lines(kit().title("Nothing here"));
    assert_eq!(lines.len(), 1, "only the title");
    assert_eq!(lines[0].0, "Nothing here");
}

/// The caller's child is mounted under the text.
#[test]
fn the_callers_child_is_mounted_under_the_text() {
    let child = Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(floom_widgets::Text::new("Action"))
    });
    let surface = kit().title("Nothing here").child(child).view();
    let column = surface.children().remove(0).build();
    assert_eq!(
        column.children().len(),
        2,
        "the title and the caller's child"
    );
}

/// The props report the text.
#[test]
fn the_props_report_the_text() {
    let kit = kit()
        .icon("!")
        .title("Nothing here")
        .description("Try something else");
    assert_eq!(kit.props().icon, "!");
    assert_eq!(kit.props().title, "Nothing here");
    assert_eq!(kit.props().description, "Try something else");
}

/// The theme: v1's Cupertino empty state took a *snapshot* of its own design
/// system, so a change of the app-wide theme does not reach it.
#[test]
fn the_theme_is_a_snapshot_of_its_own_design_system() {
    // Not `kit()`: the helper hands the widget a theme of its own, which is
    assert_ne!(
        CupertinoEmptyState::new().theme().id(),
        floom_core_theme::global_theme::theme_signal().id(),
        "v1's Cupertino empty state took a snapshot, not the app-wide signal"
    );
}
