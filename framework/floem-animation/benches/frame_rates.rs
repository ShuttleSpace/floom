use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use floem_animation::{AnimKind, AnimationScheduler, CompositorProp, Easing, Tween};

fn tick_many(c: &mut Criterion) {
    let mut group = c.benchmark_group("animation_tick");
    for &(name, hz, n) in &[("60hz_256", 60.0, 256usize), ("120hz_256", 120.0, 256usize)] {
        group.bench_function(name, |b| {
            let mut sched = AnimationScheduler::new();
            for i in 0..n {
                sched.start(
                    AnimKind::Tween(Tween {
                        from: 0.0,
                        to: 1.0,
                        duration_secs: 1.0,
                        easing: Easing::Linear,
                        compositor: Some(CompositorProp::Opacity(0.0)),
                    }),
                    i as f64 * 0.001,
                );
            }
            let dt = 1.0 / hz;
            let mut t = 0.0;
            b.iter(|| {
                t += dt;
                black_box(sched.tick(t));
            });
        });
    }
    group.finish();
}

criterion_group!(benches, tick_many);
criterion_main!(benches);
