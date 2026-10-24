//! Fluent's select as a kit crate: the token translation, the props round trip,
//! and the three numbers this kit answers differently from the other two.
//!
//! `select` is the one component in the port where the three kits are not the
//! same component. Fluent is the third answer: its hover band agrees with
//! Material's, its missing panel border agrees with Cupertino's, and its chevron
//! is a third value of its own.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Rect, Widget};
use floom_ui_composites::{Select, SelectStyle};
use floom_widgets::testkit::{AllocatingCreateContext, SceneRecorder};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_fluent_select::FluentSelect;

const TRIGGER: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn style() -> SelectStyle {
    FluentSelect::new().resolved_style()
}

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Fluent)
}

fn alpha_of(color: floom_types::Color) -> u8 {
    color.a
}

// --- the three answers ----------------------------------------------------

/// A hovered row is `on_surface`@8 — Material's answer, and explicitly *not*
/// Cupertino's `primary`@12.
#[test]
fn the_hover_row_is_on_surface_at_eight() {
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style().hover_row.r, colors.on_surface.r);
    assert_eq!(alpha_of(style().hover_row), 8);
    assert_ne!(
        style().hover_row.r,
        colors.primary.r,
        "control: Cupertino's band is `primary`, so this is not that"
    );
}

/// The panel has no border, as Cupertino's; Material's is `outline`@120.
#[test]
fn the_panel_has_no_border() {
    assert_eq!(alpha_of(style().panel_border), 0);
    assert_ne!(
        alpha_of(SelectStyle::default().panel_border),
        0,
        "control: the default carries Material's border"
    );
}

/// The chevron is `on_surface`@160: Cupertino's is 140 and Material's is the
/// full `on_surface_variant`.
#[test]
fn the_chevron_is_on_surface_at_160() {
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style().chevron.r, colors.on_surface.r);
    assert_eq!(alpha_of(style().chevron), 160);
}

/// v1's Fluent trigger painted its label in `on_surface` and never distinguished
/// a placeholder, so the two are the same colour — unlike the other two kits,
/// where the placeholder is dimmer than a chosen option.
#[test]
fn the_placeholder_is_the_same_colour_as_a_chosen_label() {
    let style = style();
    assert_eq!(style.placeholder, style.on_surface);
}

/// The shadow is black@20, which Material shares; Cupertino wrote 24.
#[test]
fn the_panel_shadow_is_black_at_twenty() {
    assert_eq!(alpha_of(style().panel_shadow), 20);
    assert_eq!(style().panel_shadow_blur, 12.0);
}

// --- what is shared -------------------------------------------------------

#[test]
fn the_geometry_is_the_select_tokens() {
    let theme = theme();
    let tokens = match theme.component(floom_core_theme::component_tokens::ComponentId::Select) {
        floom_core_theme::component_tokens::ComponentTokens::Select(t) => t,
        _ => unreachable!(),
    };
    let style = style();
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.padding_horizontal, tokens.padding_horizontal);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(style.item_height, tokens.item_height);
    assert_eq!(style.panel_max_height, tokens.panel_max_height);
    assert_eq!(style.icon_size, tokens.icon_size);
}

/// v1's Fluent trigger kept `surface` when disabled — there is no disabled
/// background to translate, so it is the same colour. Cupertino's is
/// `on_surface`@4, which is why this field exists at all.
#[test]
fn a_disabled_trigger_keeps_the_surface() {
    assert_eq!(style().disabled_background, style().surface);
    assert_eq!(alpha_of(style().disabled_border), 38);
}

// --- mounting -------------------------------------------------------------

#[test]
fn mounting_carries_the_options_the_selection_and_the_placeholder() {
    let select = FluentSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .placeholder("Pick one")
        .selected(Some(1))
        .view();

    assert_eq!(select.options(), ["Red", "Green"]);
    assert_eq!(select.selected_label(), "Green");
    assert_eq!(select.placeholder_text(), "Pick one");
}

#[test]
fn mounting_uses_this_kits_height() {
    let style = style();
    let select = FluentSelect::new().options(vec!["Red".into()]).view();
    let request = select.layout_request().expect("a select asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.height
    );
}

#[test]
fn the_mounted_trigger_paints_a_chevron_in_the_kits_colour() {
    let select = FluentSelect::new().options(vec!["Red".into()]).view();
    let mut create = AllocatingCreateContext::new();
    let id = select.create(&mut create);

    let mut recorder = SceneRecorder::new(TRIGGER);
    select.paint(&mut recorder, id);
    recorder.finish();

    let fills: Vec<_> = recorder
        .ops()
        .iter()
        .filter_map(|op| match op {
            floom_widgets::testkit::PaintOp::Path { fill, .. } => *fill,
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        vec![style().chevron],
        "the chevron is Fluent's icon, tinted Fluent's colour:\n{}",
        recorder.snapshot()
    );
}

#[test]
fn choosing_reports_the_index() {
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = seen.clone();
    let mut select: Select = FluentSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .on_change(move |index| sink.store(index + 1, Ordering::SeqCst))
        .view();

    assert!(select.select(1), "the index is in range");
    assert_eq!(seen.load(Ordering::SeqCst), 2, "index 1 became 2");

    assert!(select.select(1), "re-picking reports too");
    assert_eq!(seen.load(Ordering::SeqCst), 2, "same index, reported again");
}

/// The callback carries a plain index, not an `Option`.
///
/// v1's select reported `Option<usize>` because its `RwSignal` could be cleared;
/// the engine's cannot -- `Select::choose` writes `Some(index)` on every
/// successful pick, so there is no "unchoose" state to carry.
#[test]
fn the_callback_carries_a_plain_index() {
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = seen.clone();
    let mut select: Select = FluentSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .on_change(move |index| sink.store(index + 1, Ordering::SeqCst))
        .view();

    select.select(0);
    assert_eq!(seen.load(Ordering::SeqCst), 1, "index 0 became 1, never 0");
}

/// The same placeholder fallback the other two kits get, from
/// `SelectProps::default()` rather than from this crate.
#[test]
fn an_empty_option_list_still_mounts() {
    let select: Select = FluentSelect::new().view();
    assert!(select.options().is_empty());
    assert_eq!(
        select.selected_label(),
        floom_core_theme::props::select::SelectProps::default().placeholder
    );
}
