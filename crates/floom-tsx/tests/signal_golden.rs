//! Golden tests for the signal-shim translation (Task 5).
//!
//! The TSX reactive shim (`createSignal`/`createMemo`/`createEffect`/
//! `provideContext`/`useContext`) is recognized at compile time and turned into
//! `floom_ui_ir::Binding`s.

use floom_tsx::compile;

#[test]
fn lower_translates_create_signal() {
    let source = r#"const [count, setCount] = createSignal(0);"#;
    let doc = compile(source).expect("lower");
    assert!(
        doc.bindings
            .iter()
            .any(|b| b.name == "count" && matches!(b.kind, floom_ui_ir::BindingKind::Value(_)))
    );
}

#[test]
fn lower_translates_create_memo() {
    let source = r#"const double = createMemo(() => count() * 2);"#;
    let doc = compile(source).expect("lower");
    assert!(doc.bindings.iter().any(|b| b.name == "double"));
}

#[test]
fn lower_translates_create_effect() {
    let source = r#"createEffect(() => { console.log(count()); });"#;
    let doc = compile(source).expect("lower");
    // createEffect produces no binding; verify it lowered without error.
    assert!(!doc.nodes.is_empty());
}

#[test]
fn lower_translates_provide_context() {
    let source = r#"provideContext("theme", "dark");"#;
    let doc = compile(source).expect("lower");
    assert!(doc.bindings.iter().any(|b| b.name == "theme"));
}

#[test]
fn lower_translates_use_context() {
    let source = r#"const t = useContext("theme"); <Text label={t} />"#;
    let doc = compile(source).expect("lower");
    // useContext creates a binding named 't' with the context value.
    assert!(doc.bindings.iter().any(|b| b.name == "t"));
}
