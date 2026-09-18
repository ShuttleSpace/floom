//! Task 27.1 — the reference gallery.
//!
//! Nine representative screens built from real widgets and mounted in the real
//! headless shell. The gallery exists so the accuracy loop measures what a user
//! would see; a screen that fails to mount or loses a label is a fixture bug,
//! and these tests are the fixture's own gate.

use floem_accuracy::{Screen, ScreenTokens, evaluate_screen};

#[test]
fn every_screen_mounts_and_publishes_its_labels() {
    for screen in Screen::ALL {
        let report = evaluate_screen(screen, &ScreenTokens::reference());
        assert!(
            report.labels_missing.is_empty(),
            "{screen:?} is missing labels {report:?}"
        );
    }
}

#[test]
fn every_screen_has_a_reference_contract() {
    for screen in Screen::ALL {
        let contract = floem_accuracy::reference_contract(screen);
        assert_eq!(contract.name, screen.title());
        assert!(
            contract
                .version
                .accepts(floem_ui_ir::ContractVersion::CURRENT),
            "{screen:?} fixture was written for an older contract"
        );
        // The floors the evaluation enforces are part of the fixture, not
        // hardcoded in the harness.
        assert!(contract.accessibility.min_contrast >= 4.5);
        assert!(contract.accessibility.min_touch_target >= 44.0);
    }
}

#[test]
fn the_screens_are_nine_distinct_paintings() {
    // A gallery of relabelled clones would make every later "differs" claim
    // vacuous: nine screens, nine distinct pixel fingerprints.
    let mut hashes: Vec<String> = Screen::ALL
        .iter()
        .map(|screen| evaluate_screen(*screen, &ScreenTokens::reference()).pixel_hash)
        .collect();
    hashes.sort();
    let before = hashes.len();
    hashes.dedup();
    assert_eq!(hashes.len(), before, "two screens rendered identically");
}

#[test]
fn the_webview_screen_is_an_honest_placeholder() {
    // Real web content needs the Phase 17 OS providers, which this machine
    // cannot exercise. The screen exists, labelled, with the gap spelled out.
    let report = evaluate_screen(Screen::Webview, &ScreenTokens::reference());
    assert!(report.labels_missing.is_empty());
    assert!(Screen::Webview.placeholder_note().contains("Phase 17"));
}
