//! `class="…"` token resolution into IR style props (Task 9).
//!
//! The IR has no generic `style` prop — it has flat style properties like
//! `foreground`/`background` — so a token resolves to those.

use floom_tsx::compile;

#[test]
fn class_resolves_token() {
    let source = r#"<Text class="text-primary" />"#;
    let doc = compile(source).expect("lower");
    let node = doc.node(doc.root.as_str()).expect("root");
    let prop = node.prop("foreground");
    assert!(
        prop.is_some(),
        "`text-primary` should resolve to a foreground colour; props: {:?}",
        node.props
    );
}

#[test]
fn explicit_prop_wins_over_class() {
    // An explicit `foreground` must not be duplicated/overridden by a token.
    let source = r##"<Text class="text-primary" foreground="#123456" />"##;
    let doc = compile(source).expect("lower");
    let node = doc.node(doc.root.as_str()).expect("root");
    let foregrounds: Vec<_> = node.props.iter().filter(|p| p.name == "foreground").collect();
    assert_eq!(foregrounds.len(), 1, "foreground should appear once");
    assert_eq!(
        foregrounds[0].value,
        floom_ui_ir::PropValue::Color(floom_ui_ir::Rgba::opaque(0x12, 0x34, 0x56))
    );
}
