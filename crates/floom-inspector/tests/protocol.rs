//! The inspection protocol's contract.
//!
//! The protocol is the only thing a devtools frontend and a headless backend
//! share, so its rules are the ones that matter: snapshots are versioned with
//! *concrete* compatibility semantics, they round-trip through JSON, and an
//! inconsistent tree is rejected rather than served.

use floem_inspector::{
    AllocSummary, CacheStats, FrameSummary, JsonSnapshot, LayoutDto, LayoutEntry,
    PerformanceSnapshot, ProtocolError, ProtocolVersion, Rgba, SceneLines, SourceLocation,
    StageSpan, StyleEntry, StylePropsDto, TreeNodeInfo, TreeSnapshot,
};

fn sample_tree() -> TreeSnapshot {
    TreeSnapshot {
        version: ProtocolVersion::current(),
        root: 1,
        nodes: vec![
            TreeNodeInfo {
                id: 1,
                kind: "host".into(),
                key: Some("app.root".into()),
                source: Some(SourceLocation {
                    file: "ui/app.dsl".into(),
                    line: 2,
                    column: 1,
                }),
                parent: None,
                children: vec![2, 3],
            },
            TreeNodeInfo {
                id: 2,
                kind: "primitive".into(),
                key: Some("app.title".into()),
                source: Some(SourceLocation {
                    file: "ui/app.dsl".into(),
                    line: 3,
                    column: 5,
                }),
                parent: Some(1),
                children: vec![],
            },
            TreeNodeInfo {
                id: 3,
                kind: "composition".into(),
                key: None,
                source: None,
                parent: Some(1),
                children: vec![],
            },
        ],
    }
}

#[test]
fn the_protocol_version_has_concrete_compatibility_rules() {
    let reader = ProtocolVersion::new(1, 2);
    // Same major, older minor: loadable.
    assert!(reader.accepts(ProtocolVersion::new(1, 1)));
    assert!(reader.accepts(ProtocolVersion::new(1, 2)));
    // Same major, NEWER minor: refused. The reader cannot faithfully represent
    // fields it has never heard of, so serving the snapshot would be a lie.
    let err = reader
        .accepts_err(ProtocolVersion::new(1, 3))
        .expect_err("a newer minor must be refused");
    assert!(err.to_string().contains("newer"), "{err}");
    // Different major: refused in both directions.
    assert!(reader.accepts_err(ProtocolVersion::new(2, 0)).is_err());
    assert!(reader.accepts_err(ProtocolVersion::new(0, 9)).is_err());
}

#[test]
fn a_tree_snapshot_round_trips_through_json() {
    let tree = sample_tree();
    let json = tree.to_json().expect("serializes");
    let back = TreeSnapshot::from_json(&json).expect("parses");
    assert_eq!(back, tree);
}

#[test]
fn every_snapshot_kind_round_trips() {
    let style = floem_inspector::StyleSnapshot {
        version: ProtocolVersion::current(),
        entries: vec![StyleEntry {
            id: 2,
            state: "normal".into(),
            style: StylePropsDto {
                background: Some(Rgba::new(17, 34, 51, 255)),
                foreground: None,
                opacity: Some(0.7),
                font_size: Some(16.0),
                padding: None,
                radius: Some(8.0),
                border_width: Some(1.0),
                border_color: Some(Rgba::new(136, 136, 136, 255)),
                elevation: Some(2.0),
            },
        }],
    };
    let layout = floem_inspector::LayoutSnapshot {
        version: ProtocolVersion::current(),
        entries: vec![LayoutEntry {
            id: 2,
            layout: LayoutDto {
                width: Some(100.0),
                height: None,
                flex_grow: Some(1.0),
                axis: Some("vertical".into()),
                gap: Some(8.0),
                main_align: None,
                cross_align: None,
                position: None,
                inset: None,
            },
        }],
    };
    let scene = SceneLines {
        version: ProtocolVersion::current(),
        lines: vec!["Rect { bounds: 0..100, .. }".into()],
    };
    let performance = PerformanceSnapshot {
        version: ProtocolVersion::current(),
        frames: vec![FrameSummary {
            index: 0,
            budget_exceeded: false,
            stages: vec![
                StageSpan {
                    stage: "layout".into(),
                    duration_ns: 1_500,
                },
                StageSpan {
                    stage: "paint".into(),
                    duration_ns: 900,
                },
            ],
        }],
        allocations: AllocSummary {
            allocations: 12,
            deallocations: 10,
            bytes_allocated: 4096,
        },
        caches: vec![CacheStats {
            name: "glyph".into(),
            entries: 42,
            hits: 100,
            misses: 3,
            bytes: 2048,
        }],
    };

    for json in [
        style.to_json().expect("style"),
        layout.to_json().expect("layout"),
        scene.to_json().expect("scene"),
        performance.to_json().expect("performance"),
    ] {
        assert!(
            serde_json::from_str::<serde_json::Value>(&json).is_ok(),
            "{json}"
        );
    }
    assert_eq!(
        floem_inspector::StyleSnapshot::from_json(&style.to_json().unwrap()).unwrap(),
        style
    );
    assert_eq!(
        floem_inspector::LayoutSnapshot::from_json(&layout.to_json().unwrap()).unwrap(),
        layout
    );
    assert_eq!(
        floem_inspector::SceneLines::from_json(&scene.to_json().unwrap()).unwrap(),
        scene
    );
    assert_eq!(
        PerformanceSnapshot::from_json(&performance.to_json().unwrap()).unwrap(),
        performance
    );
}

#[test]
fn a_tree_with_a_dangling_parent_is_rejected() {
    let mut tree = sample_tree();
    tree.nodes[0].parent = Some(99);
    let err = tree.validate().expect_err("dangling parent");
    assert!(err.to_string().contains("parent"), "{err}");
}

#[test]
fn a_tree_with_a_duplicate_id_is_rejected() {
    let mut tree = sample_tree();
    let mut twin = tree.nodes[1].clone();
    twin.id = tree.nodes[0].id;
    tree.nodes.push(twin);
    assert!(tree.validate().is_err());
}

#[test]
fn a_tree_with_a_cycle_is_rejected() {
    // Two nodes that are each other's parent: walking to the root never ends.
    let mut tree = sample_tree();
    tree.nodes[1].parent = Some(3);
    tree.nodes[2].parent = Some(2);
    assert!(tree.validate().is_err());
}

#[test]
fn a_child_that_points_outside_the_node_set_is_rejected() {
    let mut tree = sample_tree();
    tree.nodes[0].children.push(77);
    let err = tree.validate().expect_err("dangling child");
    assert!(err.to_string().contains("child"), "{err}");
}

#[test]
fn the_root_must_be_one_of_the_nodes() {
    let mut tree = sample_tree();
    tree.root = 42;
    assert!(matches!(
        tree.validate(),
        Err(ProtocolError::MissingRoot { .. })
    ));
}

#[test]
fn style_state_names_are_a_closed_set() {
    assert!(floem_inspector::is_known_style_state("normal"));
    assert!(floem_inspector::is_known_style_state("hover"));
    assert!(floem_inspector::is_known_style_state("pressed"));
    assert!(floem_inspector::is_known_style_state("focused"));
    assert!(floem_inspector::is_known_style_state("selected"));
    assert!(floem_inspector::is_known_style_state("disabled"));
    assert!(!floem_inspector::is_known_style_state("shimmering"));
}

#[test]
fn a_tree_snapshot_refuses_a_foreign_version() {
    let mut tree = sample_tree();
    let current = ProtocolVersion::current();
    tree.version = ProtocolVersion::new(current.major, current.minor + 1);
    let err = tree.validate().expect_err("foreign version");
    assert!(matches!(err, ProtocolError::UnsupportedVersion { .. }));
}
