//! Floem 2.0 Phase 0 — render baseline (current paint path; no Scene ABI yet).

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use floem::headless::{HeadlessHarness, TestRoot};
use floem::prelude::*;
use floem::views::{Decorators, Label, Stack};

fn labeled_stack(count: usize) -> impl IntoView {
    let labels: Vec<_> = (0..count)
        .map(|i| {
            Label::new(format!("Paint {i}")).style(|s| {
                s.padding(4.0)
                    .margin(1.0)
                    .background(palette::css::WHITE_SMOKE)
            })
        })
        .collect();
    Stack::from_iter(labels).style(|s| s.flex_col().size(320.0, 640.0))
}

fn bench_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_baseline");
    group.measurement_time(Duration::from_secs(8));

    group.bench_function("paint_1k_labels_headless", |b| {
        let root = TestRoot::new();
        let view = labeled_stack(1_000);
        let mut harness = HeadlessHarness::new_with_size(root, view, 320.0, 640.0);
        b.iter(|| {
            harness.paint();
            black_box(harness.root_id());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_render);
criterion_main!(benches);
