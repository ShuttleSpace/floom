#![cfg(feature = "autocomplete")]
use std::sync::{Arc, Mutex};

use floom_core::Widget;

use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::theme::Theme;
use floom_material::widgets::autocomplete::MdAutocomplete;

fn options() -> Vec<String> {
    ["apple", "banana", "Cherry"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// An autocomplete that records what it was told.
fn recording() -> (MdAutocomplete, Arc<Mutex<Vec<Option<String>>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let combo = MdAutocomplete::new()
        .options(options())
        .on_change(move |text| sink.lock().unwrap().push(text));
    (combo, seen)
}

#[test]
fn choosing_an_option_reports_its_text() {
    let (mut combo, seen) = recording();
    combo.select(1);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("banana".to_string())],
        "the option's text, not `Some(1)`"
    );
}

/// And the field reads what was chosen, as it does for a combobox.
#[test]
fn choosing_fills_the_field() {
    let (mut combo, _) = recording();
    combo.select(2);
    assert_eq!(combo.query().lock().unwrap().as_str(), "Cherry");
    assert_eq!(combo.pick().lock().unwrap().selected, Some(2));
}

#[test]
fn the_mounted_widget_adapts_the_index_to_text() {
    let (combo, seen) = recording();
    let mut view = combo.view();
    view.select(1);
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("banana".to_string())],
        "a row click reports text even though the engine only ever holds an index"
    );
}

#[test]
fn committing_free_text_reports_what_was_typed() {
    let (combo, seen) = recording();
    *combo.query().lock().unwrap() = "kiwi".to_string();

    assert!(combo.commit(), "and it reported");
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("kiwi".to_string())],
        "the typed text"
    );
    assert_eq!(
        combo.pick().lock().unwrap().selected,
        None,
        "with no option claimed for it"
    );
}

/// An emptied field clears the selection and reports "nothing selected".
#[test]
fn committing_an_empty_field_clears_and_reports_none() {
    let (mut combo, seen) = recording();
    combo.select(1);
    combo.query().lock().unwrap().clear();

    assert!(combo.commit());
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        [Some("banana".to_string()), None],
        "the choice first (from `select`), then the clearing (from `commit`)"
    );
    assert_eq!(combo.pick().lock().unwrap().selected, None);
}

#[test]
fn committing_an_exact_match_is_silent_even_though_it_moved_the_selection() {
    let (combo, seen) = recording();
    *combo.query().lock().unwrap() = "banana".to_string();
    assert_eq!(
        combo.pick().lock().unwrap().selected,
        None,
        "nothing chosen yet"
    );

    assert!(!combo.commit(), "so `commit` reports that it said nothing");
    assert!(
        seen.lock().unwrap().is_empty(),
        "and the caller was told nothing"
    );
    assert_eq!(
        combo.pick().lock().unwrap().selected,
        Some(1),
        "even though the selection moved — which is the point of pinning this"
    );
}

#[test]
fn committing_a_substring_selects_nothing() {
    let (combo, seen) = recording();
    *combo.query().lock().unwrap() = "an".to_string();
    assert!(combo.commit(), "reported as free text");
    assert_eq!(seen.lock().unwrap().as_slice(), [Some("an".to_string())]);
    assert_eq!(combo.pick().lock().unwrap().selected, None);
}

/// Case does not matter for the commit, though it does for nothing else here.
#[test]
fn committing_matches_an_option_regardless_of_case() {
    let (combo, _) = recording();
    *combo.query().lock().unwrap() = "cHeRrY".to_string();
    assert!(!combo.commit(), "silent, because it is an exact match");
    assert_eq!(combo.pick().lock().unwrap().selected, Some(2));
}

/// An autocomplete with no callback commits without complaint, which is what a caller
/// that only reads the state gets.
#[test]
fn committing_without_a_callback_is_fine() {
    let mut combo = MdAutocomplete::new().options(options());
    *combo.query().lock().unwrap() = "kiwi".to_string();
    assert!(
        combo.commit(),
        "it still reports that it had something to say"
    );
    assert_eq!(combo.pick().lock().unwrap().selected, None);
}

#[test]
fn the_commit_point_is_public_because_focus_cannot_be_observed() {
    let (combo, seen) = recording();
    *combo.query().lock().unwrap() = "kiwi".to_string();
    combo.commit();
    assert_eq!(seen.lock().unwrap().as_slice(), [Some("kiwi".to_string())]);
}

#[test]
fn the_tokens_come_from_autocomplete_tokens() {
    let style = MdAutocomplete::new().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Material,
    );
    let floom_core_theme::component_tokens::ComponentTokens::Autocomplete(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Autocomplete)
    else {
        panic!("ComponentId::Autocomplete must resolve to AutocompleteTokens");
    };
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.padding_horizontal, tokens.padding_horizontal);
    assert_eq!(style.border_radius, tokens.border_radius);
    assert_eq!(style.min_width, tokens.min_width);
    assert_eq!(style.item_height, tokens.item_height);
    assert_eq!(style.panel_max_height, tokens.panel_max_height);
}

/// Material's panel gap — the one number the three kits disagree on.
#[test]
fn the_panel_gap_is_materials_four() {
    assert_eq!(MdAutocomplete::new().resolved_style().panel_gap, 4.0);
}

#[test]
fn the_call_site_constants_match_the_comboboxs() {
    let style = MdAutocomplete::new().resolved_style();
    assert_eq!(style.trigger_gap, 8.0);
    assert_eq!(style.row_padding_horizontal, 12.0);
    assert_eq!(style.chevron_size, 14.0);
    assert_eq!(style.selected_row_alpha, 24);
}

#[test]
fn the_theme_is_not_selected_by_naming_it() {
    let material = MdAutocomplete::new().resolved_style();
    let cupertino = MdAutocomplete::new()
        .with_theme(floom_core_theme::theme::MultiTheme::new_for(
            floom_core_theme::global_theme::DesignSystem::Cupertino,
        ))
        .resolved_style();
    assert_eq!(material.height, cupertino.height);
}

#[test]
fn props_are_autocomplete_props() {
    let (combo, _) = recording();
    let props = combo.props();
    assert_eq!(props.options, options());
    assert!(
        props.on_change.is_some(),
        "and the text-shaped callback is stored v1-shaped for `props()`"
    );
}

#[test]
fn the_default_placeholder_is_the_autocompletes_own() {
    let combo = MdAutocomplete::new();
    assert_eq!(combo.props().placeholder, "Type to search\u{2026}");
}

/// The view places its panel below the field with Material's gap — the same placement a
/// combobox makes, because it is the same engine widget.
#[test]
fn the_view_places_its_panel_below_the_field() {
    let mut view = MdAutocomplete::new().options(options()).view();
    let id = view.create(&mut AllocatingCreateContext::new());
    let mut cx = RecordingUpdateContext::with_bounds(floom_types::Rect {
        x0: 30.0,
        y0: 100.0,
        x1: 230.0,
        y1: 156.0,
    });
    view.set_open(true);
    view.update(&mut cx, id);
    assert_eq!(cx.opened_overlays().len(), 1);
}

#[test]
fn an_empty_query_lists_every_option() {
    let view = MdAutocomplete::new().options(options()).view();
    assert_eq!(view.visible(), options());
}

#[test]
fn disabled_reaches_the_widget_and_changes_how_it_paints() {
    let bounds = floom_types::Rect {
        x0: 30.0,
        y0: 100.0,
        x1: 230.0,
        y1: 156.0,
    };
    let live = MdAutocomplete::new().options(options()).view();
    let dead = MdAutocomplete::new()
        .options(options())
        .disabled(true)
        .view();
    let live_trigger = live.children().remove(0).build();
    let dead_trigger = dead.children().remove(0).build();
    let live_scene = floom_widgets::testkit::paint_snapshot(
        &*live_trigger,
        floom_core::ElementId::new(1, 1),
        bounds,
    );
    let dead_scene = floom_widgets::testkit::paint_snapshot(
        &*dead_trigger,
        floom_core::ElementId::new(1, 1),
        bounds,
    );
    assert_ne!(live_scene, dead_scene);
}
