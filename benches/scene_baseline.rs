//! Floom 2.0 Phase 0 — scene baseline placeholder.
//!
//! There is no `floom-scene` yet. This bench measures retained-layout churn on
//! the current headless path as a stand-in until Scene damage tracking lands.

use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use floom::headless::{HeadlessHarness, TestRoot};
use floom::prelude::*;
use floom::style::FlexWrap;
use floom::views::{Decorators, Empty, Stack};

fn grid(count: usize) -> impl IntoView {
    let cells: Vec<_> = (0..count)
        .map(|i| {
            Empty::new().style(move |s| {
                s.size(20.0, 20.0)
                    .margin(1.0)
                    .background(if i % 2 == 0 {
                        palette::css::LIGHT_GRAY
                    } else {
                        palette::css::DARK_GRAY
                    })
            })
        })
        .collect();
    Stack::from_iter(cells).style(|s| {
        s.flex_row()
            .flex_wrap(FlexWrap::Wrap)
            .size(440.0, 440.0)
            .padding(4.0)
    })
}

fn bench_scene_standin(c: &mut Criterion) {
    let mut group = c.benchmark_group("scene_baseline_standin");
    group.measurement_time(Duration::from_secs(8));

    group.bench_function("relayout_10k_empty_cells", |b| {
        let root = TestRoot::new();
        let view = grid(10_000);
        let mut harness = HeadlessHarness::new_with_size(root, view, 440.0, 440.0);
        b.iter(|| {
            harness.set_size(441.0, 441.0);
            harness.set_size(440.0, 440.0);
            black_box(harness.root_id());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scene_standin);
criterion_main!(benches);
