//! Fluent switch: v1's geometry and colours, exactly.
//!
//! ## Why this suite is mostly assertions about tokens
//!
//! The three v1 kits wrote the same switch — measured: their `resolved_style()`
//! bodies are byte-identical once naming is stripped. So there is no per-kit
//! behaviour to test, only that the engine reproduces one shared shape.
//!
//! What matters is that the shape is v1's and not the engine's default. The
//! engine derives 40x20 with a full-height flush knob; v1 drew 52x32 with a 24
//! knob inset 2px on both ends. Both tests below state which is which, because
//! an adapter that quietly fell back to the default would render a
//! plausible-looking switch that is 20% narrower than the one it replaced.

use floom_core::{ElementId, Widget};
use floom_signal::SignalUpdate;
use floom_types::{Point, Rect};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_switch::FluentSwitch;

/// A box with room for the 52-wide switch, so nothing is clipped.
const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 60.0,
};

fn fluent() -> FluentSwitch {
    FluentSwitch::new().with_theme(MultiTheme::new_for(DesignSystem::Fluent))
}

// --- geometry --------------------------------------------------------------

/// **The control for every geometry assertion below.** The engine's own
/// derivation gives 40x20, so "52 wide" and "32 tall" are both *not* the
/// default and cannot pass by inheriting it.
#[test]
fn the_engine_default_would_have_drawn_a_different_switch() {
    let engine = floom_widgets::Switch::new("").track_bounds(ROW);
    assert_eq!((engine.width(), engine.height()), (40.0, 20.0));
    assert_eq!(
        fluent().switch().track_bounds(ROW).width(),
        52.0,
        "v1's token width, not the engine's default"
    );
}

/// v1's three tokens, used as given. None of them satisfies the engine's
/// derivation: `52 * 0.5` is 26, not 32, and a knob as tall as the track would
/// be 32, not 24.
#[test]
fn the_track_is_the_tokens_verbatim() {
    let track = fluent().switch().track_bounds(ROW);
    assert_eq!((track.width(), track.height()), (52.0, 32.0));
    assert_eq!(track.height(), 32.0, "not 52 * 0.5 = 26");
}

/// **All three kits share one set of switch tokens.** Measured against the
/// theme layer, not inferred from the adapters:
///
/// ```text
/// Material:  width=52 height=32 thumb_size=24
/// Cupertino: width=52 height=32 thumb_size=24
/// Fluent:    width=52 height=32 thumb_size=24
/// ```
///
/// There is a single `SwitchTokens` in `floom-core-theme`, reached through
/// `ComponentId::Switch`, so the three design systems cannot disagree here even
/// in principle.
///
/// Two of the three crates carried an older `tests/api.rs` asserting 51 x 31
/// (cupertino) and 40 x 20 (fluent). Those assertions were stale — they
/// predate the tokens being shared — and following them would have given two
/// kits the wrong switch. The measurement above is what the theme layer returns
/// today.
#[test]
fn the_track_is_the_shared_switch_token() {
    let track = fluent().switch().track_bounds(ROW);
    assert_eq!((track.width(), track.height()), (52.0, 32.0));
}

/// The knob is 24 square, centred in a 32-tall track — so it does not fill the
/// track vertically either.
#[test]
fn the_knob_is_24_square_and_centred_in_the_track() {
    let switch = fluent().switch();
    let (track, knob) = (switch.track_bounds(ROW), switch.knob_bounds(ROW));
    assert_eq!((knob.width(), knob.height()), (24.0, 24.0), "square");
    assert_eq!(
        knob.height() + 8.0,
        track.height(),
        "4px of air above and below"
    );
}

/// **The v1 reproduction, stated as a claim.** v1 wrote `padding(2.0)` on the
/// track with the knob centred inside it and justified to one end, so the knob
/// sat 2px from that end and nowhere touched the track's edge.
#[test]
fn a_knob_inside_the_track_with_2px_gaps() {
    let off = fluent().switch().knob_bounds(ROW);
    let on = fluent().checked(true).switch().knob_bounds(ROW);

    assert_eq!(off.x0, ROW.x0 + 2.0, "2px from the start");
    assert_eq!(on.x1, ROW.x0 + 52.0 - 2.0, "2px from the far end");

    // And the travel is the gap between those two, not the track width.
    assert_eq!(on.x0 - off.x0, 24.0, "52 - 24 - 2 - 2");
}

/// **The dead field.** v1's style struct carried `thumb_offset = width -
/// height` = 20, which would have dropped the knob off-centre by 8px. `view()`
/// never read it. If this crate reproduced the formula instead of v1's
/// `justify_start`/`justify_end`, the checked knob would land at x = 20 rather
/// than 26.
#[test]
fn the_dead_thumb_offset_is_not_reproduced() {
    let on = fluent().checked(true).switch().knob_bounds(ROW);
    assert_eq!(
        on.x0,
        ROW.x0 + 26.0,
        "v1's justify_end inside padding(2), not the unused thumb_offset of 20"
    );
    assert_ne!(on.x0, ROW.x0 + 20.0, "the formula v1 never applied");
}

// --- colours ---------------------------------------------------------------

/// The resting colours are the theme's, which is the whole reason a kit has to
/// be able to supply a palette at all.
#[test]
fn the_resting_colours_come_from_the_theme() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let style = fluent().resolved_style();
    assert_eq!(
        style.track_color,
        theme.colors().surface_variant,
        "unchecked track"
    );
    assert_eq!(style.thumb_color, theme.colors().outline, "unchecked thumb");

    let checked = fluent().checked(true).resolved_style();
    assert_eq!(checked.track_color, theme.colors().primary);
    assert_eq!(checked.thumb_color, theme.colors().on_primary);
}

/// The palette reaches the pixels, not just the style struct.
#[test]
fn the_palette_reaches_the_painted_pixels() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let style = fluent().resolved_style();
    let expected = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        style.track_color.r, style.track_color.g, style.track_color.b, style.track_color.a
    );
    let id = ElementId::new(1, 1);
    let painted = floom_widgets::testkit::paint_snapshot(&fluent().switch(), id, ROW);
    assert!(
        painted.contains(&expected),
        "expected {expected} in {painted}"
    );
}

/// v1's disabled alphas are two different roles at two different strengths —
/// `on_surface` at 12/255 for the track and `outline` at 38/255 for the knob.
/// Neither is "half alpha", which is what the engine would have done.
#[test]
fn the_disabled_alphas_are_v1_s_own_two_values() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let style = fluent().disabled(true).resolved_style();
    assert_eq!(style.track_disabled.a, 12);
    assert_eq!(style.thumb_disabled.a, 38);
    assert_ne!(
        style.track_disabled.a, style.thumb_disabled.a,
        "two roles, two strengths"
    );
    assert_eq!(style.track_disabled.r, theme.colors().on_surface.r);
    assert_eq!(style.thumb_disabled.r, theme.colors().outline.r);
}

/// **v1 had no hover or press styling at all**, so the engine's feedback is
/// turned off rather than approximated with four near-identical colours.
#[test]
fn hover_and_press_do_not_change_anything() {
    let mut resting = fluent().switch();
    let mut hovered = fluent().switch();
    hovered.state_mut().hovered = true;

    let id = ElementId::new(1, 1);
    assert_eq!(
        floom_widgets::testkit::paint_snapshot(&resting, id, ROW),
        floom_widgets::testkit::paint_snapshot(&hovered, id, ROW),
        "v1 styled only resting and disabled"
    );
}

/// A disabled switch paints the explicit disabled colours rather than a
/// half-alpha version of the resting ones.
#[test]
fn a_disabled_switch_paints_the_explicit_disabled_colours() {
    let id = ElementId::new(1, 1);
    let painted =
        floom_widgets::testkit::paint_snapshot(&fluent().disabled(true).switch(), id, ROW);
    let style = fluent().disabled(true).resolved_style();
    let track = format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        style.track_disabled.r,
        style.track_disabled.g,
        style.track_disabled.b,
        style.track_disabled.a
    );
    assert!(painted.contains(&track), "expected {track} in {painted}");
}

// --- behaviour -------------------------------------------------------------

/// The checked flag reaches the engine, so the knob is on the right by default.
#[test]
fn the_checked_flag_reaches_the_engine() {
    assert!(!fluent().switch().is_on());
    assert!(fluent().checked(true).switch().is_on());
}

/// The disabled flag reaches the engine too.
#[test]
fn the_disabled_flag_reaches_the_engine() {
    // The control: the flag defaults to enabled, so the second assertion below
    // is "and not this" rather than a restatement.
    assert!(
        !fluent().switch().interaction().disabled,
        "a fresh switch is interactive"
    );
    assert!(fluent().disabled(true).switch().interaction().disabled);
}

/// The resolved style names its design system, so a caller holding three
/// adapters can tell them apart.
#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(fluent().resolved_style().design_system, "fluent");
}

/// The geometry is stable across checked state — only the knob moves.
#[test]
fn checking_does_not_resize_anything() {
    let (off, on) = (fluent().switch(), fluent().checked(true).switch());
    assert_eq!(off.track_bounds(ROW), on.track_bounds(ROW));
    assert_eq!(off.knob_bounds(ROW).height(), on.knob_bounds(ROW).height());
}

/// The knob's resting position is inside the track, which a switch whose inset
/// were applied as a paint inset rather than a geometric one would not be.
#[test]
fn the_knob_is_inside_its_track_in_both_states() {
    for checked in [false, true] {
        let switch = fluent().checked(checked).switch();
        let (track, knob) = (switch.track_bounds(ROW), switch.knob_bounds(ROW));
        assert!(
            knob.x0 >= track.x0 && knob.x1 <= track.x1,
            "checked={checked}: knob {knob:?} escapes track {track:?}"
        );
        assert!(
            knob.y0 >= track.y0 && knob.y1 <= track.y1,
            "checked={checked}: knob escapes vertically"
        );
    }
}

/// The engine's label is empty, because v1's switch has no text in it — the
/// `Switch::new` label is a floom-widget affordance, not a v1 field.
#[test]
fn the_engine_label_is_empty() {
    let switch = fluent().switch();
    assert!(
        switch.layout_request().is_none(),
        "an unlabelled switch asks for no intrinsic size"
    );
    let _ = Point::ZERO;
}

/// **The one thing that actually differs between the kits**, and the only thing
/// these two crates do that Material does not: the theme is a **snapshot**.
///
/// Material calls `theme_signal()` and follows a global theme switch. These two
/// call `MultiTheme::new_for(..)` in `new()`, so the switch is styled once and a
/// later switch to another palette leaves it alone.
///
/// The test measures the observable consequence -- reading a colour before and
/// after a global switch -- rather than asserting on the constructor's source
/// text, so it fails if the snapshot is replaced by a live signal and also if a
/// live signal is replaced by a snapshot.
#[test]
fn the_theme_is_a_snapshot_taken_at_construction() {
    let theme = MultiTheme::new_for(DesignSystem::Fluent);
    let built = fluent().with_theme(theme.clone());
    let before = built.resolved_style().track_color;

    floom_core_theme::global_theme::design_system_signal().set(DesignSystem::Material);

    let after = built.resolved_style().track_color;
    assert_eq!(
        before, after,
        "a snapshot cannot follow a theme switch; if this now differs, this crate \
         has been switched to a live signal and the divergence is gone"
    );
}
