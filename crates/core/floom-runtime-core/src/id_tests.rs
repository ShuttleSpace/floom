// SPDX-License-Identifier: MIT
use super::IdAllocator;
use floom_types::ElementId;

#[test]
fn alloc_returns_live_ids() {
    let mut a = IdAllocator::new();
    let a0 = a.alloc();
    let a1 = a.alloc();
    assert!(a.contains(a0));
    assert!(a.contains(a1));
    assert_ne!(a0, a1);
    assert_eq!(a.len(), 2);
}

#[test]
fn free_rejects_stale_id() {
    let mut a = IdAllocator::new();
    let id = a.alloc();
    assert!(a.free(id));
    assert!(!a.contains(id));
    assert!(!a.free(id));
    assert_eq!(a.len(), 0);
}

#[test]
fn reuse_bumps_generation() {
    let mut a = IdAllocator::new();
    let first = a.alloc();
    let index = first.index();
    assert_eq!(first.generation(), 1);
    assert!(a.free(first));
    let second = a.alloc();
    assert_eq!(second.index(), index);
    assert_eq!(second.generation(), 2);
    assert!(!a.contains(first));
    assert!(a.contains(second));
}

#[test]
fn invalid_never_contained() {
    let a = IdAllocator::new();
    assert!(!a.contains(ElementId::INVALID));
}

#[test]
fn restore_resurrects_the_exact_id_that_was_freed() {
    // Checkpoint/restore contract: after a rollback the allocator must be able
    // to put back the *same* id. It must refuse anything at or beyond the
    // generation the allocator would hand out next, which is the handle the
    // next alloc mints — restoring that would alias a live-or-future id.
    let mut a = IdAllocator::new();
    let first = a.alloc();
    assert_eq!(first.generation(), 1);
    assert!(a.free(first));

    let future = ElementId::new(first.index(), 2);
    assert!(
        !a.restore(future),
        "generation 2 is what alloc would mint next"
    );
    assert!(!a.contains(first));

    assert!(a.restore(first));
    assert!(a.contains(first));
    // And the slot is occupied again, so alloc must hand out a different
    // index, not the restored id.
    let next = a.alloc();
    assert_ne!(next.index(), first.index());
    assert!(a.contains(next));
}

#[test]
fn restore_refuses_a_live_or_invalid_id() {
    let mut a = IdAllocator::new();
    let live = a.alloc();
    assert!(!a.restore(live), "the slot is occupied");
    assert!(!a.restore(ElementId::INVALID));
}
