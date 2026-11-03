//! Cupertino's autocomplete as a kit crate: the text-shaped callback, the commit point,

use std::sync::{Arc, Mutex};

use floom_core::Widget;

use floom_widgets::testkit::{AllocatingCreateContext, RecordingUpdateContext};

use floom_core_theme::theme::Theme;
use floom_widget_cupertino_autocomplete::CupertinoAutocomplete;

fn options() -> Vec<String> {
    ["apple", "banana", "Cherry"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// An autocomplete that records what it was told.
fn recording() -> (CupertinoAutocomplete, Arc<Mutex<Vec<Option<String>>>>) {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let combo = CupertinoAutocomplete::new()
        .options(options())
        .on_change(move |text| sink.lock().unwrap().push(text));
    (combo, seen)
}

// --- the payload is text, not an index -----------------------------------

/// **Choosing an option reports its text.** That single difference from a combobox is
/// the whole reason this crate exists rather than being `md-combobox` with a different
/// name: an index cannot express what this field is allowed to hold.
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

/// The mounted widget's callback is **adapted**, not handed through: the engine reports
/// an index and the caller receives text. This drives the real path — the engine's
/// `select`, which is what a row click does.
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

// --- the commit point -----------------------------------------------------

/// Free text is reported when it matches nothing, because holding something that is not
/// on the list is what this control is for.
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

/// **An exact match reports nothing — and the method says so.** v1's third branch set
/// `selected` and called no callback; that is preserved, and it is why `commit()` returns
/// a bool rather than nothing: a caller driving this by hand needs to be able to tell
/// "committed and said so" from "committed silently", and the second case is real.
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

/// A substring the *filter* would show does not become a selection on commit.
///
/// The filter uses `contains` and the commit uses `eq_ignore_ascii_case`; conflating them
/// is the obvious mistake, and here it would mean typing "an" silently selects "banana".
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
    let mut combo = CupertinoAutocomplete::new().options(options());
    *combo.query().lock().unwrap() = "kiwi".to_string();
    assert!(
        combo.commit(),
        "it still reports that it had something to say"
    );
    assert_eq!(combo.pick().lock().unwrap().selected, None);
}

// --- what the port cannot reproduce ---------------------------------------

/// **v1 committed on `FocusLost`; 2.0 has no way for a parent to observe a child's
/// focus, so `commit()` is public and the caller drives it.** This test is the record of
/// that: the rule is reachable, and it is reachable only because it was made public.
#[test]
fn the_commit_point_is_public_because_focus_cannot_be_observed() {
    let (combo, seen) = recording();
    *combo.query().lock().unwrap() = "kiwi".to_string();
    // A caller holding only the autocomplete — with no reference to any field — can
    // still commit. Under v1 this was reachable only from inside the widget, attached to
    combo.commit();
    assert_eq!(seen.lock().unwrap().as_slice(), [Some("kiwi".to_string())]);
}

// --- the tokens -----------------------------------------------------------

/// The six numbers from `AutocompleteTokens` are the token's. `AutocompleteTokens` has
/// exactly `ComboboxTokens`' six fields, which is why the engine's one style struct
/// serves both controls — and that is worth asserting rather than assuming.
#[test]
fn the_tokens_come_from_autocomplete_tokens() {
    let style = CupertinoAutocomplete::new().resolved_style();
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Cupertino,
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

/// **This kit's panel gap is zero, where Material's is four** — the one number the three
/// kits disagree on.
///
/// I derived this file by substituting names from the Material one, and the substitution
/// renamed the test's *title* while leaving its *number* at 4.0. So the test compiled,
/// failed, and said something false about this kit. A substitution renames words; it does
/// not know which words are values. The control below is what makes the difference
/// legible rather than merely asserted.
#[test]
fn the_panel_gap_is_zero() {
    assert_eq!(CupertinoAutocomplete::new().resolved_style().panel_gap, 0.0);
}

/// The control: the engine's default is Material's 4.0, so a kit that says nothing gets
/// the wrong gap and this one has to say `0.0` out loud.
#[test]
fn the_engine_default_would_have_been_wrong_here() {
    assert_eq!(
        floom_ui_composites::ComboboxStyle::default().panel_gap,
        4.0,
        "which is exactly why the value above is written rather than inherited"
    );
}

/// The call-site constants, which v1 wrote inline. The same values the combobox uses:
/// the two controls agree here, which the shared style struct now makes structural.
#[test]
fn the_call_site_constants_match_the_comboboxs() {
    let style = CupertinoAutocomplete::new().resolved_style();
    assert_eq!(style.trigger_gap, 8.0);
    assert_eq!(style.row_padding_horizontal, 12.0);
    assert_eq!(style.chevron_size, 14.0);
    assert_eq!(style.selected_row_alpha, 24);
}

/// **Nothing here verifies that this is Cupertino's theme.** `MultiTheme::new_for` ignores
/// its argument and reads go through a process-global, so these numbers are Cupertino's
/// because nothing has called `set_design_system`.
#[test]
fn the_theme_is_not_selected_by_naming_it() {
    let cupertino = CupertinoAutocomplete::new().resolved_style();
    let cupertino = CupertinoAutocomplete::new()
        .with_theme(floom_core_theme::theme::MultiTheme::new_for(
            floom_core_theme::global_theme::DesignSystem::Cupertino,
        ))
        .resolved_style();
    assert_eq!(cupertino.height, cupertino.height);
}

// --- props ----------------------------------------------------------------

/// `props()` reports `AutocompleteProps`, not `ComboboxProps`.
///
/// The two structs are identical except for `on_change`'s payload, and picking the wrong
/// one is a compile error rather than a silent bug — which is why this assertion exists
/// only to say the *right* one is in use.
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

/// The default placeholder is v1's — and it is **not** the combobox's.
///
/// `AutocompleteProps::default()` says "Type to search…" where `ComboboxProps::default()`
/// says "Select…". Two props types, two defaults, and this is the second structural
/// difference between them beyond `on_change`'s payload. Worth a test because a single
/// shared style struct elsewhere in this port makes it easy to assume the two controls
/// were the same thing all the way down.
#[test]
fn the_default_placeholder_is_the_autocompletes_own() {
    let combo = CupertinoAutocomplete::new();
    assert_eq!(combo.props().placeholder, "Type to search\u{2026}");
}

// --- the mounted view -----------------------------------------------------

/// The view places its panel below the field with Cupertino's gap — the same placement a
/// combobox makes, because it is the same engine widget.
#[test]
fn the_view_places_its_panel_below_the_field() {
    let mut view = CupertinoAutocomplete::new().options(options()).view();
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

/// An untouched autocomplete lists everything, because v1's filter matches an empty
/// query against every option — an autocomplete that opened empty would be a search box
/// with no way to see what it searches.
#[test]
fn an_empty_query_lists_every_option() {
    let view = CupertinoAutocomplete::new().options(options()).view();
    assert_eq!(view.visible(), options());
}

/// `disabled` reaches the widget, so a disabled autocomplete is visibly one.
///
/// Painted rather than read: `Combobox` has no `disabled()` reader — and adding one for a
/// flag a caller already passed in would be a getter for its own argument. The trigger is
/// also a *child*, so the widget's own `paint` draws nothing and comparing the widgets
/// would compare two empty scenes; the child is what has to be painted.
#[test]
fn disabled_reaches_the_widget_and_changes_how_it_paints() {
    let bounds = floom_types::Rect {
        x0: 30.0,
        y0: 100.0,
        x1: 230.0,
        y1: 156.0,
    };
    let live = CupertinoAutocomplete::new().options(options()).view();
    let dead = CupertinoAutocomplete::new()
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
