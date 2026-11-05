// SPDX-License-Identifier: MIT
use super::ElementTree;
use crate::arena::ElementArena;

#[test]
fn append_and_walk() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let root = arena.insert_primitive();
    let a = arena.insert_primitive();
    let b = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, root, a));
    assert!(tree.append_child(&mut arena, root, b));
    assert_eq!(tree.children(root), &[a, b]);
    assert_eq!(arena.get(a).unwrap().parent, Some(root));

    let mut order = Vec::new();
    tree.walk_preorder(root, |id| order.push(id));
    assert_eq!(order, vec![root, a, b]);
}

#[test]
fn insert_at_index() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let root = arena.insert_primitive();
    let a = arena.insert_primitive();
    let b = arena.insert_primitive();
    let c = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, root, a));
    assert!(tree.append_child(&mut arena, root, c));
    assert!(tree.insert_child(&mut arena, root, 1, b));
    assert_eq!(tree.children(root), &[a, b, c]);
}

#[test]
fn detach_clears_parent() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let root = arena.insert_primitive();
    let child = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, root, child));
    assert!(tree.detach(&mut arena, child));
    assert!(tree.children(root).is_empty());
    assert_eq!(arena.get(child).unwrap().parent, None);
}

#[test]
fn move_child_reorders() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let root = arena.insert_primitive();
    let a = arena.insert_primitive();
    let b = arena.insert_primitive();
    let c = arena.insert_primitive();
    tree.append_child(&mut arena, root, a);
    tree.append_child(&mut arena, root, b);
    tree.append_child(&mut arena, root, c);
    assert!(tree.move_child(&mut arena, root, a, 2));
    assert_eq!(tree.children(root), &[b, c, a]);
}

#[test]
fn rejects_cycle() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let root = arena.insert_primitive();
    let child = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, root, child));
    assert!(!tree.append_child(&mut arena, child, root));
}

#[test]
fn reparent_detaches_from_old() {
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let a = arena.insert_primitive();
    let b = arena.insert_primitive();
    let child = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, a, child));
    assert!(tree.append_child(&mut arena, b, child));
    assert!(tree.children(a).is_empty());
    assert_eq!(tree.children(b), &[child]);
    assert_eq!(arena.get(child).unwrap().parent, Some(b));
}

#[test]
fn clear_children_drops_the_list_so_index_reuse_is_clean() {
    // A removed element's slot can be reused by a later alloc. If its child
    // list survived, the new element would start life with phantom children.
    let mut arena = ElementArena::new();
    let mut tree = ElementTree::new();
    let parent = arena.insert_primitive();
    let a = arena.insert_primitive();
    let b = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, parent, a));
    assert!(tree.append_child(&mut arena, parent, b));

    tree.clear_children(&mut arena, parent);
    assert!(tree.children(parent).is_empty());
    assert_eq!(arena.get(a).unwrap().parent, None);
    assert_eq!(arena.get(b).unwrap().parent, None);

    // The slot is reusable with a clean list: a fresh child of the *new*
    // element is the only one listed.
    assert!(arena.remove(parent).is_some());
    let reused = arena.insert_primitive();
    assert_eq!(reused.index(), parent.index());
    let kid = arena.insert_primitive();
    assert!(tree.append_child(&mut arena, reused, kid));
    assert_eq!(tree.children(reused), &[kid]);
}
