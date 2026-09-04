use super::ElementArena;
use floem_core::{Element, ElementKind};
use floem_types::ElementId;

#[test]
fn insert_get_remove() {
    let mut arena = ElementArena::new();
    let id = arena.insert_primitive();
    assert!(arena.get(id).is_some());
    assert_eq!(arena.get(id).unwrap().kind, ElementKind::Primitive);
    let removed = arena.remove(id).unwrap();
    assert_eq!(removed.id, id);
    assert!(arena.get(id).is_none());
    assert!(arena.is_empty());
}

#[test]
fn stale_id_rejected() {
    let mut arena = ElementArena::new();
    let id = arena.insert_primitive();
    assert!(arena.remove(id).is_some());
    assert!(arena.get(id).is_none());
    assert!(arena.get_mut(id).is_none());
    assert!(arena.remove(id).is_none());

    let reused = arena.insert_primitive();
    assert_eq!(reused.index(), id.index());
    assert_ne!(reused.generation(), id.generation());
    assert!(arena.get(id).is_none());
    assert!(arena.get(reused).is_some());
}

#[test]
fn parent_field_roundtrip() {
    let mut arena = ElementArena::new();
    let parent = arena.insert_primitive();
    let mut child = Element::new(ElementId::INVALID, ElementKind::Primitive);
    child.parent = Some(parent);
    let child_id = arena.insert(child);
    assert_eq!(arena.get(child_id).unwrap().parent, Some(parent));
}

#[test]
fn element_is_stack_allocated_copy() {
    // Regression guard: Element must stay Copy so the arena Vec does not
    // force a heap allocation per element by default.
    fn assert_copy<T: Copy>(_: &T) {}
    let e = Element::new(ElementId::new(1, 1), ElementKind::Host);
    assert_copy(&e);
    assert!(std::mem::size_of::<Element>() <= 32);
}
