// SPDX-License-Identifier: MIT
//! Benchmark: native slot acquire/release churn for a 10k virtualized row set.

use criterion::{Criterion, criterion_group, criterion_main};
use floom_native::NativeViewPool;

fn bench_rows(c: &mut Criterion) {
    c.bench_function("native_pool_10k_churn", |b| {
        b.iter(|| {
            let mut pool: NativeViewPool<u32> = NativeViewPool::new();
            for k in 0..10_000u32 {
                pool.acquire(k);
            }
            for k in 0..10_000u32 {
                pool.release(&k);
            }
            for k in 0..10_000u32 {
                pool.acquire(k + 1_000_000);
            }
        });
    });
}

criterion_group!(benches, bench_rows);
criterion_main!(benches);
