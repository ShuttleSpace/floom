// SPDX-License-Identifier: MIT
//! Task 27.2 — the render-and-compare loop.
//!
//! `evaluate_screen` mounts a screen headless, measures geometry, style,
//! semantics and pixels out of the *rendered* result, and compares them
//! against the screen's reference design contract (Phase 26 vocabulary). The
//! tests here pin both directions: the reference implementation must be clean,
//! and every introduced drift must be caught at a named path.

use floom_accuracy::{EvaluationReport, Screen, ScreenTokens, evaluate_screen};

#[test]
fn the_reference_implementation_matches_its_contract() {
    for screen in Screen::ALL {
        let report = evaluate_screen(screen, &ScreenTokens::reference());
        assert!(
            report.is_clean(),
            "{screen:?} drifted from its own contract: {}",
            report.to_json()
        );
    }
}

#[test]
fn a_drifted_implementation_disagrees_at_named_paths() {
    let tokens = ScreenTokens::reference().drifted(&["gap", "radius", "accent"]);
    let report = evaluate_screen(Screen::Settings, &tokens);
    let paths: Vec<&str> = report
        .diff
        .entries
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    // `compare` walks the contract in field order: spacing, then radius, then
    // the sorted colour roles.
    assert_eq!(
        paths,
        vec!["spacing.gap", "radius.corner", "colors.accent.value"]
    );
    // The drifted accent also drags white-on-accent under the WCAG floor, so
    // the report carries the three contract paths *plus* that floor failure.
    assert_eq!(report.diff.entries.len(), 3);
    assert_eq!(report.problems(), 4);
    assert!(report.floors.iter().any(|f| f.check.contains("contrast")));
}

#[test]
fn a_colour_drift_is_visible_in_the_pixels() {
    // Control for the whole harness: the pixel layer must actually carry the
    // tokens, so a repaint that changed a colour changes the fingerprint.
    let reference = evaluate_screen(Screen::Dashboard, &ScreenTokens::reference());
    let drifted = evaluate_screen(
        Screen::Dashboard,
        &ScreenTokens::reference().drifted(&["accent", "card"]),
    );
    assert_ne!(reference.pixel_hash, drifted.pixel_hash);
}

#[test]
fn rendering_is_deterministic() {
    let report = evaluate_screen(Screen::Feed, &ScreenTokens::reference());
    assert!(report.pixel_stable, "two renders of one mount disagreed");
}

#[test]
fn the_report_round_trips_through_json() {
    let report = evaluate_screen(
        Screen::Dialog,
        &ScreenTokens::reference().drifted(&["padding"]),
    );
    let parsed = EvaluationReport::from_json(&report.to_json()).expect("valid JSON");
    assert_eq!(parsed, report);
}

#[test]
fn typography_measurement_tracks_the_token() {
    // The falsifiability check for the type ramp: a bigger body size must
    // produce a bigger measured line height, or the measurement is not
    // measuring type.
    let small = evaluate_screen(Screen::Feed, &ScreenTokens::reference());
    let big = evaluate_screen(
        Screen::Feed,
        &ScreenTokens::reference().drifted(&["body_size"]),
    );
    assert!(big.observation.body_line_height > small.observation.body_line_height);
    assert_ne!(
        small.observation.body_font_size,
        big.observation.body_font_size
    );
}

#[test]
fn the_contrast_floor_catches_an_unreadable_theme() {
    // Control first: the reference passes its own floor.
    let reference = evaluate_screen(Screen::Settings, &ScreenTokens::reference());
    assert!(reference.floors.is_empty());

    // Pale text on a pale card is exactly the failure the floor exists for.
    let report = evaluate_screen(
        Screen::Settings,
        &ScreenTokens::reference().drifted(&["text"]),
    );
    assert!(
        report.floors.iter().any(|f| f.check.contains("contrast")),
        "pale-on-pale was reported clean: {report:?}"
    );
}

#[test]
fn the_touch_target_floor_catches_an_undersized_action() {
    let report = evaluate_screen(
        Screen::Form,
        &ScreenTokens::reference().drifted(&["control_height"]),
    );
    assert!(
        report
            .floors
            .iter()
            .any(|f| f.check.contains("touch target")),
        "a 32 px action passed a 48 px floor: {report:?}"
    );
}
