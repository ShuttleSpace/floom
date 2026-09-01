//! Floem 2.0 Phase 0 — runtime baseline (existing View/reactive path).
//!
//! Measures signal-driven updates and large trees via `HeadlessHarness`.
//! Replace / extend once `floem-runtime-core` lands.

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use floem::headless::{HeadlessHarness, TestRoot};
use floem::prelude::*;
use floem::views::{Decorators, Label, Stack};

fn tree_of_labels(count: usize) -> impl IntoView {
    let labels: Vec<_> = (0..count)
        .map(|i| Label::new(format!("Item {i}")))
        .collect();
    Stack::from_iter(labels).style(|s| s.flex_col().size(400.0, 800.0))
}

fn bench_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime_baseline");
    group.measurement_time(Duration::from_secs(8));

    group.bench_function("mount_1k_labels", |b| {
        b.iter(|| {
            let root = TestRoot::new();
            let view = tree_of_labels(1_000);
            let harness = HeadlessHarness::new_with_size(root, view, 400.0, 800.0);
            black_box(harness.root_id());
        });
    });

    group.bench_function("signal_update_single_label_in_1k", |b| {
        b.iter_with_setup(
            || {
                let counter = RwSignal::new(0u32);
                let root = TestRoot::new();
                let labels: Vec<_> = (0..1_000)
                    .map(|i| {
                        if i == 0 {
                            Label::derived(move || format!("c={counter}")).into_any()
                        } else {
                            Label::new(format!("Item {i}")).into_any()
                        }
                    })
                    .collect();
                let view = Stack::from_iter(labels).style(|s| s.flex_col().size(400.0, 800.0));
                let harness = HeadlessHarness::new_with_size(root, view, 400.0, 800.0);
                (harness, counter)
            },
            |(mut harness, counter)| {
                counter.set(counter.get() + 1);
                harness.set_size(400.0, 800.0);
                black_box(harness.root_id());
            },
        );
    });

    group.finish();
}

criterion_group!(benches, bench_runtime);
criterion_main!(benches);
