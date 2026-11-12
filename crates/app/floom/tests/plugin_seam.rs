// SPDX-License-Identifier: MIT
//! The plugin seam costs a frame nothing.
//!
//! "The registry is setup-only" is a claim about code that is not there: the
//! registry is a `Mutex<BTreeMap>` behind a table, and a host that asked it
//! something per frame would pay for the lock and the map every time. The
//! claim is worth a measurement, not an argument.
//!
//! The measurement is the registry's own access counter. A frame is drawn
//! with sixty-four plugins registered, and the counter is read before and
//! after: if the frame path consulted the registry, the count would move.
//! It does not move.
//!
//! What the registry *does* cost is setup, and that is measured too — the
//! enable path allocates, because it is the moment the capability claims its
//! platform state.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use floom::prelude::*;
use floom_platform::{register_plugin, table_accesses};

/// Counts allocations while the flag is on.
struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static COUNTING: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) == 1 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) == 1 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many allocations `body` makes.
fn allocations(body: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.load(Ordering::Relaxed);
    COUNTING.store(1, Ordering::Relaxed);
    body();
    COUNTING.store(0, Ordering::Relaxed);
    ALLOCATIONS.load(Ordering::Relaxed) - before
}

/// A host with a small tree in it.
fn host() -> WidgetHost {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(320.0, 120.0));
    let root = Stack::new(Axis::Vertical).child(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        || Box::new(Text::new("a frame")),
    ));
    host.mount_root(LayoutStyle::default(), root);
    host
}

/// Register `count` plugins, each with an enable function that does nothing.
///
/// The names carry `tag` so two tests can register their own without the
/// registry refusing the second as a duplicate.
fn register(tag: &str, count: usize) {
    for index in 0..count {
        register_plugin(&format!("seam-probe-{tag}-{index}"), || Ok(()));
    }
}

/// The frame path does not consult the plugin registry, and the seam's cost is
/// all at setup.
///
/// Sixty-four plugins are registered, ten frames are drawn, and the registry's
/// access counter is read either side. If the frame path asked the registry
/// anything — "is my camera enabled?", "what is registered?" — the count
/// would move. It does not: the seam is setup-only, and this is the
/// measurement rather than the argument.
///
/// The counter is process-wide, so both halves live in one test: two tests
/// running in parallel would each see the other's registrations and neither
/// measurement would mean anything.
#[test]
fn the_seam_costs_at_setup_and_not_per_frame() {
    let mut host = host();
    host.frame();

    // The setup: registering a plugin locks the table and allocates, because
    // the registry stores it and the capability claims its platform state.
    let before_setup = table_accesses();
    let setup = allocations(|| {
        register("setup", 64);
    });
    let after_setup = table_accesses();
    assert!(
        setup > 0,
        "registering a plugin allocates: the registry stores it"
    );
    assert!(
        after_setup > before_setup,
        "and it locks the table: the registry was written to"
    );

    // The frames: they must not consult the registry again.
    let before_frames = table_accesses();
    for _ in 0..10 {
        host.frame();
    }
    assert_eq!(
        before_frames,
        table_accesses(),
        "ten frames consulted the plugin registry: the seam is on the frame path"
    );
}
