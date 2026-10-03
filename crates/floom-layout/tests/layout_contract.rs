use floom_layout::{FlexDirection, JustifyContent, LayoutEngine, LayoutStyle, MeasureFn};
use floom_types::{Point, Size};

#[test]
fn flex_row_places_children() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle {
        width: Some(40.0),
        height: Some(20.0),
        ..Default::default()
    });
    let b = eng.new_leaf(LayoutStyle {
        width: Some(60.0),
        height: Some(20.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Row,
            width: Some(200.0),
            height: Some(40.0),
            gap: 10.0,
            ..Default::default()
        },
        &[a, b],
    );

    eng.compute(root, Size::new(200.0, 40.0));
    let la = eng.layout(a).rect;
    let lb = eng.layout(b).rect;
    assert!((la.x0 - 0.0).abs() < 0.01);
    assert!((lb.x0 - 50.0).abs() < 0.01); // 40 + gap 10
    assert!((la.width() - 40.0).abs() < 0.01);
}

#[test]
fn column_stack_and_center() {
    let mut eng = LayoutEngine::new();
    let child = eng.new_leaf(LayoutStyle {
        width: Some(30.0),
        height: Some(10.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            width: Some(100.0),
            height: Some(100.0),
            ..Default::default()
        },
        &[child],
    );
    eng.compute(root, Size::new(100.0, 100.0));
    let c = eng.layout(child).rect;
    assert!((c.y0 - 45.0).abs() < 0.5, "y0={}", c.y0);
}

#[test]
fn measure_fn_leaf() {
    let mut eng = LayoutEngine::new();
    let measure: MeasureFn = Box::new(|_avail| Size::new(55.0, 33.0));
    let leaf = eng.new_leaf_with_measure(LayoutStyle::default(), measure);
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(200.0),
            height: Some(100.0),
            align_items: floom_layout::AlignItems::Start,
            ..Default::default()
        },
        &[leaf],
    );
    eng.compute(root, Size::new(200.0, 100.0));
    let r = eng.layout(leaf).rect;
    assert!((r.width() - 55.0).abs() < 0.01, "w={}", r.width());
    assert!((r.height() - 33.0).abs() < 0.01, "h={}", r.height());
}

#[test]
fn public_api_exports_opaque_layout_node_only() {
    let src = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"));
    assert!(src.contains("LayoutNode"));
    assert!(!src.contains("pub use taffy::"));
    assert!(!src.contains("NodeId"));
}

#[test]
fn absolute_child_escapes_parent_flow() {
    let mut eng = LayoutEngine::new();
    // In-flow sibling that would otherwise sit beside the overlay.
    let sibling = eng.new_leaf(LayoutStyle {
        width: Some(40.0),
        height: Some(20.0),
        ..Default::default()
    });
    // Overlay anchored to all four sides of the containing block.
    let overlay = eng.new_leaf(LayoutStyle {
        position: floom_layout::Position::Absolute,
        inset: floom_layout::AbsoluteInset::fill(),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(200.0),
            height: Some(100.0),
            ..Default::default()
        },
        &[sibling, overlay],
    );
    eng.compute(root, Size::new(200.0, 100.0));

    let sibling_rect = eng.layout(sibling).rect;
    assert_eq!(sibling_rect.x0, 0.0, "in-flow child still starts at 0");
    assert_eq!(sibling_rect.width(), 40.0);

    let r = eng.layout(overlay).rect;
    assert_eq!(r.origin(), Point::ZERO);
    assert_eq!(r.size(), Size::new(200.0, 100.0));
}

#[test]
fn absolute_child_uses_parent_alignment_as_static_position() {
    let mut eng = LayoutEngine::new();
    let panel = eng.new_leaf(LayoutStyle {
        width: Some(120.0),
        height: Some(60.0),
        position: floom_layout::Position::Absolute,
        ..Default::default()
    });
    // With every inset `auto` the box falls back to its static position, which
    // CSS defines by the parent's alignment — not by centring on its own.
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(200.0),
            height: Some(100.0),
            justify_content: JustifyContent::Center,
            align_items: floom_layout::AlignItems::Center,
            ..Default::default()
        },
        &[panel],
    );
    eng.compute(root, Size::new(200.0, 100.0));

    let r = eng.layout(panel).rect;
    assert!((r.x0 - 40.0).abs() < 0.5, "x0={}", r.x0);
    assert!((r.y0 - 20.0).abs() < 0.5, "y0={}", r.y0);
}

#[test]
fn absolute_inset_pins_to_one_edge() {
    let mut eng = LayoutEngine::new();
    // A bottom-anchored sheet: pinned down, stretched sideways.
    let sheet = eng.new_leaf(LayoutStyle {
        height: Some(50.0),
        position: floom_layout::Position::Absolute,
        inset: floom_layout::AbsoluteInset {
            left: Some(0.0),
            right: Some(0.0),
            bottom: Some(0.0),
            top: None,
        },
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(300.0),
            height: Some(200.0),
            ..Default::default()
        },
        &[sheet],
    );
    eng.compute(root, Size::new(300.0, 200.0));

    let r = eng.layout(sheet).rect;
    assert_eq!(r.width(), 300.0);
    assert_eq!(r.height(), 50.0);
    assert!(
        (r.y0 - 150.0).abs() < 0.5,
        "sheet must sit at the bottom: y0={}",
        r.y0
    );
}

#[test]
fn removing_a_child_updates_the_flow() {
    let mut eng = LayoutEngine::new();
    let keep = eng.new_leaf(LayoutStyle {
        width: Some(40.0),
        height: Some(20.0),
        ..Default::default()
    });
    let drop = eng.new_leaf(LayoutStyle {
        width: Some(40.0),
        height: Some(20.0),
        ..Default::default()
    });
    let root = eng.new_with_children(
        LayoutStyle {
            width: Some(200.0),
            height: Some(100.0),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        },
        &[keep, drop],
    );
    eng.compute(root, Size::new(200.0, 100.0));
    assert!((eng.layout(drop).rect.y0 - 20.0).abs() < 0.5);

    eng.remove_child(root, drop);
    eng.remove(drop);
    eng.compute(root, Size::new(200.0, 100.0));

    assert!(
        !eng.contains(drop),
        "removed handles are stale, not dangling"
    );
    assert_eq!(eng.node_count(), 2, "root and the surviving child remain");

    // The remaining child still lays out as the only row item.
    assert_eq!(eng.layout(keep).rect.y0, 0.0);
}

#[test]
fn insert_child_at_index_preserves_order() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle {
        width: Some(10.0),
        height: Some(10.0),
        ..Default::default()
    });
    let b = eng.new_leaf(LayoutStyle {
        width: Some(10.0),
        height: Some(10.0),
        ..Default::default()
    });
    let c = eng.new_leaf(LayoutStyle {
        width: Some(10.0),
        height: Some(10.0),
        ..Default::default()
    });
    let root = eng.new_with_children(LayoutStyle::column(), &[a, b]);
    // `c` goes in the middle rather than after `b`.
    eng.insert_child_at_index(root, 1, c);
    eng.compute(root, Size::new(200.0, 100.0));

    let expected = [(a, 0.0), (c, 10.0), (b, 20.0)];
    for (node, y) in expected {
        let rect = eng.layout(node).rect;
        assert!((rect.y0 - y).abs() < 0.5, "y0={} expected {}", rect.y0, y);
    }
}

#[test]
fn removed_slots_are_reused() {
    let mut eng = LayoutEngine::new();
    let a = eng.new_leaf(LayoutStyle::default());
    let before = eng.node_count();
    eng.remove(a);
    let b = eng.new_leaf(LayoutStyle::default());
    assert_eq!(b, a, "freed slot comes back first");
    // Reusing the slot keeps the live count bounded instead of growing forever.
    assert_eq!(eng.node_count(), before);
    assert!(eng.contains(b));
}

/// Measure nodes must re-measure under the space their parent actually gave
/// them — not the root viewport captured once at compute time. Shrinking the
/// fixed box that holds a text-like leaf must re-run the measure fn with the
/// tighter width so the leaf can wrap (grow taller).
#[test]
fn measure_nodes_remeasure_under_per_node_constraints() {
    use std::sync::Arc;
    use std::sync::Mutex;

    let mut eng = LayoutEngine::new();
    let seen = Arc::new(Mutex::new(f64::NAN));

    let measure_seen = seen.clone();
    let measure: MeasureFn = Box::new(move |avail| {
        *measure_seen.lock().unwrap() = avail.width;
        // Fake "text": single line 100x20, wrapping to 60 tall under 50 wide.
        let width = avail.width.min(100.0);
        let height = if avail.width < 50.0 { 60.0 } else { 20.0 };
        Size::new(width, height)
    });

    let text = eng.new_leaf_with_measure(LayoutStyle::default(), measure);
    let holder = eng.new_with_children(
        LayoutStyle {
            width: Some(100.0),
            height: Some(100.0),
            ..LayoutStyle::column()
        },
        &[text],
    );
    let root = eng.new_with_children(LayoutStyle::default(), &[holder]);

    eng.compute(root, Size::new(200.0, 200.0));
    let wide = eng.layout(text).rect;
    assert!(
        (wide.height() - 20.0).abs() < 0.5,
        "single line in a 100-wide box, got {}",
        wide.height()
    );
    assert!(
        (*seen.lock().unwrap() - 100.0).abs() < 0.5,
        "measure saw the holder's box"
    );

    // Shrink the holder: same measure fn, tighter constraint, taller leaf.
    eng.set_style(
        holder,
        LayoutStyle {
            width: Some(40.0),
            height: Some(100.0),
            ..LayoutStyle::column()
        },
    );
    eng.compute(root, Size::new(200.0, 200.0));
    let narrow = eng.layout(text).rect;
    assert!(
        (narrow.height() - 60.0).abs() < 0.5,
        "wrapped in the 40-wide box, got {}",
        narrow.height()
    );
    assert!(
        (*seen.lock().unwrap() - 40.0).abs() < 0.5,
        "measure saw the new constraint"
    );
}
