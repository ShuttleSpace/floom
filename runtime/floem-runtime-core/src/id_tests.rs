use super::IdAllocator;
use floem_types::ElementId;

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
