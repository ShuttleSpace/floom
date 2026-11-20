use floom_design::{contrast_ratio, gallery_size, GallerySample};
use floom_fluent::{
    button_gallery, button_gallery_labels, FluentColors, FluentTheme, GALLERY_COLUMNS,
};
use floom_snapshot::{
    assert_snapshot, snapshots_dir, PixelSnapshot, ReferencePixels, SceneSnapshot,
};
use floom_theme::ColorScheme;
use floom_types::Color;

#[test]
fn the_light_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&FluentTheme::light()))
        .assert(snapshots_dir!(), "fluent_gallery_light");
}

#[test]
fn the_dark_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&FluentTheme::dark()))
        .assert(snapshots_dir!(), "fluent_gallery_dark");
}

#[test]
fn the_resolved_property_table_matches_its_golden() {
    assert_snapshot(
        snapshots_dir!(),
        "fluent_labels_light",
        &button_gallery_labels(&FluentTheme::light()),
    );
    assert_snapshot(
        snapshots_dir!(),
        "fluent_labels_dark",
        &button_gallery_labels(&FluentTheme::dark()),
    );
}

#[test]
fn the_gallery_renders_to_the_expected_pixels() {
    let theme = FluentTheme::light();
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    PixelSnapshot::of(&button_gallery(&theme), None, size, 1.0)
        .assert(snapshots_dir!(), "fluent_gallery_pixels");
}

#[test]
fn every_declared_pair_clears_its_own_threshold() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let colors = FluentTheme::for_scheme(scheme).colors;
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
fn the_stroke_is_held_to_visibility_and_not_to_accessibility() {
    // A separate assertion because it is a different *kind* of claim. If this
    // starts failing, either the stroke moved or someone promoted it to a
    // boundary — both are deliberate decisions.
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let colors = FluentTheme::for_scheme(scheme).colors;
        let ratio = contrast_ratio(colors.stroke_default, colors.card_background);
        assert!(
            ratio >= FluentColors::STROKE_VISIBILITY_FLOOR,
            "{scheme:?} stroke vanished: {ratio:.2}:1"
        );
        assert!(
            ratio < floom_design::WCAG_AA_LARGE,
            "{scheme:?} stroke is now an accessible boundary: {ratio:.2}:1"
        );
    }
}

#[test]
fn the_light_schemes_accent_content_only_just_clears_aa() {
    // Fluent's published white-on-accent pairing is a narrow pass. Pinning the
    // margin means a small token tweak cannot silently drop it below AA.
    let colors = FluentTheme::light().colors;
    let ratio = contrast_ratio(colors.on_accent, colors.accent);
    assert!(
        (floom_design::WCAG_AA_NORMAL..floom_design::WCAG_AA_NORMAL + 0.2).contains(&ratio),
        "white on Fluent light accent is {ratio:.3}:1"
    );
}

#[test]
fn the_snapshot_moves_when_the_accent_moves() {
    let base = FluentTheme::light();
    let mut changed = FluentTheme::light();
    changed.colors.accent = Color::rgb(0, 200, 0);
    assert_ne!(
        SceneSnapshot::of(&button_gallery(&base)).hash,
        SceneSnapshot::of(&button_gallery(&changed)).hash,
        "a recoloured accent left the scene identical"
    );
}

#[test]
fn the_two_schemes_render_differently() {
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    let l = PixelSnapshot::of(&button_gallery(&FluentTheme::light()), None, size, 1.0);
    let d = PixelSnapshot::of(&button_gallery(&FluentTheme::dark()), None, size, 1.0);
    assert_ne!(l.hash, d.hash, "light and dark rendered identically");
}
