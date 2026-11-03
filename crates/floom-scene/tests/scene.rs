use floom_scene::{Color, DamageRegion, Rect, Scene, SceneNode};
use floom_types::{Point, Size};

#[test]
fn stable_id_survives_upsert() {
    let mut scene = Scene::new();
    scene.begin_update();
    let id = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        fill: floom_types::Brush::from(Color::BLACK),
        radius: 0.0,

        shadow: None,
    });
    scene.finish_update();

    scene.begin_update();
    scene.upsert(
        id,
        SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(10.0, 10.0)),
            fill: floom_types::Brush::from(Color::WHITE),
            radius: 0.0,
            shadow: None,
        },
    );
    scene.finish_update();

    assert!(scene.contains(id));
    assert!(matches!(scene.get(id), Some(SceneNode::Rect { .. })));
}

#[test]
fn remove_invalidates_id_and_reuses_index() {
    let mut scene = Scene::new();
    scene.begin_update();
    let id = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(5.0, 5.0)),
        fill: floom_types::Brush::from(Color::BLACK),
        radius: 0.0,

        shadow: None,
    });
    scene.finish_update();

    scene.begin_update();
    scene.remove(id);
    scene.finish_update();
    assert!(!scene.contains(id));

    scene.begin_update();
    let id2 = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
        fill: floom_types::Brush::from(Color::WHITE),
        radius: 0.0,

        shadow: None,
    });
    scene.finish_update();
    assert_eq!(id2.index(), id.index());
    assert_ne!(id2.generation(), id.generation());
    assert!(!scene.contains(id));
}

#[test]
fn damage_tracks_rect_updates() {
    let mut scene = Scene::new();
    scene.begin_update();
    let id = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        fill: floom_types::Brush::from(Color::BLACK),
        radius: 0.0,

        shadow: None,
    });
    scene.finish_update();
    assert!(!scene.damage().is_empty());
    scene.clear_damage();
    assert!(scene.damage().is_empty());

    scene.begin_update();
    scene.upsert(
        id,
        SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(2.0, 2.0), Size::new(10.0, 10.0)),
            fill: floom_types::Brush::from(Color::BLACK),
            radius: 0.0,
            shadow: None,
        },
    );
    scene.finish_update();
    assert!(!scene.damage().is_full_frame());
    assert!(!scene.damage().rects().is_empty());
}

#[test]
fn group_update_marks_full_frame() {
    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::GroupPush {
        transform: Default::default(),
    });
    scene.finish_update();
    assert!(scene.damage().is_full_frame());
}

/// A shadow reaches past the box it belongs to, so the hint damage is computed
/// from has to contain the whole reach — a damage region built from the bare
/// box would clip the blur's outer edge, and a shadow that loses its edge
/// under partial repaints is worse than no shadow at all.
#[test]
fn a_shadowed_rects_damage_contains_its_reach() {
    let box_of = Rect::from_origin_size(Point::new(10.0, 10.0), Size::new(20.0, 20.0));
    let shadow = floom_types::Shadow {
        color: Color::rgba(0, 0, 0, 128),
        blur: 4.0,
        offset_x: 0.0,
        offset_y: 3.0,
        spread: 0.0,
    };
    let bare = SceneNode::Rect {
        bounds: box_of,
        fill: floom_types::Brush::from(Color::WHITE),
        radius: 0.0,
        shadow: None,
    };
    let shadowed = SceneNode::Rect {
        bounds: box_of,
        fill: floom_types::Brush::from(Color::WHITE),
        radius: 0.0,
        shadow: Some(shadow),
    };
    assert_eq!(bare.bounds_hint(), Some(box_of));

    // reach = furthest offset (3) + blur (4) = 7 px in every direction.
    let reach = 7.0;
    assert_eq!(
        shadowed.bounds_hint(),
        Some(Rect {
            x0: box_of.x0 - reach,
            y0: box_of.y0 - reach,
            x1: box_of.x1 + reach,
            y1: box_of.y1 + reach,
        })
    );
}

#[test]
fn damage_region_spills_to_full_frame() {
    let mut d = DamageRegion::new();
    for i in 0..10 {
        d.union_rect(Rect::from_origin_size(
            Point::new(i as f64, 0.0),
            Size::new(1.0, 1.0),
        ));
    }
    assert!(d.is_full_frame());
}

#[test]
fn crate_manifest_has_no_os_deps() {
    let manifest = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    for forbidden in ["winit", "wgpu", "objc", "android", "napi", "ash", "metal"] {
        assert!(
            !manifest.to_lowercase().contains(forbidden),
            "floom-scene must not depend on {forbidden}"
        );
    }
}
