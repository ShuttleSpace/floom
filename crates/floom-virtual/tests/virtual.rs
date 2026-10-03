//! TDD verification for the virtual viewport: range math, overscan, and the
//! stable-key element recycling loop that bounds memory on huge lists.

use std::collections::HashMap;

use floem_types::ElementId;
use floem_virtual::{ItemSizes, Overscan, VirtualViewport};

#[test]
fn uniform_visible_no_overscan() {
    let mut vp = VirtualViewport::<usize>::new(ItemSizes::uniform(10.0, 100), 100.0);
    assert_eq!(vp.visible_range(), 0..10);
    vp.scroll_to(250.0);
    assert_eq!(vp.visible_range(), 25..35);
    vp.scroll_to(995.0); // near end, clamps
    assert_eq!(vp.visible_range(), 99..100);
}

#[test]
fn uniform_overscan_items() {
    let mut vp = VirtualViewport::<usize>::new(ItemSizes::uniform(10.0, 100), 100.0);
    vp.scroll_to(250.0);
    vp.set_overscan(Overscan::Items(2));
    assert_eq!(vp.visible_range(), 23..37);
}

#[test]
fn uniform_overscan_pixels() {
    let mut vp = VirtualViewport::<usize>::new(ItemSizes::uniform(10.0, 100), 100.0);
    vp.scroll_to(100.0);
    vp.set_overscan(Overscan::Pixels(10.0));
    // window becomes [90, 210) -> items 9..21
    assert_eq!(vp.visible_range(), 9..21);
}

#[test]
fn variable_visible_range() {
    let sizes = ItemSizes::from_extents(vec![10.0, 20.0, 10.0, 20.0, 10.0]);
    let mut vp = VirtualViewport::<usize>::new(sizes, 25.0);
    assert_eq!(vp.visible_range(), 0..2);
    vp.scroll_to(25.0);
    assert_eq!(vp.visible_range(), 1..4);
}

#[test]
fn empty_list_yields_empty_range() {
    let vp = VirtualViewport::<usize>::new(ItemSizes::uniform(10.0, 0), 100.0);
    assert_eq!(vp.visible_range(), 0..0);
}

#[test]
fn recycle_keeps_pool_bounded() {
    // Scrolling a 1000-item uniform list must keep materialized elements bounded
    // and reuse a small pool instead of allocating one element per item.
    let count = 1000usize;
    let mut vp = VirtualViewport::<usize, ElementId>::new(ItemSizes::uniform(10.0, count), 100.0);
    let mut pool: Vec<ElementId> = Vec::new();
    let mut next: u32 = 0;
    let mut created = 0u32;

    let step = |vp: &mut VirtualViewport<usize, ElementId>,
                s: f32,
                pool: &mut Vec<ElementId>,
                next: &mut u32,
                created: &mut u32| {
        vp.scroll_to(s);
        let diff = vp.update(|i| i);
        for (_, elem) in diff.exit {
            pool.push(elem);
        }
        for (_, key) in diff.enter {
            let elem = pool.pop().unwrap_or_else(|| {
                let e = ElementId::new(*next, 1);
                *next += 1;
                *created += 1;
                e
            });
            vp.assign(key, elem);
        }
        // Invariant: live elements never exceed the visible window count (+ slack).
        let r = vp.visible_range();
        assert!(vp.materialized_count() <= r.end.saturating_sub(r.start) + 1);
    };

    for s in (0..count).step_by(10) {
        step(&mut vp, s as f32, &mut pool, &mut next, &mut created);
    }
    // Only ~viewport/itemsize elements were ever created; the rest were recycled.
    assert!(created <= 20, "created too many elements: {created}");
}

#[test]
fn materialized_key_set_is_stable_across_scroll() {
    // The viewport must report a correct, key-stable diff: scrolling away and
    // back to the same offset yields the same materialized key set, and a key
    // that left is reported in `exit` then re-reported in `enter` on return.
    let count = 500usize;
    let mut vp = VirtualViewport::<usize, ElementId>::new(ItemSizes::uniform(10.0, count), 100.0);
    let mut pool: Vec<ElementId> = Vec::new();
    let mut next: u32 = 0;

    let step = |vp: &mut VirtualViewport<usize, ElementId>,
                s: f32,
                pool: &mut Vec<ElementId>,
                next: &mut u32| {
        vp.scroll_to(s);
        let diff = vp.update(|i| i);
        for (_, elem) in diff.exit {
            pool.push(elem);
        }
        for (_, key) in diff.enter {
            let elem = pool.pop().unwrap_or_else(|| {
                let e = ElementId::new(*next, 1);
                *next += 1;
                e
            });
            vp.assign(key, elem);
        }
    };

    let key_set = |vp: &VirtualViewport<usize, ElementId>| -> Vec<usize> {
        let mut v: Vec<usize> = (0..count).filter(|i| vp.get(i).is_some()).collect();
        v.sort_unstable();
        v
    };

    step(&mut vp, 0.0, &mut pool, &mut next);
    let s0 = key_set(&vp);
    assert_eq!(
        s0,
        (0..10).collect::<Vec<_>>(),
        "scroll 0 should show keys 0..9"
    );
    step(&mut vp, 250.0, &mut pool, &mut next);
    step(&mut vp, 480.0, &mut pool, &mut next);
    step(&mut vp, 0.0, &mut pool, &mut next);
    let s1 = key_set(&vp);
    assert_eq!(
        s0, s1,
        "materialized key set changed after scroll round-trip"
    );
}
