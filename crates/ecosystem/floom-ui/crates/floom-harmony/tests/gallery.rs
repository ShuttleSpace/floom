//! HarmonyOS gallery snapshots and the capsule audit.
//!
//! Harmony's signature rule is that a control's corner is *derived* from its own
//! height, so the same variant is a different shape at each size step. That is
//! a claim about the raster, not about the property table, so the strongest test
//! here reads pixels: the corner of a filled capsule must not be filled.

use floom_design::{
    CellPaint, ControlSize, GallerySample, InteractionState, Variant, contrast_ratio, gallery_size,
};
use floom_harmony::{
    GALLERY_COLUMNS, HarmonyTheme, button_gallery, button_gallery_labels, button_metrics,
    button_style,
};
use floom_scene::SceneNode;
use floom_snapshot::{PixelSnapshot, SceneSnapshot, assert_snapshot, snapshots_dir};
use floom_theme::ColorScheme;
use floom_types::{Color, Size};

fn light() -> HarmonyTheme {
    HarmonyTheme::light()
}

fn dark() -> HarmonyTheme {
    HarmonyTheme::dark()
}

/// One filled control at `size`, alone in a one-column gallery, so its cell
/// occupies the whole canvas and a pixel probe needs no grid arithmetic.
///
/// `square` zeroes the derived radius, which exists for the control group in
/// [`the_capsule_corner_is_empty_and_its_centre_is_not`] — a probe that cannot
/// tell the two apart proves nothing about the corner.
fn solo_filled_shape(theme: &HarmonyTheme, size: ControlSize, square: bool) -> PixelSnapshot {
    let samples = [GallerySample::new(
        Variant::Filled,
        size,
        InteractionState::Rest,
    )];
    let scene = floom_design::gallery_scene(&samples, theme.colors.background, 1, |sample| {
        let mut props = button_style(theme, sample.variant, sample.size, sample.state);
        if square {
            props.radius = Some(0.0);
        }
        CellPaint::new(props, button_metrics(theme, sample.size).height as f64)
    });
    PixelSnapshot::of(&scene, None, gallery_size(&samples, 1), 1.0)
}

fn solo_filled(theme: &HarmonyTheme, size: ControlSize) -> PixelSnapshot {
    solo_filled_shape(theme, size, false)
}

#[test]
fn the_light_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&light())).assert(snapshots_dir!(), "harmony_gallery_light");
}

#[test]
fn the_dark_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&dark())).assert(snapshots_dir!(), "harmony_gallery_dark");
}

#[test]
fn the_resolved_property_table_matches_its_golden() {
    assert_snapshot(
        snapshots_dir!(),
        "harmony_labels_light",
        &button_gallery_labels(&light()),
    );
    assert_snapshot(
        snapshots_dir!(),
        "harmony_labels_dark",
        &button_gallery_labels(&dark()),
    );
}

#[test]
fn the_gallery_renders_to_the_expected_pixels() {
    let theme = light();
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    PixelSnapshot::of(&button_gallery(&theme), None, size, 1.0)
        .assert(snapshots_dir!(), "harmony_gallery_pixels");
}

#[test]
fn the_capsule_corner_is_empty_and_its_centre_is_not() {
    // The whole point of a capsule. A `radius` value that never reaches the
    // rasterizer leaves every style test green and every control square, so this
    // asserts on the painted result: the bounding box's corner is outside the
    // shape (the page shows through) while the middle is the brand colour.
    let theme = light();
    let shot = solo_filled(&theme, ControlSize::Lg);
    let brand = theme.colors.brand;

    let corner = shot.rgba_pixel(1, 1).expect("in bounds");
    assert_ne!(
        [corner[0], corner[1], corner[2]],
        [brand.r, brand.g, brand.b],
        "the corner is filled: the radius never reached the raster"
    );

    let middle = shot.rgba_pixel(80, 24).expect("in bounds");
    assert_eq!(
        [middle[0], middle[1], middle[2]],
        [brand.r, brand.g, brand.b],
        "the body of the capsule is not the brand colour"
    );

    // Control group: the identical probe over a square control must *find* the
    // brand colour. Without this, the assertion above would also pass if the
    // gallery simply never painted that pixel at all.
    let square = solo_filled_shape(&theme, ControlSize::Lg, true);
    let square_corner = square.rgba_pixel(1, 1).expect("in bounds");
    assert_eq!(
        [square_corner[0], square_corner[1], square_corner[2]],
        [brand.r, brand.g, brand.b],
        "the probe cannot see a filled corner even when there is one"
    );
}

#[test]
fn the_painted_body_follows_the_size_step() {
    // Material's size steps differ only in height, so a gallery that drew every
    // control in a fixed box would show three identical swatches. Harmony's
    // differ in height *and* in the radius derived from it, so the difference has
    // to be measurable in the pixels, not just in the metrics struct.
    let theme = light();
    let tall = solo_filled(&theme, ControlSize::Lg);
    let short = solo_filled(&theme, ControlSize::Sm);
    let brand = theme.colors.brand;
    let is_brand = |p: Option<[u8; 4]>| {
        p.map(|p| [p[0], p[1], p[2]] == [brand.r, brand.g, brand.b])
            .unwrap_or(false)
    };

    // Near the top of the canvas: the 48px control reaches it, the 32px one is
    // centred and starts 8px lower.
    assert!(
        is_brand(tall.rgba_pixel(80, 2)),
        "the tall control stops short of the top"
    );
    assert!(
        !is_brand(short.rgba_pixel(80, 2)),
        "the short control reaches the top"
    );
    // Both still cover their own middle.
    assert!(is_brand(short.rgba_pixel(80, 24)));
}

#[test]
fn a_larger_control_has_a_larger_derived_corner() {
    // Stated against the glyph rule rather than against a number, so a table
    // rewrite that hard-codes a radius fails here.
    let theme = light();
    let radius = |size| {
        button_style(&theme, Variant::Filled, size, InteractionState::Rest)
            .radius
            .expect("a capsule has a radius")
    };
    assert!(radius(ControlSize::Sm) < radius(ControlSize::Md));
    assert!(radius(ControlSize::Md) < radius(ControlSize::Lg));
    // Half the height, exactly, at every step.
    for size in ControlSize::ALL {
        assert_eq!(
            radius(size),
            button_metrics(&theme, size).height / 2.0,
            "{size:?}"
        );
    }
}

#[test]
fn the_pixel_hash_follows_the_scheme() {
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    let l = PixelSnapshot::of(&button_gallery(&light()), None, size, 1.0);
    let d = PixelSnapshot::of(&button_gallery(&dark()), None, size, 1.0);
    assert_ne!(l.hash, d.hash, "light and dark rendered identically");
    assert_eq!((l.width, l.height), (d.width, d.height));
}

#[test]
fn the_scene_snapshot_moves_when_a_single_role_moves() {
    let base = light();
    let mut changed = light();
    changed.colors.brand = Color::rgb(0, 200, 0);
    let a = SceneSnapshot::of(&button_gallery(&base));
    let b = SceneSnapshot::of(&button_gallery(&changed));
    assert_ne!(
        a.hash, b.hash,
        "a recoloured brand left the scene identical"
    );

    // The divider is used only by the disabled outlined control. If recolouring
    // it moves the scene, that path is genuinely painted.
    let mut divider_changed = light();
    divider_changed.colors.divider = Color::rgb(255, 0, 255);
    let c = SceneSnapshot::of(&button_gallery(&divider_changed));
    assert_ne!(
        a.hash, c.hash,
        "a recoloured divider left the scene identical"
    );
}

#[test]
fn the_gallery_paints_the_requested_number_of_rects() {
    // Derived from the *resolved* properties rather than from each variant's
    // rest contract: a focus ring adds a border to variants that carry none at
    // rest, and a rule that ignores that comes out short.
    let theme = light();
    let samples = GallerySample::full_grid();
    let scene = button_gallery(&theme);
    let resolved = |s: &GallerySample| button_style(&theme, s.variant, s.size, s.state);
    let border_rects = samples
        .iter()
        .filter(|s| resolved(s).border_width.unwrap_or(0.0) > 0.0)
        .count();
    assert_eq!(scene.len(), 1 + samples.len() + border_rects);

    // A disabled raised button loses its shadow, which is the one place the
    // shadow count is not simply "every elevated sample". The shadows
    // themselves are asserted on separately: exactly the elevated cells carry
    // one, now on the fill node rather than as a rect of its own.
    let elevated = samples
        .iter()
        .filter(|s| resolved(s).elevation.unwrap_or(0.0) > 0.0)
        .count();
    let shadowed = scene
        .iter()
        .filter(|(_, node)| {
            matches!(
                node,
                SceneNode::Rect {
                    shadow: Some(_),
                    ..
                }
            )
        })
        .count();
    assert_eq!(shadowed, elevated, "every elevated cell carries its shadow");
    assert_eq!(elevated, 15, "18 raised samples minus 3 disabled");

    let size = gallery_size(&samples, GALLERY_COLUMNS);
    assert_eq!(size, Size::new(size.width, size.height));
    assert!(size.width > 0.0 && size.height > 0.0);
}

#[test]
fn every_declared_pair_clears_its_own_threshold() {
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let colors = HarmonyTheme::for_scheme(scheme).colors;
        for (name, fg, bg, threshold) in colors.contrast_pairs() {
            let ratio = contrast_ratio(fg, bg);
            assert!(
                ratio >= threshold,
                "{scheme:?} {name}: {ratio:.2}:1 is below the claimed {threshold:.1}:1"
            );
        }
    }
}
