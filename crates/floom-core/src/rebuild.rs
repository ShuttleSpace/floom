//! Reactive rebuild requests from an effect back into the host.
//!
//! `request_repaint` is paint-only by construction. A reactive *rebuild* is
//! different: when a signal an effect read flips, the builder-tree composite
//! that owns the closure holding that signal must re-roll its children —
//! re-running each child build closure (which reads the signal through
//! `.get()`) and swapping in the fresh widgets. That is the "closure-capture
//! seam": reactivity lives in the builder tree, not in a host lookup, but the
//! trigger has to cross from the effect back into the host, exactly as the
//! repaint queue does.

use std::cell::Cell;

thread_local! {
    static REBUILD_ROOT: Cell<bool> = const { Cell::new(false) };
}

/// Asks the host to re-roll the root composite's children on its next frame.
///
/// The host consumes this once per frame through [`take_rebuild_root`]; it is
/// coalesced into at most one rebuild.
pub fn request_rebuild_root() {
    REBUILD_ROOT.with(|flag| flag.set(true));
}

/// Takes the pending rebuild request, if any. Clears it.
pub fn take_rebuild_root() -> bool {
    REBUILD_ROOT.with(|flag| {
        let value = flag.get();
        flag.set(false);
        value
    })
}
