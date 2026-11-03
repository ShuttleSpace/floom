//! Inline handler + signal-ref lowering (the missing piece the static demo needed).
//!
//! These tests exercise the compiler half only: TSX -> `UiDocument`. They assert
//! that an inline `onClick` arrow is translated to Rust and carried on the event,
//! and that a `createSignal` read in a prop lowers to a `Ref` binding. The runtime
//! re-render is the closure-capture seam and is verified separately on a GPU box.

use floom_ui_ir::{NodeKind, PropValue};
use floom_tsx::compile;

/// Find the first node of a kind, so a test can inspect its events/props.
fn find<'a>(doc: &'a floom_ui_ir::UiDocument, kind: NodeKind) -> &'a floom_ui_ir::UiNode {
    doc.nodes
        .iter()
        .find(|n| n.kind == kind)
        .expect("node of kind present")
}

#[test]
fn inline_click_handler_translates_to_rust() {
    let src = r#"
        const [count, setCount] = createSignal(0);
        <Stack>
            <Button label="Add" onClick={() => setCount(count() + 1)} />
        </Stack>
    "#;
    let doc = compile(src).expect("lower");

    // The signal is a binding (declared at module scope).
    assert!(doc.bindings.iter().any(|b| b.name == "count"));

    let btn = find(&doc, NodeKind::Button);
    assert_eq!(btn.events.len(), 1, "one click handler");
    let event = &btn.events[0];
    assert_eq!(event.event.name(), "click");
    assert_eq!(
        event.body.as_deref(),
        Some("count.set(count.get() + 1)"),
        "setter call rewritten to signal set; signal read rewritten to .get()"
    );
}

#[test]
fn inline_handler_with_block_body() {
    let src = r#"
        const [count, setCount] = createSignal(0);
        <Button onClick={() => { setCount(count() + 1); }} />
    "#;
    let doc = compile(src).expect("lower");
    let btn = find(&doc, NodeKind::Button);
    assert_eq!(
        btn.events[0].body.as_deref(),
        Some("count.set(count.get() + 1);"),
        "block-bodied arrow keeps the trailing semicolon"
    );
}

#[test]
fn signal_read_in_prop_lowers_to_ref() {
    let src = r#"
        const [msg, setMsg] = createSignal("hi");
        <Stack>
            <Text label={msg()} />
        </Stack>
    "#;
    let doc = compile(src).expect("lower");

    let text = find(&doc, NodeKind::Text);
    let label = text
        .props
        .iter()
        .find(|p| p.name == "label")
        .expect("label prop");
    assert!(
        matches!(label.value, PropValue::Ref(ref name) if name == "msg"),
        "label is a reactive ref to the signal"
    );
}

#[test]
fn inline_handler_rejects_unsupported_expression() {
    // A `new` expression is outside the supported subset, so it must be a
    // span-based diagnostic rather than a silently-wrong emission.
    let src = r#"
        const [count, setCount] = createSignal(0);
        <Button onClick={() => setCount(new Foo())} />
    "#;
    assert!(
        compile(src).is_err(),
        "unsupported expression in inline handler should fail"
    );
}
