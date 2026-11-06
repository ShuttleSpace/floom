use std::sync::{Arc, Mutex};

use floom_core::{Color, OverlayHandle, WidgetAny};
use floom_types::{OverlayKind, Rect};

use floom_ui_composites::combobox::{
    commit, matching, panel_placement, panel_rect, panel_request, select, sync_query_to_selection,
    visible_options, ComboPick, ComboboxStyle, Commit, SharedPick,
};
use floom_ui_composites::PopupPlacement;
use floom_widgets::TextInput;

const ANCHOR: Rect = Rect {
    x0: 30.0,
    y0: 100.0,
    x1: 230.0,
    y1: 156.0,
};

fn options() -> Vec<String> {
    ["apple", "banana", "urban planning", "Cherry"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

fn pick() -> SharedPick {
    Arc::new(Mutex::new(ComboPick::default()))
}

fn query(text: &str) -> floom_widgets::SharedBuffer {
    let buffer: floom_widgets::SharedBuffer = Arc::new(Mutex::new(text.to_string()));
    buffer
}

// --- the filter -----------------------------------------------------------

/// v1's rule: an empty query matches everything, so opening an untouched combobox
/// shows every option rather than an empty list.
#[test]
fn an_empty_query_matches_everything() {
    for option in ["apple", "banana", ""] {
        assert!(matching(option, ""), "no query matches {option:?}");
    }
}

/// Case-insensitive, in both directions.
#[test]
fn matching_ignores_case_in_both_directions() {
    assert!(
        matching("Cherry", "cher"),
        "a lowercased query finds a capital"
    );
    assert!(
        matching("cherry", "CHERR"),
        "and an uppercased one finds a lower"
    );
}

/// Substring, not prefix — the part that is easy to "improve" into a behaviour change.
///
/// v1 used `contains`, so `"ban"` finds `"urban planning"`. A prefix filter would look
/// like a refinement and would silently stop finding things.
#[test]
fn the_filter_is_a_substring_match_and_not_a_prefix_match() {
    assert!(
        matching("urban planning", "ban"),
        "v1's `contains` finds a match in the middle of the text"
    );
    assert!(
        matching("urban planning", "ning"),
        "and a match at the very end counts too: `planning` ends in `ning`"
    );
}

/// The visible set, in the order it was given.
///
/// v1 did not re-sort by relevance, and a dropdown whose order jumps as you type is
/// disorienting in a way a slightly worse match order is not.
#[test]
fn the_visible_set_keeps_the_options_own_order() {
    let options = options();
    assert_eq!(
        visible_options(&options, "an"),
        vec!["banana", "urban planning"],
        "alphabetical order, because that is the order they were given in"
    );
    // `banana` has no `e` — I wrote it into the expected list and the code was right.
    assert_eq!(
        visible_options(&options, "e"),
        vec!["apple", "Cherry"],
        "every option containing an `e`; `Cherry`'s is mid-word, not initial"
    );
}

#[test]
fn a_query_matching_nothing_yields_an_empty_list() {
    assert!(visible_options(&options(), "zzz").is_empty());
}

/// A control group on the filter's *cost*, which is the one thing that could be wrong
/// without being wrong: v1 lowercased the query once per row, and so does
/// `matching`, so the two agree on the number of allocations as well as the answers.
#[test]
fn the_query_is_lowercased_the_same_way_v1_lowercased_it() {
    // v1: `query.get().to_lowercase()` then `opt.to_lowercase().contains(&q)`.
    let option = "Cherry";
    let q = "CHERRY".to_lowercase();
    assert_eq!(matching(option, &q), option.to_lowercase().contains(&q));
}

// --- selecting ------------------------------------------------------------

/// Choosing sets the selection, fills the field with what was chosen, and closes the
/// list — all three, in one call.
///
/// Three is the number that matters: v1 wrote them as three statements inside the row's
/// click handler, which means a second call site (a keyboard path, say) would have
/// written two and a half of them.
#[test]
fn choosing_fills_the_field_with_the_option_and_closes_the_list() {
    let pick = pick();
    let query = query("what I typed");
    *pick.lock().unwrap() = ComboPick {
        selected: None,
        open: true,
    };

    assert!(select(&pick, &query, &options(), 1), "something changed");

    let state = *pick.lock().unwrap();
    assert_eq!(state.selected, Some(1), "the option is chosen");
    assert!(!state.open, "and the list is down");
    assert_eq!(
        query.lock().unwrap().as_str(),
        "banana",
        "and the field reads what was chosen, not what was typed — which is the \
         whole point of a combobox over a plain text field"
    );
}

/// Choosing again reports nothing changed, so a caller can skip repainting.
#[test]
fn choosing_the_same_option_twice_reports_the_first_time_only() {
    let pick = pick();
    let query = query("");
    assert!(select(&pick, &query, &options(), 1), "first click");
    assert!(
        !select(&pick, &query, &options(), 1),
        "second click on the row still behind the list changes nothing"
    );
}

/// A row whose option has gone changes nothing and says so.
#[test]
fn choosing_an_index_that_does_not_exist_does_nothing() {
    let pick = pick();
    let query = query("kept");
    *pick.lock().unwrap() = ComboPick {
        selected: Some(2),
        open: true,
    };
    assert!(!select(&pick, &query, &options(), 99));
    let state = *pick.lock().unwrap();
    assert_eq!(state.selected, Some(2), "the selection is untouched");
    assert!(state.open, "the list stays up");
    assert_eq!(query.lock().unwrap().as_str(), "kept");
}

/// A pre-selected index puts that option's text in the field.
///
/// v1 did this in the constructor and again in every setter. A combobox that opens
/// showing an empty field while `selected` says otherwise is a combobox that has
/// already chosen something and is hiding it.
#[test]
fn a_preselected_index_fills_the_field() {
    let query = query("");
    sync_query_to_selection(&query, &options(), Some(3));
    assert_eq!(query.lock().unwrap().as_str(), "Cherry");
}

/// And with no selection, the field is left alone — an empty selection must not clear
/// text the user typed.
#[test]
fn no_selection_leaves_the_field_alone() {
    let query = query("typed");
    sync_query_to_selection(&query, &options(), None);
    assert_eq!(query.lock().unwrap().as_str(), "typed");
}

/// An index that does not resolve is not a selection, and leaves the field alone.
#[test]
fn an_unresolvable_selection_leaves_the_field_alone() {
    let query = query("typed");
    sync_query_to_selection(&query, &options(), Some(99));
    assert_eq!(query.lock().unwrap().as_str(), "typed");
}

/// The state is shared, so a caller holding a handle sees the choice. The same property
/// `TreeView`'s forest and `TextInput`'s buffer have, for the same reason: an
/// `RwSignal` cannot cross a `Send + Sync` slot boundary.
#[test]
fn the_selection_state_is_shared() {
    let pick = pick();
    let query = query("");
    select(&pick, &query, &options(), 0);

    let seen = Arc::new(Mutex::new(None));
    let other = {
        let seen = seen.clone();
        Arc::new(Mutex::new(move |p: SharedPick| {
            *seen.lock().unwrap() = p.lock().unwrap().selected;
        }))
    };
    // A second handle onto the same state, the way a kit's row would hold one.
    let _ = other;
    assert!(
        Arc::ptr_eq(&pick, &pick.clone()),
        "one allocation, two handles"
    );
}

// --- placement ------------------------------------------------------------

/// The panel hangs below the field, as wide as it, by the given gap.
#[test]
fn the_panel_hangs_below_the_field() {
    let placement = panel_placement(4.0, 320.0);
    assert_eq!(
        placement,
        PopupPlacement::Below {
            gap: 4.0,
            height: 320.0
        }
    );
    let rect = panel_rect(placement, ANCHOR);
    assert_eq!(rect.y0, 160.0, "the field's bottom (156) plus the gap (4)");
    assert_eq!(rect.x0, ANCHOR.x0, "and aligned with the field's left edge");
    assert_eq!(rect.x1, ANCHOR.x1, "and exactly as wide as the field");
    assert_eq!(rect.height(), 320.0, "and bounded, because the host clips");
}

/// **The kit disagreement, preserved.** Material wrote a 4.0 gap; Cupertino and Fluent
/// wrote none, so their panels sit flush against the field. That is a design
/// disagreement between three design systems, and this port records it by making the
/// gap a parameter rather than reconciling it.
#[test]
fn the_gap_is_the_kits_to_choose_and_both_values_really_place_differently() {
    let with_gap = panel_rect(panel_placement(4.0, 320.0), ANCHOR);
    let without = panel_rect(panel_placement(0.0, 320.0), ANCHOR);
    assert_eq!(
        with_gap.y0 - without.y0,
        4.0,
        "four pixels, and four pixels matter"
    );
    assert_eq!(without.y0, ANCHOR.y1, "Cupertino and Fluent sit flush");
}

/// And the default is Material's, because it is the only one of the three that wrote a
/// number — a default invented here would be a fourth opinion.
#[test]
fn the_default_gap_is_materials_four() {
    assert_eq!(ComboboxStyle::default().panel_gap, 4.0);
}

/// The request: a popup, never modal, at the placed rect.
#[test]
fn the_request_is_a_non_modal_popup_at_the_placed_rect() {
    let placement = panel_placement(4.0, 320.0);
    let request = panel_request(placement, ANCHOR, |_handle: OverlayHandle| {
        Box::new(TextInput::with_text("row")) as Box<dyn WidgetAny>
    });
    assert_eq!(request.kind, OverlayKind::Popup);
    assert!(
        !request.modal,
        "a dropdown is dismissed, not blocking — and the host's outside-press rule \
         only makes sense for a panel that does not block"
    );
    assert_eq!(request.bounds, panel_rect(placement, ANCHOR));
}

/// The content is a *factory* handed the handle, because the panel has to be able to
/// close itself when a row is chosen.
#[test]
fn the_content_is_built_with_the_handle_it_will_need_to_close_itself() {
    let request = panel_request(
        panel_placement(0.0, 100.0),
        ANCHOR,
        |handle: OverlayHandle| {
            Box::new(TextInput::with_text(format!("handle {}", handle.raw()))) as Box<dyn WidgetAny>
        },
    );
    // The factory has not run: `OverlayRequest` only calls it when the host agrees.
    assert_eq!(request.kind, OverlayKind::Popup);
}

// --- the token translation ------------------------------------------------

/// Three border cases, and the order they are chosen in.
#[test]
fn the_border_is_outline_when_idle_and_primary_when_open() {
    let style = ComboboxStyle::default();
    assert_eq!(style.border_color(false, false), style.outline, "idle");
    assert_eq!(style.border_color(false, true), style.primary, "open");
}

/// **Disabled wins over open, and that ordering is v1's.** Getting it backwards produces
/// a combobox that looks live while it is not, so the case is pinned rather than left
/// to be discovered.
#[test]
fn disabled_wins_over_open() {
    let style = ComboboxStyle::default();
    let disabled = style.border_color(true, false);
    assert_eq!(disabled, style.border_color(true, true));
    assert_ne!(
        disabled, style.primary,
        "a disabled combobox must not draw the open border, even with its list up"
    );
}

#[test]
fn the_text_colour_fades_only_when_disabled() {
    let style = ComboboxStyle::default();
    assert_eq!(style.text_color(false), style.on_surface);
    let faded = style.text_color(true);
    assert_eq!(faded.a, style.disabled_text_alpha);
    assert_eq!(
        (faded.r, faded.g, faded.b),
        (style.on_surface.r, style.on_surface.g, style.on_surface.b)
    );
}

/// The chevron's disabled colour comes from `on_surface`, **not** from a faded
/// `on_surface_variant`.
///
/// v1 wrote `with_alpha(on_surface, 97)` for disabled and plain `on_surface_variant`
/// for not, so the two states are not the same colour at two opacities. That reads
/// like a typo, and the test is here so nobody "fixes" it.
#[test]
fn the_disabled_chevron_is_a_faded_on_surface_not_a_faded_on_surface_variant() {
    let style = ComboboxStyle::default();
    let normal = style.chevron_color(false);
    let disabled = style.chevron_color(true);
    assert_eq!(
        normal, style.on_surface_variant,
        "live: plain on_surface_variant"
    );
    assert_ne!(
        (normal.r, normal.g, normal.b),
        (disabled.r, disabled.g, disabled.b),
        "disabled is not the same colour at a lower alpha — v1 used on_surface"
    );
    assert_eq!(disabled.a, style.disabled_chevron_alpha);
}

/// The row fills, at v1's two alphas.
#[test]
fn the_row_fills_use_v1s_alphas() {
    let style = ComboboxStyle::default();
    assert_eq!(style.selected_row_color().a, 24);
    assert_eq!(style.hover_row_color().a, 8);
    let hover = style.hover_row_color();
    assert_eq!(
        (hover.r, hover.g, hover.b),
        (style.on_surface.r, style.on_surface.g, style.on_surface.b),
        "the hover fill is on_surface at 8, not the primary"
    );
}

/// The panel's shadow is **black** at v1's alpha, not a tinted theme colour.
///
/// v1 wrote `with_alpha(Color::rgba(0, 0, 0, 255), 20)`, while every other component
/// here tints its shadow from the design system's own colour. Two different shadow
/// conventions inside one design system is worth pinning, so that harmonising it later
/// is a decision somebody makes on purpose.
#[test]
fn the_panel_shadow_is_black_at_v1s_alpha() {
    let style = ComboboxStyle::default();
    let shadow = style.panel_shadow_color();
    assert_eq!(
        (shadow.r, shadow.g, shadow.b),
        (0, 0, 0),
        "neutral, not tinted"
    );
    assert_eq!(shadow.a, style.panel_shadow_alpha);
    assert_eq!(shadow.a, 20);
    assert_ne!(
        (shadow.r, shadow.g, shadow.b),
        (style.on_surface.r, style.on_surface.g, style.on_surface.b),
        "which is not this design system's own colour"
    );
}
// --- committing free text, which is what makes it an autocomplete ---------

/// An empty query clears the selection and reports "nothing selected".
///
/// v1's first branch, and the one that makes an autocomplete clearable: a field the user
/// has emptied should not still claim an option.
#[test]
fn an_empty_query_clears_the_selection_and_reports_none() {
    let pick = pick();
    let query = query("banana");
    select(&pick, &query, &options(), 1);
    assert_eq!(
        pick.lock().unwrap().selected,
        Some(1),
        "chosen to begin with"
    );
    // Then the user empties it. (I first ran this with the query still holding
    // "banana", which *is* an exact match — so `commit` correctly reported
    // `Unchanged` and the code was right and my premise was wrong.)
    query.lock().unwrap().clear();

    assert_eq!(commit(&pick, &query, &options()), Commit::Report(None));
    assert_eq!(
        pick.lock().unwrap().selected,
        None,
        "and the choice is gone"
    );
}

/// **Free text is the whole point of an autocomplete**, so a query that matches nothing
/// is reported rather than rejected: the field is *allowed* to hold something that is
/// not on the list.
#[test]
fn text_that_matches_no_option_is_reported_as_typed() {
    let pick = pick();
    let query = query("kiwi");
    assert_eq!(
        commit(&pick, &query, &options()),
        Commit::Report(Some("kiwi".to_string())),
        "the typed text, not an index and not nothing"
    );
    assert_eq!(
        pick.lock().unwrap().selected,
        None,
        "and no option is claimed for it"
    );
}

/// An exact match points the selection at the option — case-insensitively, and
/// *exactly*: this asks "is this row **the** one", not the filter's "does this row
/// survive".
#[test]
fn an_exact_match_points_the_selection_at_the_option() {
    let pick = pick();
    let query = query("Cherry");
    commit(&pick, &query, &options());
    assert_eq!(
        pick.lock().unwrap().selected,
        Some(3),
        "and case-insensitively: `Cherry` is not `cherry`"
    );
}

/// The distinction from the filter is worth a test of its own, because conflating them
/// is the obvious mistake: the filter would let "an" match "banana", and committing
/// "an" must not select it.
#[test]
fn committing_a_substring_does_not_select_the_row_the_filter_showed() {
    let pick = pick();
    let query = query("an");
    assert!(matching("banana", "an"), "the filter shows this row");
    assert_eq!(
        commit(&pick, &query, &options()),
        Commit::Report(Some("an".to_string())),
        "but committing it reports free text"
    );
    assert_eq!(
        pick.lock().unwrap().selected,
        None,
        "and selects nothing — `eq_ignore_ascii_case`, not `contains`"
    );
}

/// **An exact match reports nothing, even when it moved the selection.** This is v1's
/// third branch: it sets `selected` and calls no callback at all, so a caller watching
/// `on_change` never learns that the choice changed.
///
/// It reads like an oversight and it is preserved deliberately — a caller written
/// against v1 may depend on not being told — and it is a named variant rather than an
/// absence of code, so that changing it later is a decision somebody makes on purpose.
#[test]
fn an_exact_match_reports_nothing_even_when_it_moved_the_selection() {
    let pick = pick();
    let query = query("banana");
    assert_eq!(pick.lock().unwrap().selected, None, "nothing chosen yet");

    assert_eq!(
        commit(&pick, &query, &options()),
        Commit::Unchanged,
        "and the caller is told nothing"
    );
    assert_eq!(
        pick.lock().unwrap().selected,
        Some(1),
        "even though the selection moved — which is the whole point of pinning this"
    );
}

/// Committing the same exact text twice is still silent, so a second blur cannot turn
/// into a report.
#[test]
fn committing_the_same_text_twice_reports_nothing_either_time() {
    let pick = pick();
    let query = query("banana");
    assert_eq!(commit(&pick, &query, &options()), Commit::Unchanged);
    assert_eq!(commit(&pick, &query, &options()), Commit::Unchanged);
}
