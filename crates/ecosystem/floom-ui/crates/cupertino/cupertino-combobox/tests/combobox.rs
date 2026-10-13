//! Cupertino's combobox as a kit crate: the token translation, the props round trip,
//! and the two things this kit decides differently from Material.
//!
//! The 12 tests it shares with `md-combobox` are not repeated here on purpose — they
//! are the engine's, tested in `floom-widgets/tests/combobox*.rs` and in the Material
//! crate. What is here is what is *this kit's*.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::Widget;
use floom_types::{OverlayKind, Rect};

use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};
use floom_widgets::{PopupPlacement};
use floom_ui_composites::{ComboboxStyle};

use floom_core_theme::theme::Theme;
use floom_widget_cupertino_combobox::CupertinoCombobox;

/// The field's box, so the panel's placement can be asserted against a real anchor.
const FIELD: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 156.0,
};

fn options() -> Vec<String> {
    ["apple", "banana", "Cherry"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn style() -> ComboboxStyle {
    CupertinoCombobox::new().resolved_style()
}

// --- the one number that differs from Material ---------------------------

/// **This kit wrote no panel gap, so its panel sits flush against the field.**
///
/// v1's Cupertino combobox positioned the panel with `.absolute().inset_top(origin.y +
/// trigger_height)` and no `margin_top`, while Material's wrote `.margin_top(4.0)`. That
/// is a disagreement between two design systems, so it is a parameter rather than a
/// reconciliation — and this assertion is what stops a later "just use the default"
/// edit from quietly erasing it.
#[test]
fn the_panel_sits_flush_against_the_field() {
    assert_eq!(style().panel_gap, 0.0);
}

/// The control for the test above: **the engine's default is Material's 4.0**, so a kit
/// that says nothing gets the wrong gap. This is why the parameter exists at all.
#[test]
fn the_engine_default_is_materials_four() {
    assert_eq!(
        ComboboxStyle::default().panel_gap,
        4.0,
        "Material is the only kit that wrote a number, so it is the default — and any \
         other kit must say `0.0` out loud rather than inherit it"
    );
}

/// And the two really place the panel differently, four pixels apart.
#[test]
fn the_two_kits_gap_the_panel_differently() {
    let mine = floom_widgets::combobox::panel_placement(style().panel_gap, 320.0);
    let materials = floom_widgets::combobox::panel_placement(4.0, 320.0);
    assert_eq!(
        floom_widgets::combobox::panel_rect(mine, FIELD).y0,
        FIELD.y1,
        "flush, for this kit"
    );
    assert_eq!(
        floom_widgets::combobox::panel_rect(materials, FIELD).y0,
        FIELD.y1 + 4.0,
        "and four below, for Material"
    );
}

// --- the icon, which is this kit's own -----------------------------------

/// **The chevron comes from this kit's own table.** That is not tidiness: `expand_more`
/// is a different glyph in each design system's icon set, so a lookup that reached for
/// the Material table would draw a Material chevron inside a Cupertino control and
/// nothing would fail.
///
/// It deliberately does **not** compare against `md-icon`, which is what I first wrote.
/// A Cupertino crate depending on Material's icon crate to prove a point about
/// Cupertino's icons is the wrong dependency: it would put Material in this crate's
/// graph for a test, and the next person to read it would reasonably assume Cupertino
/// *needs* Material's icons. The claim is made from this kit's own table instead.
#[test]
fn the_chevron_comes_from_this_kits_own_table() {
    let path = floom_widget_cupertino_icon::cupertino_icon_path("expand_more");
    assert!(!path.is_empty(), "this kit has an `expand_more` of its own");
    assert_ne!(
        path,
        floom_widget_cupertino_icon::cupertino_icon_path("definitely_not_an_icon"),
        "and it is a real glyph rather than the set's fallback circle"
    );
}

/// The engine widget the lookup hands over is built from *that* path, so the table and
/// the mounted chevron cannot drift apart.
#[test]
fn the_engine_icon_uses_the_same_path_the_table_holds() {
    use floom_widgets::VectorIcon;
    let from_table = floom_widget_cupertino_icon::cupertino_icon("expand_more");
    let reparsed = VectorIcon::from_path_data(
        floom_widget_cupertino_icon::cupertino_icon_path("expand_more"),
        floom_widget_cupertino_icon::VIEW_BOX,
    );
    assert_eq!(
        format!("{:?}", from_table.path()),
        format!("{:?}", reparsed.path()),
        "one table, one route into a VectorIcon"
    );
}

// --- the theme, which no test verifies -----------------------------------

/// **A kit's own design system is not selected by naming it.** `MultiTheme::new_for`
/// ignores its argument and reads go through a process-global, so these numbers are
/// Material's because nothing has called `set_design_system` — not because this crate
/// says Cupertino.
#[test]
fn the_theme_is_not_selected_by_naming_it() {
    let named_cupertino = style();
    let named_fluent = CupertinoCombobox::new()
        .with_theme(floom_core_theme::theme::MultiTheme::new_for(
            floom_core_theme::global_theme::DesignSystem::Fluent,
        ))
        .resolved_style();
    assert_eq!(
        named_cupertino.height, named_fluent.height,
        "two different named design systems resolve the same metrics"
    );
}

// --- the tokens -----------------------------------------------------------

/// The five numbers from `ComboboxTokens` are the token's, not a default's. The failure
/// this pins is a kit silently resolving to `ComboboxStyle::default()` because
/// `theme.component(..)` did not match — which is `unreachable!()` at runtime, so the
/// real symptom is a combobox in the wrong metrics rather than a failure.
#[test]
fn the_tokens_come_from_combobox_tokens() {
    let style = style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Cupertino,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Combobox(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Combobox)
    else {
        panic!("ComponentId::Combobox must resolve to ComboboxTokens");
    };
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.padding_horizontal, tokens.padding_horizontal);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(style.item_height, tokens.item_height);
    // v1 declared this and never read it; the host needs it now.
    assert_eq!(style.panel_max_height, tokens.panel_max_height);
}

/// The numbers v1 wrote at the call site. These are the same values Material wrote —
/// the two kits agree here — which is worth a test, because "the kits disagree about
/// everything" is the impression the 26-line diff gives and it is not true.
#[test]
fn the_call_site_constants_match_v1s() {
    let style = style();
    assert_eq!(style.trigger_gap, 8.0);
    assert_eq!(style.row_padding_horizontal, 12.0);
    assert_eq!(style.chevron_size, 14.0);
    assert_eq!(style.border_width, 1.0);
    assert_eq!(style.disabled_text_alpha, 38);
    assert_eq!(style.disabled_chevron_alpha, 97);
    assert_eq!(style.selected_row_alpha, 24);
    assert_eq!(style.panel_shadow_alpha, 20);
}

/// The field's border follows v1's three cases with disabled winning over open.
#[test]
fn the_border_follows_v1s_order() {
    let style = style();
    assert_eq!(style.border_color(false, false), style.outline);
    assert_eq!(style.border_color(false, true), style.primary);
    assert_ne!(
        style.border_color(true, true),
        style.primary,
        "disabled wins over open — reversed, this is a combobox that looks live while \
         it is not"
    );
}

// --- props ----------------------------------------------------------------

/// `props()` keeps reporting what v1 reported, including the callback's presence —
/// which is why the crate stores it twice.
#[test]
fn props_report_what_the_builder_set() {
    let combo = CupertinoCombobox::new()
        .options(options())
        .selected(Some(1))
        .placeholder("Pick fruit")
        .disabled(true)
        .on_change(|_| {});
    let props = combo.props();
    assert_eq!(props.options, options());
    assert_eq!(props.selected, Some(1));
    assert_eq!(props.placeholder, "Pick fruit");
    assert!(props.disabled);
    assert!(props.on_change.is_some());
}

// --- the mounted view -----------------------------------------------------

/// The viewed combobox asks for a panel **flush below** the field — this kit's own
/// gap, not the engine's default.
#[test]
fn the_viewed_combobox_places_its_panel_flush_below_the_field() {
    let mut combo = CupertinoCombobox::new().options(options()).view();
    let id = combo.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(FIELD);

    combo.set_open(true);
    combo.update(&mut cx, id);

    match cx.opened_overlays()[0] {
        floom_widgets::testkit::OverlayCall::Open {
            bounds, kind, modal, ..
        } => {
            assert_eq!(*kind, OverlayKind::Popup);
            assert!(!modal, "a dropdown is dismissed, not blocking");
            assert_eq!(bounds.y0, FIELD.y1, "flush — no gap, as v1 had it");
            assert_eq!(bounds.x0, FIELD.x0);
            assert_eq!(bounds.x1, FIELD.x1);
        }
        _ => panic!("expected an open, got a close"),
    }
}

/// The style drives the placement the request carries, so a kit's gap reaches the
/// request rather than only living in a struct nobody reads.
#[test]
fn the_style_drives_the_placement_the_request_carries() {
    let style = style();
    assert_eq!(
        style.placement(),
        PopupPlacement::Below {
            gap: 0.0,
            height: f64::from(style.panel_max_height),
        }
    );
}

/// The options reach the widget, so an untouched combobox lists them all.
#[test]
fn the_options_reach_the_widget() {
    let combo = CupertinoCombobox::new().options(options()).view();
    assert_eq!(combo.visible(), options());
}

/// And the callback fires when an option is chosen, through the `Send + Sync` handle
/// rather than the v1 `Box<dyn Fn(..)>`.
#[test]
fn the_callback_fires_when_an_option_is_chosen() {
    let fired = Arc::new(AtomicUsize::new(0));
    let counter = fired.clone();
    let mut combo = CupertinoCombobox::new()
        .options(options())
        .on_change(move |index| {
            assert_eq!(index, Some(2));
            counter.fetch_add(1, Ordering::SeqCst);
        })
        .view();
    combo.select(2);
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert_eq!(combo.selected(), Some(2));
}

/// `disabled` reaches the widget, and a disabled combobox paints differently.
#[test]
fn disabled_reaches_the_widget_and_changes_how_it_paints() {
    let live = CupertinoCombobox::new().options(options()).view();
    let dead = CupertinoCombobox::new()
        .options(options())
        .disabled(true)
        .view();
    // The trigger is a child, so the widget's own `paint` draws nothing — comparing
    // the widgets would compare two empty scenes.
    let live_trigger = live.children().remove(0).build();
    let dead_trigger = dead.children().remove(0).build();
    let live_scene = floom_widgets::testkit::paint_snapshot(
        &*live_trigger,
        floom_core::ElementId::new(1, 1),
        FIELD,
    );
    let dead_scene = floom_widgets::testkit::paint_snapshot(
        &*dead_trigger,
        floom_core::ElementId::new(1, 1),
        FIELD,
    );
    assert_ne!(live_scene, dead_scene);
}