use super::{DirtyFlags, DirtySet};
use crate::arena::ElementArena;
use floom_core::Element;
use floom_types::ElementId;

#[test]
fn property_specific_invalidation() {
    let mut arena = ElementArena::new();
    let id = arena.insert_primitive();
    let mut dirty = DirtySet::new();

    assert!(dirty.invalidate(&arena, id, DirtyFlags::PAINT));
    assert_eq!(dirty.get(id), DirtyFlags::PAINT);

    assert!(dirty.invalidate(&arena, id, DirtyFlags::STYLE));
    assert_eq!(dirty.get(id), DirtyFlags::PAINT | DirtyFlags::STYLE);

    dirty.clear(id);
    assert_eq!(dirty.get(id), DirtyFlags::empty());
}

#[test]
fn stale_id_not_marked() {
    let mut arena = ElementArena::new();
    let id = arena.insert_primitive();
    arena.remove(id);
    let mut dirty = DirtySet::new();
    assert!(!dirty.invalidate(&arena, id, DirtyFlags::LAYOUT));
}

#[test]
fn propagate_layout_walks_parents() {
    let mut arena = ElementArena::new();
    let root = arena.insert_primitive();
    let mid = {
        let mut e = Element::new(ElementId::INVALID, floom_core::ElementKind::Primitive);
        e.parent = Some(root);
        arena.insert(e)
    };
    let leaf = {
        let mut e = Element::new(ElementId::INVALID, floom_core::ElementKind::Primitive);
        e.parent = Some(mid);
        arena.insert(e)
    };

    let mut dirty = DirtySet::new();
    assert!(dirty.propagate_layout(&arena, leaf));
    assert!(dirty.get(leaf).contains(DirtyFlags::LAYOUT));
    assert!(dirty.get(mid).contains(DirtyFlags::LAYOUT));
    assert!(dirty.get(root).contains(DirtyFlags::LAYOUT));
}
