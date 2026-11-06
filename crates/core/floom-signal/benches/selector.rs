// SPDX-License-Identifier: MIT
use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use floom_signal::{SignalGet, store};

#[derive(Clone)]
struct BigState {
    hot: u64,
    cold: [u64; 64],
}

fn bench_selector(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_selector");
    group.measurement_time(Duration::from_secs(5));

    let s = store(BigState {
        hot: 0,
        cold: [0; 64],
    });
    let hot = s.select(|st| st.hot);

    group.bench_function("update_cold_field_read_hot_selector", |b| {
        b.iter(|| {
            s.update(|st| {
                st.cold[0] = st.cold[0].wrapping_add(1);
            });
            black_box(hot.get());
        });
    });

    group.bench_function("update_hot_field_read_hot_selector", |b| {
        b.iter(|| {
            s.update(|st| {
                st.hot = st.hot.wrapping_add(1);
            });
            black_box(hot.get());
        });
    });

    group.finish();
}

criterion_group!(benches, bench_selector);
criterion_main!(benches);
