//! Phase 28.1 — allocation audit: a settled frame must not allocate per node.
//!
//! A counting global allocator (thread-local, so parallel tests don't pollute
//! each other) wraps the scenarios the plan names: 1k/10k/100k element trees.
//! The falsifiable claim is about *cost*, not about "it runs": a frame on a
//! tree that has stopped changing must allocate a bounded number of times —
//! a count that does not grow with `nodes.len()` — because every per-node
//! allocation on that path is work repeated for a pixel that did not change.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::Ordering::Relaxed;

use floem_animation::{AnimKind, CompositorProp, Tween};
use floem_app::{NodeId, WidgetHost};
use floem_layout::LayoutStyle;
use floem_types::Size;
use floem_widgets::{Container, Text};

thread_local! {
    /// `const` initializers: touching these from inside `alloc` can never
    /// allocate (lazy TLS setup would recurse through the allocator itself).
    static MEASURING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<u64> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // `try_with` (not `with`): during thread teardown the TLS may already
        // be gone, and an allocator that panics there takes the process down.
        if MEASURING.try_with(|m| m.get()).unwrap_or(false) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        if MEASURING.try_with(|m| m.get()).unwrap_or(false) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if MEASURING.try_with(|m| m.get()).unwrap_or(false) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Runs `f` with this thread's allocation counter armed; returns the number of
/// allocations `f` performed (transients included — that is the point).
fn measure<T>(f: impl FnOnce() -> T) -> (u64, T) {
    MEASURING.with(|m| m.set(true));
    COUNT.with(|c| c.set(0));
    let out = f();
    MEASURING.with(|m| m.set(false));
    (COUNT.with(|c| c.get()), out)
}

/// A wide static tree: one root container plus `groups` children, each of
/// which holds `leaves` text leaves. All leaves share one string, so shaping
/// and glyph resources are cached tree-wide and cannot masquerade as the
/// allocation signal under test. Returns the host and the first leaf's id.
fn wide_tree(groups: usize, leaves: usize) -> (WidgetHost, NodeId) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(800.0, 600.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let mut first_leaf = None;
    for _ in 0..groups {
        let group = host.mount(root, LayoutStyle::default(), Container::new());
        for _ in 0..leaves {
            let leaf = host.mount(group, LayoutStyle::default(), Text::new("leaf"));
            first_leaf.get_or_insert(leaf);
        }
    }
    (host, first_leaf.expect("at least one leaf"))
}

/// The allocation cost of one frame that changes nothing must not grow with
/// the size of the tree. Every per-node allocation on the settled path is a
/// hidden per-node cost — the thing this phase's audit exists to surface.
#[test]
fn a_settled_frame_does_not_allocate_per_node() {
    // Control: the *first* frame has real work (mount-time scene writes) and
    // is allowed to allocate — this guard keeps the assertion from passing
    // because frames simply do nothing at all.
    let (mut host, _) = wide_tree(10, 10);
    let (first_frame_allocs, _) = measure(|| host.frame());
    assert!(
        first_frame_allocs > 0,
        "control: building a fresh tree's first frame does allocate"
    );

    // Settle: a second frame consumes the cascade and leaves nothing dirty.
    host.frame();

    for &(groups, leaves) in &[(100usize, 10usize), (1_000, 10), (10_000, 10)] {
        let nodes = 1 + groups * (1 + leaves);
        let (mut host, _) = wide_tree(groups, leaves);
        host.frame();
        host.frame();
        let (allocs, stats) = measure(|| host.frame());
        assert_eq!(
            stats.updated, 0,
            "{nodes}-node tree should be settled before the measured frame"
        );
        assert!(
            allocs <= 8,
            "a settled frame on a {nodes}-node tree allocated {allocs} times; \
             per-node allocation on the static path is a Phase 28.1 regression"
        );
    }
}

/// One widget changing must cost the changed widget, not the tree: a frame
/// that re-measures, re-lays-out and repaints a single leaf on a 10k-node
/// tree still runs the per-node passes, but the passes must not *allocate*
/// per node to do it.
#[test]
fn a_one_widget_change_frame_does_not_allocate_per_node() {
    // Two change cycles before the measured one: the first rebuild of the
    // semantics index grows its (capacity-reusing) storage from empty, which
    // is one-time construction cost like the mount itself — the claim under
    // test is about the *steady state* of a changed frame.
    let (mut host, victim) = wide_tree(1_000, 10);
    host.frame();
    host.frame();
    for content in ["first-change", "leaf"] {
        if let Some(text) = host.widget_mut::<Text>(victim) {
            text.set_content(content);
        }
        host.frame();
        host.frame();
    }
    let (allocs, _) = measure(|| {
        if let Some(text) = host.widget_mut::<Text>(victim) {
            text.set_content("changed");
        }
        host.frame()
    });
    let nodes = 1 + 1_000 * 11;
    // The bound is *structural*, not absolute: a changed frame still rebuilds
    // the semantics index and re-lays-out, and those passes cost per
    // children-bearing node — but they must stay a small fraction of the tree,
    // never one allocation per node. (Half the tree is generous headroom; a
    // genuine per-node regression lands at ~1.5 allocations per node and
    // fails this with room to spare.)
    assert!(
        allocs * 2 < nodes as u64,
        "one changed leaf on a ~{nodes}-node tree allocated {allocs} times; \
             a per-node allocation hid behind the change"
    );
}

/// Audit record (not a hard gate): a forced re-layout at scale, with the host's
/// own scratch reuse in place. Printed so the ledger can carry the measured
/// numbers; the hard bounds live in the two assertions above.
#[test]
fn relayout_allocation_audit_record() {
    for &(groups, leaves) in &[(100usize, 10usize), (1_000, 10), (10_000, 10)] {
        let nodes = 1 + groups * (1 + leaves);
        let (mut host, _) = wide_tree(groups, leaves);
        host.frame();
        host.frame();
        let (allocs, stats): (u64, floem_app::FrameStats) = measure(|| {
            host.invalidate_layout();
            host.frame()
        });
        eprintln!(
            "relayout audit: {nodes} nodes -> {allocs} allocations ({} laid out)",
            stats.layout_nodes
        );
        assert!(stats.layout_nodes > 0, "the forced relayout actually ran");
    }
}

/// The alpha of the first text node on screen.
fn first_text_alpha(host: &WidgetHost) -> u8 {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            floem_scene::SceneNode::Text { color, .. } => Some(color.a),
            _ => None,
        })
        .expect("the tree painted a text node")
}

/// S1/B9 — a *running* animation must not make a settled frame allocate per
/// node either.
///
/// This is the claim that makes an animation affordable at all: a spinner runs
/// for as long as the user waits, so "the frame loop is awake" cannot be
/// allowed to mean "the frame loop allocates". Two things have to hold, and
/// both are easy to get wrong in a way no "did it move" test can see: the
/// per-frame samples live in a buffer the host owns (which is why the
/// scheduler has `tick_into` at all), and the node's motion is a field that is
/// written in place rather than a fresh collection built per frame.
#[test]
fn a_running_animation_does_not_allocate_per_node() {
    let (mut host, leaf) = wide_tree(1_000, 10);
    host.frame();
    host.frame();

    // Four seconds, on a tree that will be measured within four frames: the
    // sampled value moves the 8-bit alpha by ~1 every frame, so every measured
    // frame really does repaint the node it animates. A longer animation would
    // round to the same byte for hundreds of frames and quietly turn this into
    // a test of an idle frame.
    host.animate(
        leaf,
        AnimKind::Tween(Tween::new(0.0, 1.0, 4.0).with_compositor(CompositorProp::Opacity(1.0))),
    );
    let warm = host.frame();
    assert_eq!(host.animations_running(), 1, "control: it is running");
    assert!(
        warm.updated == 0,
        "control: no widget changed — whatever this frame costs is the \
         animation's cost, not a widget's"
    );
    let before = first_text_alpha(&host);

    let (allocs, stats) = measure(|| host.frame());
    assert!(
        first_text_alpha(&host) != before,
        "control: the measured frame is a frame the animation moved"
    );
    assert!(stats.painted > 0, "and it painted");

    // The same constant the settled-frame test allows, on purpose: the claim
    // is that an animation does not move a frame's allocation profile at all.
    // A per-node bound here would be useless — on an 11k-node tree it would
    // permit 5.5k allocations and call that "no per-node cost".
    let nodes = 1 + 1_000 * 11;
    assert!(
        allocs <= 8,
        "one frame of a running animation on a {nodes}-node tree allocated \
         {allocs} times; the animation's own per-frame work is hiding a \
         per-node or per-frame allocation"
    );
    eprintln!("motion audit: {nodes} nodes, animation running -> {allocs} allocations/frame");
}

/// A flat tree of `leaves` text children, every one of them animating.
///
/// Built to make the *number of animations* the only thing that scales, so a
/// cost that grows with it has nowhere to hide behind the tree's shape.
fn animated_tree(leaves: usize) -> (WidgetHost, Vec<NodeId>) {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(800.0, 600.0));
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let mut ids = Vec::new();
    for _ in 0..leaves {
        let leaf = host.mount(root, LayoutStyle::default(), Text::new("leaf"));
        ids.push(leaf);
    }
    host.frame();
    host.frame();
    for &id in &ids {
        host.animate(
            id,
            AnimKind::Tween(
                Tween::new(0.0, 1.0, 4.0).with_compositor(CompositorProp::Opacity(1.0)),
            ),
        );
    }
    // Two frames to settle whatever the first one builds (the damage list grows
    // to hold the nodes that moved) before anything is measured.
    host.frame();
    host.frame();
    (host, ids)
}

/// S1/B9, the other axis — a frame with a hundred animations must not cost
/// more *allocations* than a frame with ten.
///
/// The count is the falsifiable part: an implementation that rebuilt a
/// per-animation structure each frame (`motion_samples.clone()` in the parking
/// loop is the shape of it) allocates once per animation and lands orders of
/// magnitude over this bound, while costing exactly the same number of bytes
/// per node it would otherwise have. Nothing about the pixels gives it away.
#[test]
fn a_running_animation_does_not_allocate_per_animation() {
    let (mut few, few_ids) = animated_tree(10);
    assert_eq!(
        few.animations_running(),
        few_ids.len(),
        "control: 10 running"
    );
    let (small, small_stats) = measure(|| few.frame());
    assert_eq!(small_stats.updated, 0, "control: no widget changed");

    let (mut many, many_ids) = animated_tree(400);
    assert_eq!(
        many.animations_running(),
        many_ids.len(),
        "control: 400 running"
    );
    let (big, big_stats) = measure(|| many.frame());
    assert_eq!(big_stats.updated, 0, "control: no widget changed");

    assert!(
        big <= small + 8,
        "40x the animations turned {small} allocations into {big}; the frame's \
         per-frame cost is scaling with how many animations are running"
    );
    eprintln!(
        "motion audit: 10 animations -> {small} allocations/frame, \
         400 animations -> {big} allocations/frame"
    );
}
