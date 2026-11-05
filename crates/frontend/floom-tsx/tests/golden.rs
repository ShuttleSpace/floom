// SPDX-License-Identifier: MIT
//! Golden tests for the TSX -> UiDocument lowering (P0: Tasks 1-4).
//!
//! These assert the *shape* of the lowered `UiDocument`, not its exact byte
//! layout, so they survive harmless formatting changes in the IR.

use floom_tsx::compile;
use floom_dsl::ui_ir::{EventName, NodeKind, PropValue, UiDocument};

fn doc(source: &str) -> UiDocument {
    compile(source).expect("compile should succeed")
}

#[test]
fn tsx_to_uidoc_static() {
    let d = doc("<Stack><Text>hello</Text></Stack>");
    // Root Stack + its Text child (the text run folds into the label).
    assert_eq!(d.nodes.len(), 2, "expected a Stack and a Text");
}

#[test]
fn lower_stack_text_folds_label() {
    let d = doc("<Stack><Text>hello</Text></Stack>");
    let root = d.node(d.root.as_str()).expect("root exists");
    assert_eq!(root.kind, NodeKind::Stack);
    let children: Vec<_> = root.children().collect();
    assert_eq!(children.len(), 1, "Stack holds one child");
    let text = d.node(children[0].as_str()).expect("text child exists");
    assert_eq!(text.kind, NodeKind::Text);
    let label = text.prop("label").expect("Text has a label");
    assert_eq!(label, &PropValue::Text("hello".into()));
}

#[test]
fn lower_slot_children() {
    let d = doc("<Stack><Button/><Text/></Stack>");
    let stack = d.node("n0").expect("root exists");
    let children: Vec<_> = stack.children().collect();
    assert_eq!(children.len(), 2, "Stack holds two children");
    assert_eq!(children[0].as_str(), "n1");
    assert_eq!(children[1].as_str(), "n2");
}

#[test]
fn lower_event_binding() {
    let d = doc("<Button onClick={h}/>");
    assert_eq!(d.nodes.len(), 1);
    let btn = &d.nodes[0];
    assert_eq!(btn.kind, NodeKind::Button);
    assert_eq!(btn.events.len(), 1);
    assert_eq!(btn.events[0].event, EventName::Click);
    assert_eq!(btn.events[0].handler, "h");
    // The handler must be registered so validation resolves it.
    assert!(d.bindings.iter().any(|b| b.name == "h"));
}

#[test]
fn lower_conditional() {
    // A bare ternary is not a valid TSX root; wrap it in a fragment, as a
    // component's `return` would.
    let d = doc("<>{cond ? <Text/> : <Button/>}</>");
    // Root fragment wraps a Stack; the conditional lives underneath.
    let root = d.node(d.root.as_str()).expect("root exists");
    assert_eq!(root.kind, NodeKind::Stack);
    let nested: Vec<_> = root.children().collect();
    assert_eq!(nested.len(), 1, "fragment holds one child");
    let cond = d.node(nested[0].as_str()).expect("conditional exists");
    assert_eq!(cond.kind, NodeKind::Conditional);
    assert!(d.nodes.len() >= 3, "stack + conditional + shown child");
    // `cond` is referenced as a boolean signal.
    assert!(d.bindings.iter().any(|b| b.name == "cond"));
}

#[test]
fn lower_full_nodekind_coverage() {
    // Every kind in the vocabulary must lower through the schema. `NodeKind::parse`
    // normalizes case and separators, so the snake_case `name()` spelling is a
    // valid TSX tag; native kinds use their variant name. Expression-child kinds
    // (Conditional) are emitted as `{expr}`, not `<expr/>`.
    for kind in NodeKind::ALL {
        let child: String = match kind {
            NodeKind::Conditional => "{c ? <Text/> : <Button/>}".to_string(),
            NodeKind::Native(n) => match n {
                floom_dsl::ui_ir::NativeKind::Web => "<Web/>".to_string(),
                floom_dsl::ui_ir::NativeKind::Video => "<Video/>".to_string(),
                floom_dsl::ui_ir::NativeKind::Map => "<Map/>".to_string(),
                floom_dsl::ui_ir::NativeKind::TextInput => "<TextInput/>".to_string(),
            },
            other => format!("<{}/>", other.name()),
        };
        let source = format!("<Stack>{child}</Stack>");
        let d = compile(&source).unwrap_or_else(|e| panic!("lower {kind:?} ({child}) failed: {e}"));
        // Stack (root) + the covered child (Conditional adds its shown child too).
        let root = d.node(d.root.as_str()).expect("root exists");
        assert_eq!(
            root.kind,
            NodeKind::Stack,
            "{kind:?} ({child}) root is Stack"
        );
        assert!(
            d.nodes.len() >= 2,
            "{kind:?} ({child}) produced too few nodes"
        );
    }
}

#[test]
fn rejects_unknown_prop() {
    // `not_a_prop` is not in the schema, so validation must reject it.
    let result = compile("<Text not_a_prop=\"x\"/>");
    assert!(result.is_err(), "unknown prop should fail validation");
}

#[test]
fn bare_expression_is_wrapped() {
    // A bare JSX expression (no export) still compiles via the default-export wrap.
    let d = doc("<Stack axis=\"vertical\"><Text>hi</Text></Stack>");
    let root = d.node(d.root.as_str()).expect("root exists");
    assert_eq!(root.kind, NodeKind::Stack);
    let axis = root.prop("axis").expect("axis prop present");
    assert_eq!(axis, &PropValue::Ident("vertical".into()));
}
