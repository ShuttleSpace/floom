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
    let src = r#"
        const [count, setCount] = createSignal(0);
        <Button onClick={() => setCount(new Foo())} />
    "#;
    assert!(
        compile(src).is_err(),
        "unsupported expression in inline handler should fail"
    );
}

#[test]
fn create_signal_seed_is_captured() {
    let src = r#"
        const [status, setStatus] = createSignal("hello world");
        const [progress, setProgress] = createSignal(64);
        const [on, setOn] = createSignal(true);
        <Stack>
            <Text label={status()} />
        </Stack>
    "#;
    let doc = compile(src).expect("lower");

    let status = doc
        .bindings
        .iter()
        .find(|b| b.name == "status")
        .expect("status binding");
    assert_eq!(
        status.init,
        Some(PropValue::Text("hello world".to_string())),
        "string initial value is seeded"
    );

    let progress = doc
        .bindings
        .iter()
        .find(|b| b.name == "progress")
        .expect("progress binding");
    assert_eq!(progress.init, Some(PropValue::Number(64.0)));

    let on = doc
        .bindings
        .iter()
        .find(|b| b.name == "on")
        .expect("on binding");
    assert_eq!(on.init, Some(PropValue::Bool(true)));
}
