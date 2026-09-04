use floem_layout::{
    FlexDirection, JustifyContent, LayoutEngine, LayoutStyle, MeasureFn,
};
use floem_types::Size;

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
    let root = eng.new_with_children(LayoutStyle {
        width: Some(200.0),
        height: Some(100.0),
        align_items: floem_layout::AlignItems::Start,
        ..Default::default()
    }, &[leaf]);
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
