// SPDX-License-Identifier: MIT
//! `.d.ts` generation from the IR schema (Task 8).

use floom_tsx::compile;

#[test]
fn decl_generates_complete_types() {
    // A single root with the kinds whose interfaces should be emitted.
    let source = r#"<Stack><Text label="hi" /><Button label="ok" /></Stack>"#;
    let doc = compile(source).expect("compile");
    let dts = floom_tsx::generate_dts(&doc);

    assert!(dts.contains("Text"), "missing Text interface:\n{dts}");
    assert!(dts.contains("Stack"), "missing Stack interface:\n{dts}");
    assert!(dts.contains("Button"), "missing Button interface:\n{dts}");
    assert!(
        dts.contains("interface") || dts.contains("type "),
        "no interface/type declarations:\n{dts}"
    );
    // The reactive shim declarations are exposed too.
    assert!(dts.contains("createSignal"), "missing shim decls:\n{dts}");
    assert!(dts.contains("onClick"), "missing event handler:\n{dts}");
}
