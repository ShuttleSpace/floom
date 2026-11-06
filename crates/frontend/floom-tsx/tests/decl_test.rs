// SPDX-License-Identifier: MIT
//! `.d.ts` generation from the IR schema (Task 8).

use floom_tsx::generate_dts;

#[test]
fn decl_exposes_module_and_jsx_namespaces() {
    let dts = generate_dts();
    assert!(dts.contains("declare module \"floom\""));
    assert!(dts.contains("declare namespace JSX"));
    assert!(dts.contains("interface IntrinsicElements"));
}

#[test]
fn decl_generates_complete_types() {
    let dts = generate_dts();

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
