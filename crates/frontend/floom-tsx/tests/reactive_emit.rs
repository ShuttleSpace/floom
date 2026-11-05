// SPDX-License-Identifier: MIT
//! Reactive emit: TSX signal props lowered and then emitted through
//! `floom-dsl`'s closure-capture seam (Task 6).
//!
//! The plan placed this in `crates/frontend/floom-dsl/tests/`, but `floom-dsl` cannot
//! dev-depend on `floom-tsx` (that is a dependency cycle, since `floom-tsx`
//! depends on `floom-dsl`). It lives here instead, which also exercises the
//! whole path: TSX -> UiDocument -> Rust builder program.

use floom_dsl::emit::{EmitOptions, emit};
use floom_tsx::compile;

#[test]
fn emit_reactive_prop_reads_signal_in_closure() {
    let doc = compile("<Stack><Text label={count()} /></Stack>").expect("lower");
    // `count()` is a signal reader, so it lowered to a value binding.
    assert!(doc.bindings.iter().any(|b| b.name == "count"));

    let opts = EmitOptions {
        reactive: true,
        ..EmitOptions::default()
    };
    let out = emit(&doc, &opts).expect("emit").code;

    // A signal is declared with a type-default initial value.
    assert!(
        out.contains("SyncBaseSignal::new"),
        "expected a signal declaration:\n{out}"
    );
    // It is read inside the per-child closure (the closure-capture seam).
    assert!(out.contains(".get()"), "expected a signal read:\n{out}");
    assert!(out.contains("move ||"), "expected a move closure:\n{out}");
    // The rejected `host.widget_mut` / `create_effect` seam must not appear.
    assert!(!out.contains("widget_mut"), "unexpected widget_mut:\n{out}");
    assert!(
        !out.contains("create_effect"),
        "unexpected create_effect:\n{out}"
    );
}

#[test]
fn emit_static_document_has_no_signals() {
    let doc = compile("<Stack><Text>hi</Text></Stack>").expect("lower");
    let out = emit(&doc, &EmitOptions::default()).expect("emit").code;
    assert!(
        !out.contains("SyncBaseSignal"),
        "static emit grew signals:\n{out}"
    );
    assert!(
        !out.contains("move ||"),
        "static emit grew move closures:\n{out}"
    );
}
