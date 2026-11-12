// SPDX-License-Identifier: MIT
//! The progress kinds, from a TSX source to the Rust it becomes.
//!
//! The vocabulary grew a ring and a bar because the satellite ships the
//! circular indicator as its own component, and one generic kind cannot stand
//! for both. This walks the whole path: a TSX source is compiled to the IR,
//! the IR is emitted as Rust, and the Rust names the real widgets.

use floom_dsl::ui_ir::NodeKind;
use floom_dsl::{EmitOptions, WidgetSource};

/// The kinds a compiled document holds.
fn kinds(document: &floom_dsl::ui_ir::UiDocument) -> Vec<String> {
    document
        .nodes
        .iter()
        .map(|node| node.kind.name().to_string())
        .collect()
}

#[test]
fn a_tsx_source_compiles_the_progress_kinds() {
    let source = r#"
        function App() {
          return (
            <Stack gap={8}>
              <ProgressCircular value={0.5} size={32} />
              <ProgressCircular indeterminate={true} />
              <ProgressLinear value={50} />
            </Stack>
          );
        }
        // The canonical form: a named component, mounted.
    "#;
    let compiled = floom_tsx::compile(source).expect("the source compiles");
    let held = kinds(&compiled);
    assert!(
        held.contains(&"progress_circular".to_string()),
        "the document holds {held:?}"
    );
    assert!(
        held.contains(&"progress_linear".to_string()),
        "the document holds {held:?}"
    );
}

#[test]
fn the_progress_kinds_emit_the_real_widgets() {
    let source = r#"
        function App() {
          return (
            <Stack gap={8}>
              <ProgressCircular value={0.5} size={32} />
              <ProgressCircular indeterminate={true} />
              <ProgressLinear value={50} />
            </Stack>
          );
        }
        // The canonical form: a named component, mounted.
    "#;
    let compiled = floom_tsx::compile(source).expect("the source compiles");
    let emitted = floom_dsl::emit(
        &compiled,
        &EmitOptions {
            widgets: WidgetSource::WithComposites,
            ..EmitOptions::default()
        },
    )
    .expect("emits");

    assert!(
        emitted.code.contains("CircularProgress::determinate"),
        "the ring is determinate:\n{}",
        emitted.code
    );
    assert!(
        emitted.code.contains("CircularProgress::indeterminate"),
        "the spinning ring is indeterminate:\n{}",
        emitted.code
    );
    assert!(
        emitted.code.contains("LinearProgress::new"),
        "the bar is a linear progress:\n{}",
        emitted.code
    );
    // The bar takes a percentage and the ring a fraction; scaling the
    // normalised fraction on the way in is what makes `value={50}` a half-full
    // bar rather than a bar that overflows.
    assert!(
        emitted.code.contains("* 100.0"),
        "the bar scales the fraction to a percentage:\n{}",
        emitted.code
    );
}

/// The vocabulary's progress kinds are the satellite's two progress
/// components, not one kind standing for both.
#[test]
fn the_two_progress_kinds_are_distinct() {
    assert_ne!(
        NodeKind::parse("progress_circular"),
        NodeKind::parse("progress_linear"),
    );
    assert!(NodeKind::parse("progress_circular").is_some());
    assert!(NodeKind::parse("progress_linear").is_some());
}
