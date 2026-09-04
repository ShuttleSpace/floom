use std::hint::black_box;
use std::time::Duration;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floem_runtime_core::ElementArena;

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

criterion_group!(benches, bench_arena);
criterion_main!(benches);
