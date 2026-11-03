//!

use floom_core::{ElementId, Widget};
use floom_signal::SignalUpdate;
use floom_types::{Point, Rect};

use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

use floom_widget_fluent_switch::FluentSwitch;

const ROW: Rect = Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 200.0,
    y1: 60.0,
};

fn fluent() -> FluentSwitch {
    FluentSwitch::new().with_theme(MultiTheme::new_for(DesignSystem::Fluent))
}


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

#[test]
fn the_track_is_the_tokens_verbatim() {
    let track = fluent().switch().track_bounds(ROW);
    assert_eq!((track.width(), track.height()), (52.0, 32.0));
    assert_eq!(track.height(), 32.0, "not 52 * 0.5 = 26");
}

#[test]
fn the_track_is_the_shared_switch_token() {
    let track = fluent().switch().track_bounds(ROW);
    assert_eq!((track.width(), track.height()), (52.0, 32.0));
}

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

#[test]
fn a_knob_inside_the_track_with_2px_gaps() {
    let off = fluent().switch().knob_bounds(ROW);
    let on = fluent().checked(true).switch().knob_bounds(ROW);

    assert_eq!(off.x0, ROW.x0 + 2.0, "2px from the start");
    assert_eq!(on.x1, ROW.x0 + 52.0 - 2.0, "2px from the far end");

    assert_eq!(on.x0 - off.x0, 24.0, "52 - 24 - 2 - 2");
}

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


#[test]
fn the_checked_flag_reaches_the_engine() {
    assert!(!fluent().switch().is_on());
    assert!(fluent().checked(true).switch().is_on());
}

#[test]
fn the_disabled_flag_reaches_the_engine() {
    assert!(
        !fluent().switch().interaction().disabled,
        "a fresh switch is interactive"
    );
    assert!(fluent().disabled(true).switch().interaction().disabled);
}

#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(fluent().resolved_style().design_system, "fluent");
}

#[test]
fn checking_does_not_resize_anything() {
    let (off, on) = (fluent().switch(), fluent().checked(true).switch());
    assert_eq!(off.track_bounds(ROW), on.track_bounds(ROW));
    assert_eq!(off.knob_bounds(ROW).height(), on.knob_bounds(ROW).height());
}

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

#[test]
fn the_engine_label_is_empty() {
    let switch = fluent().switch();
    assert!(
        switch.layout_request().is_none(),
        "an unlabelled switch asks for no intrinsic size"
    );
    let _ = Point::ZERO;
}

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
