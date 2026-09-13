use floem_scene::{Color, DamageRegion, Rect, Scene, SceneNode};
use floem_types::{Point, Size};

#[test]
fn stable_id_survives_upsert() {
    let mut scene = Scene::new();
    scene.begin_update();
    let id = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        fill: Color::BLACK,
        radius: 0.0,
    });
    scene.finish_update();

    scene.begin_update();
    scene.upsert(
        id,
        SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(1.0, 1.0), Size::new(10.0, 10.0)),
            fill: Color::WHITE,
            radius: 0.0,
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
        fill: Color::BLACK,
        radius: 0.0,
    });
    scene.finish_update();

    scene.begin_update();
    scene.remove(id);
    scene.finish_update();
    assert!(!scene.contains(id));

    scene.begin_update();
    let id2 = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
        fill: Color::WHITE,
        radius: 0.0,
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
        fill: Color::BLACK,
        radius: 0.0,
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
            fill: Color::BLACK,
            radius: 0.0,
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
    let _ = scene.insert(SceneNode::Group {
        children: smallvec::smallvec![],
        transform: Default::default(),
    });
    scene.finish_update();
    assert!(scene.damage().is_full_frame());
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
            "floem-scene must not depend on {forbidden}"
        );
    }
}
