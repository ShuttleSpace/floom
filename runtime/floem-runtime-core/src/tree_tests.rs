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
