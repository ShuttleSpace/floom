//! Material's select as a kit crate: the token translation, and the numbers
//! this kit wrote that the other two wrote differently.
//!
//! Material is the reference kit here, so its values are also the engine
//! defaults for several fields — which is exactly why the Cupertino and Fluent
//! tests use those defaults as their controls. Where a default *is* Material's
//! number, this file says so instead of pretending the assertion is a
//! discovery.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Rect, Widget};
use floom_widgets::testkit::{AllocatingCreateContext, SceneRecorder};
use floom_ui_composites::{Select, SelectStyle};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_md_select::MdSelect;

const TRIGGER: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn style() -> SelectStyle {
    MdSelect::new().resolved_style()
}

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Material)
}

fn alpha_of(color: floom_types::Color) -> u8 {
    color.a
}

// --- Material's answers ---------------------------------------------------

/// A hovered row is `on_surface`@8. Cupertino's is `primary`@12, so this
/// assertion is on the *rgb* too, not just the alpha.
#[test]
fn the_hover_row_is_on_surface_at_eight() {
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style().hover_row.r, colors.on_surface.r);
    assert_eq!(alpha_of(style().hover_row), 8);
    assert_ne!(style().hover_row.r, colors.primary.r);
}

/// Material's panel is the only one with a border: `outline`@120. This value is
/// also `SelectStyle::default()`'s, which is what lets the other two kits use
/// "the default has a border" as the control for their own missing one.
#[test]
fn the_panel_border_is_outline_at_120() {
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style().panel_border.r, colors.outline.r);
    assert_eq!(alpha_of(style().panel_border), 120);
    assert_eq!(
        alpha_of(SelectStyle::default().panel_border),
        120,
        "the engine default is Material's number, by construction"
    );
}

/// Black@20, blurred 8. Cupertino wrote black@24 and Fluent black@20 blurred 12.
#[test]
fn the_panel_shadow_is_black_at_twenty_blurred_eight() {
    assert_eq!(alpha_of(style().panel_shadow), 20);
    assert_eq!(style().panel_shadow_blur, 8.0);
    assert_eq!(style().panel_shadow_spread, 1.0);
}

/// Placeholder and chevron are both the full `on_surface_variant` — undimmed,
/// unlike Cupertino's `on_surface`@140 and Fluent's `@160`.
#[test]
fn the_placeholder_and_the_chevron_are_the_full_on_surface_variant() {
    let theme = theme();
    let colors = theme.colors();
    let style = style();
    assert_eq!(style.placeholder, style.chevron);
    assert_eq!(style.placeholder.r, colors.on_surface_variant.r);
    assert_eq!(alpha_of(style.placeholder), 255);
}

/// A disabled Material trigger keeps `surface` and dims its border to
/// `outline`@97 — so the disabled state is a *border* change here, where
/// Cupertino's is a fill change (`on_surface`@4).
#[test]
fn a_disabled_trigger_keeps_the_surface_and_dims_the_border() {
    let theme = theme();
    let colors = theme.colors();
    assert_eq!(style().disabled_background, style().surface);
    assert_eq!(style().disabled_border.r, colors.outline.r);
    assert_eq!(alpha_of(style().disabled_border), 97);
    assert_eq!(alpha_of(style().disabled_text), 97);
    assert_eq!(alpha_of(style().disabled_chevron), 97);
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

/// v1's call-site constants, all three of which the engine now reads.
#[test]
fn the_call_site_constants_survived() {
    let style = style();
    assert_eq!(style.trigger_gap, 8.0);
    assert_eq!(style.row_padding_horizontal, 12.0);
    assert_eq!(style.panel_gap, 0.0);
    assert_eq!(style.panel_padding, 4.0);
    assert_eq!(style.border_width, 1.0);
}

/// See the same test in the Cupertino kit for why this is here: the theme
/// crate's `activate` is a placeholder, so a kit's identity lives in its
/// constants and not in the tokens it reads.
#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let material_theme = theme();
    let cupertino_theme = MultiTheme::new_for(DesignSystem::Cupertino);
    let material = material_theme.colors();
    let cupertino = cupertino_theme.colors();
    assert_eq!(cupertino.surface, material.surface);
    assert_eq!(cupertino.on_surface_variant, material.on_surface_variant);
}

// --- mounting -------------------------------------------------------------

#[test]
fn mounting_carries_the_options_the_selection_and_the_placeholder() {
    let select = MdSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .placeholder("Pick one")
        .selected(Some(1))
        .view();

    assert_eq!(select.options(), ["Red", "Green"]);
    assert_eq!(select.selected_label(), "Green");
    assert_eq!(select.placeholder_text(), "Pick one");

    let unselected = MdSelect::new()
        .options(vec!["Red".into()])
        .placeholder("Pick one")
        .view();
    assert_eq!(unselected.selected_label(), "Pick one");
}

#[test]
fn mounting_uses_this_kits_height() {
    let style = style();
    let select = MdSelect::new().options(vec!["Red".into()]).view();
    let request = select.layout_request().expect("a select asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.height
    );
}

#[test]
fn the_mounted_trigger_paints_a_chevron_in_the_kits_colour() {
    let select = MdSelect::new().options(vec!["Red".into()]).view();
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
        "Material's chevron, Material's colour:\n{}",
        recorder.snapshot()
    );
}

#[test]
fn choosing_reports_the_index_as_an_option() {
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = seen.clone();
    let mut select: Select = MdSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .on_change(move |index| {
            sink.store(index.map(|i| i + 1).unwrap_or(0), Ordering::SeqCst);
        })
        .view();

    assert!(select.select(1), "the index is in range");
    assert_eq!(seen.load(Ordering::SeqCst), 2, "`Some(1)` became 2");
}

#[test]
fn an_empty_option_list_still_mounts() {
    let select: Select = MdSelect::new().view();
    assert!(select.options().is_empty());
    assert_eq!(
        select.selected_label(),
        floom_core_theme::props::select::SelectProps::default().placeholder
    );
}
