
use floom_design::{CELL_H, ControlSize, GAP, GallerySample, InteractionState, gallery_size};
use floom_material::{
    GALLERY_COLUMNS, MaterialTheme, button_gallery, button_gallery_labels, button_metrics,
    button_style,
};
use floom_scene::SceneNode;
use floom_snapshot::{
    PixelSnapshot, ReferencePixels, SceneSnapshot, assert_snapshot, snapshots_dir,
};
use floom_types::Color;

fn light() -> MaterialTheme {
    MaterialTheme::light()
}

fn dark() -> MaterialTheme {
    MaterialTheme::dark()
}

#[test]
fn the_light_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&light())).assert(snapshots_dir!(), "material_gallery_light");
}

#[test]
fn the_dark_scheme_gallery_matches_its_golden() {
    SceneSnapshot::of(&button_gallery(&dark())).assert(snapshots_dir!(), "material_gallery_dark");
}

#[test]
fn the_resolved_property_table_matches_its_golden() {
    // The readable half of the snapshot: a diff here names the variant and the
    // property that moved, where a scene hash only says "something changed".
    assert_snapshot(
        snapshots_dir!(),
        "material_labels_light",
        &button_gallery_labels(&light()),
    );
    assert_snapshot(
        snapshots_dir!(),
        "material_labels_dark",
        &button_gallery_labels(&dark()),
    );
}

#[test]
fn the_gallery_renders_to_the_expected_pixels() {
    // The pixel layer is the only one that proves the scene is *painterly*:
    // scene text says what was asked for, this says what came out.
    let theme = light();
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    PixelSnapshot::of(&button_gallery(&theme), None, size, 1.0)
        .assert(snapshots_dir!(), "material_gallery_pixels");
}

#[test]
fn the_pixel_hash_follows_the_scheme() {
    let size = gallery_size(&GallerySample::full_grid(), GALLERY_COLUMNS);
    let l = PixelSnapshot::of(&button_gallery(&light()), None, size, 1.0);
    let d = PixelSnapshot::of(&button_gallery(&dark()), None, size, 1.0);
    assert_ne!(
        l.hash, d.hash,
        "light and dark rendered identically — the palette is not reaching the painter"
    );
    assert_eq!((l.width, l.height), (d.width, d.height));
}

#[test]
fn the_scene_snapshot_moves_when_a_single_role_moves() {
    // Falsifiability. If this passes trivially the goldens above prove nothing.
    let base = light();
    let mut changed = light();
    changed.roles.primary = Color::rgb(0, 200, 0);
    let a = SceneSnapshot::of(&button_gallery(&base));
    let b = SceneSnapshot::of(&button_gallery(&changed));
    assert_ne!(
        a.hash, b.hash,
        "a recoloured primary left the scene identical"
    );

    // A change confined to the outline must move the scene too — otherwise the
    // outlined variant is not really being painted.
    let mut outline_changed = light();
    outline_changed.roles.outline = Color::rgb(255, 0, 255);
    let c = SceneSnapshot::of(&button_gallery(&outline_changed));
    assert_ne!(
        a.hash, c.hash,
        "a recoloured outline left the scene identical"
    );
}

#[test]
fn a_size_step_moves_the_snapshot() {
    // Sizes are the dimension most likely to be dropped by a table rewrite, and
    // the snapshot is where that shows.
    let theme = light();
    let small: Vec<GallerySample> =
        GallerySample::variants_at(ControlSize::Sm, InteractionState::Rest);
    let large: Vec<GallerySample> =
        GallerySample::variants_at(ControlSize::Lg, InteractionState::Rest);
    let render = |samples: &[GallerySample]| {
        SceneSnapshot::of(&floom_design::gallery_scene(
            samples,
            theme.roles.surface,
            samples.len(),
            |s| {
                floom_design::CellPaint::new(
                    button_style(&theme, s.variant, s.size, s.state),
                    button_metrics(&theme, s.size).height as f64,
                )
            },
        ))
    };
    assert_ne!(render(&small).hash, render(&large).hash, "size was ignored");
}

#[test]
fn the_gallery_paints_the_requested_number_of_rects() {
    // A cheap invariant that catches a broken grid before the goldens are read.
    // The two extras are counted *independently*: a focused elevated button has
    // both a ring and a shadow and therefore costs two extra rects, and folding
    // them into one counter is how this assertion first came out short.
    let theme = light();
    let samples = GallerySample::full_grid();
    let scene = button_gallery(&theme);
    let resolved = |s: &GallerySample| button_style(&theme, s.variant, s.size, s.state);
    let border_rects = samples
        .iter()
        .filter(|s| resolved(s).border_width.unwrap_or(0.0) > 0.0)
        .count();
    assert_eq!(scene.len(), 1 + samples.len() + border_rects);

    // A disabled raised button loses its shadow along with its colour, so the
    // shadow count sits strictly below the sample count for that variant. The
    // shadows themselves are asserted on separately: exactly the elevated
    // cells carry one, now on the fill node rather than as a rect of its own.
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
    assert_eq!(elevated, 15, "18 elevated samples minus 3 disabled");

    let size = gallery_size(&samples, GALLERY_COLUMNS);
    let rows = samples.len().div_ceil(GALLERY_COLUMNS) as f64;
    assert_eq!(size.height, rows * (CELL_H + GAP) - GAP);
    assert!(
        size.width > CELL_H,
        "a 15-column grid is wider than it is tall"
    );
}
