//! Cupertino gallery snapshots and the colour audit.

use floem_cupertino::{CupertinoTheme, GALLERY_COLUMNS, button_gallery, button_gallery_labels};
use floem_design::{GallerySample, contrast_ratio, gallery_size};
use floem_snapshot::{PixelSnapshot, SceneSnapshot, assert_snapshot, snapshots_dir};
use floem_theme::ColorScheme;
use floem_types::Color;

#[test]
fn the_light_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&CupertinoTheme::light()))
        .assert(snapshots_dir!(), "cupertino_gallery_light");
}

#[test]
fn the_dark_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&CupertinoTheme::dark()))
        .assert(snapshots_dir!(), "cupertino_gallery_dark");
}

#[test]
fn the_resolved_property_table_matches_its_golden() {
    assert_snapshot(
        snapshots_dir!(),
        "cupertino_labels_light",
        &button_gallery_labels(&CupertinoTheme::light()),
    );
    assert_snapshot(
        snapshots_dir!(),
        "cupertino_labels_dark",
        &button_gallery_labels(&CupertinoTheme::dark()),
    );
}

#[test]
fn the_gallery_renders_to_the_expected_pixels() {
    let theme = CupertinoTheme::light();
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    PixelSnapshot::of(&button_gallery(&theme), None, size, 1.0)
        .assert(snapshots_dir!(), "cupertino_gallery_pixels");
}

#[test]
fn every_declared_pair_clears_its_own_threshold() {
    // The thresholds differ per pair on purpose — see `contrast_pairs`. This
    // test's job is to hold the pair list to what it claims, in both schemes.
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let colors = CupertinoTheme::for_scheme(scheme).colors;
        for (name, fg, bg, threshold) in colors.contrast_pairs() {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= threshold,
                "{scheme:?} {name}: {ratio:.2}:1 is below the claimed {threshold:.1}:1"
            );
        }
    }
}

#[test]
fn the_tint_pair_is_exactly_the_case_the_threshold_was_lowered_for() {
    // Apple's systemBlue on white does not clear AA for body text. Pin the fact
    // so nobody "fixes" the palette and quietly stops shipping Cupertino.
    let colors = CupertinoTheme::light().colors;
    let ratio = contrast_ratio(colors.on_tint, colors.tint);
    assert!(
        ratio > floem_design::WCAG_AA_LARGE && ratio < floem_design::WCAG_AA_NORMAL,
        "systemBlue/white is {ratio:.2}:1; if Apple's value or the audit changed, revisit the claim"
    );
}

#[test]
fn the_snapshot_moves_when_the_tint_moves() {
    let base = CupertinoTheme::light();
    let mut changed = CupertinoTheme::light();
    changed.colors.tint = Color::rgb(255, 0, 0);
    assert_ne!(
        SceneSnapshot::of(&button_gallery(&base)).hash,
        SceneSnapshot::of(&button_gallery(&changed)).hash,
        "a recoloured tint left the scene identical"
    );
}

#[test]
fn the_two_schemes_render_differently() {
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    let l = PixelSnapshot::of(&button_gallery(&CupertinoTheme::light()), None, size, 1.0);
    let d = PixelSnapshot::of(&button_gallery(&CupertinoTheme::dark()), None, size, 1.0);
    assert_ne!(l.hash, d.hash, "light and dark rendered identically");
}
