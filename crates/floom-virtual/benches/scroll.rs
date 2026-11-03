//! Benchmarks for the virtual viewport: range queries at 1k/10k/100k items and
//! a full scroll-and-recycle loop exercising the stable-key diff.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floom_types::ElementId;
use floom_virtual::{ItemSizes, VirtualViewport};

fn bench_visible_range(c: &mut Criterion) {
    let mut group = c.benchmark_group("visible_range");
    for &n in &[1_000usize, 10_000, 100_000] {
        let sizes = ItemSizes::uniform(10.0, n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, n_ref| {
            let n = *n_ref;
            b.iter(|| {
                let mut vp = VirtualViewport::<usize>::new(sizes.clone(), 100.0);
                for s in (0..n).step_by(997) {
                    vp.scroll_to(s as f32);
                    let r = vp.visible_range();
                    std::hint::black_box(r);
                }
            });
        });
    }
    group.finish();
}

fn bench_update_reuse(c: &mut Criterion) {
    let n = 100_000usize;
    let sizes = ItemSizes::uniform(10.0, n);
    let mut group = c.benchmark_group("update_reuse");
    group.bench_function("scroll_100k_recycle", |b| {
        b.iter(|| {
            let mut vp = VirtualViewport::<usize, ElementId>::new(sizes.clone(), 100.0);
            let mut pool: Vec<ElementId> = Vec::new();
            let mut next: u32 = 0;
            for s in (0..n).step_by(50) {
                vp.scroll_to(s as f32);
                let diff = vp.update(|i| i);
                for (_, elem) in diff.exit {
                    pool.push(elem);
                }
                for (_, key) in diff.enter {
                    let elem = pool.pop().unwrap_or_else(|| {
                        let e = ElementId::new(next, 1);
                        next += 1;
                        e
                    });
                    vp.assign(key, elem);
                }
            }
        });
    });
    group.finish();
}

criterion_group!(benches, bench_visible_range, bench_update_reuse);
criterion_main!(benches);
