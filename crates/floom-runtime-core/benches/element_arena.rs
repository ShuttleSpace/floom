use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floom_core::{Element, ElementKind};
use floom_runtime_core::{DirtyFlags, DirtySet, ElementArena, ElementTree};
use floom_types::ElementId;

fn bench_arena(c: &mut Criterion) {
    let mut group = c.benchmark_group("element_arena");
    group.measurement_time(Duration::from_secs(5));

    for n in [1_000usize, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::new("insert", n), &n, |b, &n| {
            b.iter(|| {
                let mut arena = ElementArena::new();
                for _ in 0..n {
                    black_box(arena.insert_primitive());
                }
                black_box(arena.len());
            });
        });

        group.bench_with_input(BenchmarkId::new("get", n), &n, |b, &n| {
            let mut arena = ElementArena::new();
            let mut ids = Vec::with_capacity(n);
            for _ in 0..n {
                ids.push(arena.insert_primitive());
            }
            b.iter(|| {
                for &id in &ids {
                    black_box(arena.get(id));
                }
            });
        });
    }

    group.finish();
}

fn chain_of(n: usize) -> (ElementArena, ElementId) {
    let mut arena = ElementArena::new();
    let root = arena.insert_primitive();
    let mut parent = root;
    for _ in 1..n {
        let mut e = Element::new(ElementId::INVALID, ElementKind::Primitive);
        e.parent = Some(parent);
        parent = arena.insert(e);
    }
    (arena, parent)
}

fn bench_dirty(c: &mut Criterion) {
    let mut group = c.benchmark_group("dirty_vs_scan");
    group.measurement_time(Duration::from_secs(5));
    let n = 10_000usize;
    let (arena, leaf) = chain_of(n);

    group.bench_function("propagate_layout_10k_chain", |b| {
        let mut dirty = DirtySet::new();
        b.iter(|| {
            dirty.clear(leaf);
            black_box(dirty.propagate_layout(&arena, leaf));
        });
    });

    group.bench_function("whole_tree_scan_10k", |b| {
        b.iter(|| {
            let mut count = 0usize;
            for i in 0..n as u32 {
                if arena.get(ElementId::new(i, 1)).is_some() {
                    count += 1;
                }
            }
            black_box(count);
        });
    });

    let _ = DirtyFlags::LAYOUT;
    group.finish();
}

fn bench_tree(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_mutation");
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("append_1k_children", |b| {
        b.iter(|| {
            let mut arena = ElementArena::new();
            let mut tree = ElementTree::new();
            let root = arena.insert_primitive();
            for _ in 0..1_000 {
                let child = arena.insert_primitive();
                black_box(tree.append_child(&mut arena, root, child));
            }
            black_box(tree.children(root).len());
        });
    });

    group.bench_function("reparent_1k", |b| {
        let mut arena = ElementArena::new();
        let mut tree = ElementTree::new();
        let parent_a = arena.insert_primitive();
        let parent_b = arena.insert_primitive();
        let mut kids = Vec::with_capacity(1_000);
        for _ in 0..1_000 {
            let child = arena.insert_primitive();
            tree.append_child(&mut arena, parent_a, child);
            kids.push(child);
        }
        b.iter(|| {
            for &child in &kids {
                black_box(tree.append_child(&mut arena, parent_b, child));
            }
            for &child in &kids {
                black_box(tree.append_child(&mut arena, parent_a, child));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_arena, bench_dirty, bench_tree);
criterion_main!(benches);
