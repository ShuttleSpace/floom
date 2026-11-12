// SPDX-License-Identifier: MIT
//! A component module is an app.
//!
//! A TSX module can write its app as a bare JSX expression, as a default
//! export, or as a named component. The named form is what `compile`
//! documents — "`export function Counter() { … }`" — and it is what a
//! developer writes when the app has a name.
//!
//! It used to lower to an *empty* tree: the parser's root scan is shallow, so
//! a named component was invisible to it, and with no `mount` call and no
//! default export there was nothing else to be the root. The module compiled,
//! and rendered nothing — no error, no warning, just a blank app. This file
//! is the net under that.

use floom_dsl::ui_ir::NodeKind;

/// The kinds a compiled document holds.
fn kinds(document: &floom_dsl::ui_ir::UiDocument) -> Vec<String> {
    document
        .nodes
        .iter()
        .map(|node| node.kind.name().to_string())
        .collect()
}

/// Every way of writing the app keeps the tree.
///
/// Each of these is a form a developer writes. They all have to lower to the
/// same document, because from the author's side they are the same app.
#[test]
fn every_form_of_component_module_keeps_the_tree() {
    let forms = [
        // A bare JSX expression.
        r#"<Stack><Text label="x" /></Stack>"#,
        // A default export.
        r#"export default function App() { return <Stack><Text label="x" /></Stack>; }"#,
        // A named export — the form `compile` documents.
        r#"export function Counter() { return <Stack><Text label="x" /></Stack>; }"#,
        // A named component with no export at all.
        r#"function App() { return <Stack><Text label="x" /></Stack>; }"#,
        // An arrow component.
        r#"const App = () => <Stack><Text label="x" /></Stack>;"#,
        // The form the CLI's own scaffold writes.
        r#"import { mount } from "floom";
           function App() { return <Stack><Text label="x" /></Stack>; }
           mount(App);"#,
    ];
    for form in forms {
        let document =
            floom_tsx::compile(form).unwrap_or_else(|e| panic!("{form} must compile: {e}"));
        let held = kinds(&document);
        assert!(
            held.contains(&"text".to_string()) && held.contains(&"stack".to_string()),
            "{form} lowered to {held:?}: the tree was lost"
        );
    }
}

/// A component module with a named component is the root, not the empty one.
///
/// The specific regression: the document held a single synthetic node and the
/// app's own nodes were nowhere in it.
#[test]
fn a_named_component_module_is_not_an_empty_document() {
    let document = floom_tsx::compile(
        r#"export function Counter() {
             return (
               <Stack axis="vertical">
                 <Text label="count" />
                 <Button label="Add" />
               </Stack>
             );
           }"#,
    )
    .expect("the module compiles");
    let held = kinds(&document);
    assert_eq!(
        held.len(),
        3,
        "the stack, the text and the button: {held:?}"
    );
    assert!(held.contains(&"button".to_string()), "{held:?}");
    assert!(held.contains(&"text".to_string()), "{held:?}");
}

/// The fix does not steal the root from an explicit `mount`.
#[test]
fn an_explicit_mount_still_wins() {
    let document = floom_tsx::compile(
        r#"import { mount } from "floom";
           function Other() { return <Stack><Text label="other" /></Stack>; }
           function App() { return <Stack><Text label="app" /></Stack>; }
           mount(App);"#,
    )
    .expect("the module compiles");
    // The mounted component's text, and not the other one's.
    let labels: Vec<&str> = document
        .nodes
        .iter()
        .filter_map(|node| {
            node.props.iter().find_map(|prop| match &prop.value {
                floom_dsl::ui_ir::PropValue::Text(text) if text == "app" || text == "other" => {
                    Some(text.as_str())
                }
                _ => None,
            })
        })
        .collect();
    assert_eq!(labels, vec!["app"], "the mounted component is the root");
}

/// The kinds the vocabulary can name are the kinds the parser accepts.
#[test]
fn the_parser_accepts_the_kind_the_document_holds() {
    let document = floom_tsx::compile(r#"<Stack><Text label="x" /></Stack>"#).expect("compiles");
    for node in &document.nodes {
        assert!(
            NodeKind::parse(node.kind.name()).is_some(),
            "{} is in the document but not in the vocabulary",
            node.kind.name()
        );
    }
}
