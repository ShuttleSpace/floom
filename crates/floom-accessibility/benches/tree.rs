use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floom_accessibility::{SemanticsNode, SemanticsRole, SemanticsTree};
use floom_types::ElementId;

/// Draining the semantics diff is the per-frame hot path when a11y is enabled,
/// so it must scale with the number of *changed* nodes, not the tree size.
fn bench_collect_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("collect_updates");
    for n in [1_000usize, 10_000usize] {
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            let mut tree = SemanticsTree::new();
            for i in 0..n {
                let id = ElementId::new(i as u32, 1);
                tree.set_node(
                    id,
                    SemanticsNode::new(SemanticsRole::ListItem)
                        .with_label(format!("item {i}"))
                        .with_value(i.to_string()),
                );
            }
            // Baseline drain so the measured pass only sees updates.
            let _ = tree.collect_updates();

            b.iter(|| {
                for i in 0..n {
                    tree.mark_semantics_dirty(ElementId::new(i as u32, 1));
                }
                std::hint::black_box(tree.collect_updates());
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_collect_updates);
criterion_main!(benches);
