//! Cupertino's select as a kit crate: the token translation, the props round
//! trip, and the four numbers this kit decides differently from Material.
//!
//! The behaviour tests are not repeated here on purpose — they are the engine's,
//! in `floom-widgets/tests/controls.rs` and `tests/select_style.rs`. What is here
//! is what is *this kit's*, plus a control for each claim so a "just use the
//! default" edit cannot quietly pass.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Rect, Widget};
use floom_widgets::testkit::{AllocatingCreateContext, SceneRecorder};
use floom_ui_composites::{Select, SelectStyle};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_cupertino_select::CupertinoSelect;

const TRIGGER: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 40.0,
};

fn style() -> SelectStyle {
    CupertinoSelect::new().resolved_style()
}

fn theme() -> MultiTheme {
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// `MultiTheme::colors()` borrows the theme, so a test that reads several
/// colours holds the theme in a binding rather than taking it per line.
fn colors() -> MultiTheme {
    theme()
}

fn alpha_of(color: floom_types::Color) -> u8 {
    color.a
}

// --- the four numbers that differ -----------------------------------------

/// Cupertino tints a hovered row with `primary`@12. Material and Fluent use
/// `on_surface`@8 — a different *source* colour, which is why `SelectStyle`
/// carries resolved colours rather than an alpha over one theme colour.
#[test]
fn the_hover_row_is_primary_at_twelve() {
    let theme = colors();
    let colors = theme.colors();
    assert_eq!(style().hover_row.r, colors.primary.r);
    assert_eq!(style().hover_row.g, colors.primary.g);
    assert_eq!(style().hover_row.b, colors.primary.b);
    assert_eq!(alpha_of(style().hover_row), 12);

    // Control: the engine's default is Material's answer, so saying nothing
    // would give this kit the wrong band.
    let default = SelectStyle::default();
    assert_ne!(
        default.hover_row, style().hover_row,
        "the default is Material's `on_surface`@8"
    );
}

/// v1's Cupertino panel drew no border; Material's drew `outline`@120.
#[test]
fn the_panel_has_no_border() {
    assert_eq!(alpha_of(style().panel_border), 0);
    assert_ne!(
        alpha_of(SelectStyle::default().panel_border),
        0,
        "control: the default carries Material's border"
    );
}

/// Cupertino's shadow is black@24; the other two kits wrote black@20.
#[test]
fn the_panel_shadow_is_black_at_twentyfour() {
    assert_eq!(alpha_of(style().panel_shadow), 24);
    assert_eq!(
        alpha_of(SelectStyle::default().panel_shadow),
        20,
        "control: the default is Material's and Fluent's 20"
    );
}

/// And it is blurred 12, not Material's 8.
#[test]
fn the_panel_shadow_is_blurred_twelve() {
    assert_eq!(style().panel_shadow_blur, 12.0);
    assert_eq!(
        SelectStyle::default().panel_shadow_blur,
        8.0,
        "control: the default is Material's 8"
    );
}

/// A disabled Cupertino trigger fills `on_surface`@4. Material's keeps
/// `surface`, so this is a real difference and not an omitted field.
#[test]
fn a_disabled_trigger_is_filled_with_on_surface_at_four() {
    let theme = colors();
    let colors = theme.colors();
    assert_eq!(alpha_of(style().disabled_background), 4);
    assert_eq!(style().disabled_background.r, colors.on_surface.r);
    assert_ne!(
        style().disabled_background, style().surface,
        "a disabled trigger looks filled, not empty"
    );
}

/// The disabled border is `on_surface`@38 here and `outline`@97 in Material.
#[test]
fn the_disabled_border_is_on_surface_at_thirty_eight() {
    let theme = colors();
    let colors = theme.colors();
    assert_eq!(style().disabled_border.r, colors.on_surface.r);
    assert_eq!(alpha_of(style().disabled_border), 38);
}

/// Placeholder and chevron are both `on_surface`@140 in this kit. Fluent wrote
/// 160 for the chevron and Material used `on_surface_variant` for both, so the
/// three kits give three answers to the same question.
#[test]
fn the_placeholder_and_the_chevron_are_on_surface_at_140() {
    let theme = colors();
    let colors = theme.colors();
    let style = style();
    assert_eq!(alpha_of(style.placeholder), 140);
    assert_eq!(alpha_of(style.chevron), 140);
    assert_eq!(style.placeholder, style.chevron);
    assert_eq!(style.placeholder.r, colors.on_surface.r);
}

// --- what is shared -------------------------------------------------------

#[test]
fn the_geometry_is_the_select_tokens() {
    let tokens = match theme().component(floom_core_theme::component_tokens::ComponentId::Select) {
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

/// **The three design systems resolve the same tokens today, so a kit's identity
/// lives in its constants.**
///
/// `MultiTheme::new_for(ds)` calls `activate`, which the theme crate documents
/// as a no-op placeholder kept "for future per-DS overrides" — so Material,
/// Cupertino and Fluent currently hand back one identical set of colours and
/// `SelectTokens`. That is why every test above asserts a *constant* the kit
/// wrote rather than a token value: a token assertion would pass for all three
/// kits no matter what any of them did.
///
/// This test exists so that the day per-DS tokens land, it fails loudly and the
/// kits get revisited — a silent improvement here would otherwise leave three
/// crates that differ only by accident.
#[test]
fn the_three_design_systems_still_resolve_identical_tokens() {
    let material_theme = MultiTheme::new_for(DesignSystem::Material);
    let cupertino_theme = theme();
    let fluent_theme = MultiTheme::new_for(DesignSystem::Fluent);
    let material = material_theme.colors();
    let cupertino = cupertino_theme.colors();
    let fluent = fluent_theme.colors();
    assert_eq!(cupertino.surface, material.surface);
    assert_eq!(cupertino.primary, material.primary);
    assert_eq!(fluent.surface, material.surface);
    assert_eq!(
        style().height,
        CupertinoSelect::new()
            .with_theme(MultiTheme::new_for(DesignSystem::Material))
            .resolved_style()
            .height,
        "the tokens are one set, so the geometry cannot differ either"
    );
}

// --- mounting -------------------------------------------------------------

#[test]
fn mounting_carries_the_options_the_selection_and_the_placeholder() {
    let select = CupertinoSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .placeholder("Pick one")
        .selected(Some(1))
        .view();

    assert_eq!(select.options(), ["Red", "Green"]);
    assert_eq!(select.selected_label(), "Green");
    assert_eq!(select.placeholder_text(), "Pick one");

    let unselected = CupertinoSelect::new()
        .options(vec!["Red".into()])
        .placeholder("Pick one")
        .view();
    assert_eq!(
        unselected.selected_label(),
        "Pick one",
        "an unchosen trigger shows the placeholder"
    );
}

#[test]
fn mounting_uses_this_kits_height() {
    let style = style();
    let select = CupertinoSelect::new().options(vec!["Red".into()]).view();
    let id = {
        let mut create = AllocatingCreateContext::new();
        select.create(&mut create)
    };
    let _ = id;
    let request = select.layout_request().expect("a select asks for a size");
    assert_eq!(
        request.height.expect("a height").px().expect("in px"),
        style.height
    );
}

#[test]
fn the_mounted_trigger_paints_a_chevron_in_the_kits_colour() {
    let select = CupertinoSelect::new().options(vec!["Red".into()]).view();
    let id = {
        let mut create = AllocatingCreateContext::new();
        select.create(&mut create)
    };
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
        "the chevron is this kit's icon, tinted this kit's colour:\n{}",
        recorder.snapshot()
    );
}

#[test]
fn choosing_reports_the_index() {
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = seen.clone();
    let mut select: Select = CupertinoSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .on_change(move |index| sink.store(index + 1, Ordering::SeqCst))
        .view();

    assert!(select.select(1), "the index is in range");
    assert_eq!(seen.load(Ordering::SeqCst), 2, "index 1 became 2");

    // Re-picking the open row still reports, which is what lets a kit treat a
    // choice as a confirmation rather than only as a change. `Slider::set_value`
    // does the opposite and both are pinned on purpose.
    assert!(select.select(1), "re-picking reports too");
    assert_eq!(seen.load(Ordering::SeqCst), 2, "same index, reported again");
}

/// The callback carries a plain index, not an `Option`.
///
/// v1's select reported `Option<usize>` because its `RwSignal` could be cleared;
/// the engine's cannot -- `Select::choose` writes `Some(index)` on every
/// successful pick, so there is no "unchoose" state to carry. This is recorded
/// here rather than quietly assumed, because the `+ 1` encoding the kit uses
/// only works while the value is never `None`.
#[test]
fn the_callback_carries_a_plain_index() {
    let seen = Arc::new(AtomicUsize::new(0));
    let sink = seen.clone();
    let mut select: Select = CupertinoSelect::new()
        .options(vec!["Red".into(), "Green".into()])
        .on_change(move |index| sink.store(index + 1, Ordering::SeqCst))
        .view();

    select.select(0);
    assert_eq!(seen.load(Ordering::SeqCst), 1, "index 0 became 1, never 0");
}

/// A kit that hands the engine an empty option list still mounts; an empty list
/// is a state v1's `list(rows)` handled by rendering nothing. The label it shows
/// is `SelectProps::default().placeholder` — v1 wrote that string at the call
/// site, and the engine's own fallback (an em dash) only appears when a caller
/// builds a `Select` directly with no props behind it.
#[test]
fn an_empty_option_list_still_mounts() {
    let select: Select = CupertinoSelect::new().view();
    assert!(select.options().is_empty());
    assert_eq!(
        select.selected_label(),
        floom_core_theme::props::select::SelectProps::default().placeholder
    );
}
