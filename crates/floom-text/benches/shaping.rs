use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use floom_text::{LayoutCache, TextLayoutOptions, TextShaper};

fn shaping_uncached(c: &mut Criterion) {
    let mut shaper = TextShaper::new();
    let opts = TextLayoutOptions {
        max_width: 200.0,
        font_size: 14.0,
        ..Default::default()
    };
    let text = "The quick brown fox jumps over the lazy dog. مرحبا 😀";
    c.bench_function("shape_uncached", |b| {
        b.iter(|| {
            black_box(shaper.shape(black_box(text), black_box(&opts)));
        })
    });
}

fn shaping_cached(c: &mut Criterion) {
    let mut shaper = TextShaper::new();
    let mut cache = LayoutCache::new();
    let opts = TextLayoutOptions {
        max_width: 200.0,
        font_size: 14.0,
        ..Default::default()
    };
    let text = "The quick brown fox jumps over the lazy dog.";
    // Warm
    cache.get_or_insert_with(text, &opts, || shaper.shape(text, &opts));
    c.bench_function("shape_cached", |b| {
        b.iter(|| {
            black_box(cache.get_or_insert_with(text, &opts, || shaper.shape(text, &opts)));
        })
    });
}

criterion_group!(benches, shaping_uncached, shaping_cached);
criterion_main!(benches);
