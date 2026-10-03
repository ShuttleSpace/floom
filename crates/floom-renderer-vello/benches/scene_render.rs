use std::hint::black_box;
use std::time::Duration;

use criterion::{Criterion, criterion_group, criterion_main};
use floem_renderer::{FrameInfo, render_once};
use floem_renderer_vello::VelloRendererAdapter;
use floem_scene::{Color, Point, Rect, Scene, SceneNode, Size};

fn build_scene(n: usize) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    for i in 0..n {
        let _ = scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(
                Point::new((i % 100) as f64, (i / 100) as f64),
                Size::new(4.0, 4.0),
            ),
            fill: floem_types::Brush::from(Color::BLACK),
            radius: 0.0,

            shadow: None,
        });
    }
    scene.finish_update();
    scene
}

fn bench_scene_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("vello_adapter");
    group.measurement_time(Duration::from_secs(5));

    let full = build_scene(1_000);
    group.bench_function("full_frame_1k_rects", |b| {
        let mut adapter = VelloRendererAdapter::new();
        b.iter(|| {
            render_once(
                &mut adapter,
                &FrameInfo::software(200, 200, 1),
                black_box(&full),
            );
        });
    });

    let mut partial = build_scene(1_000);
    partial.clear_damage();
    // Touch one rect to create partial damage.
    let first_id = partial.iter().next().map(|(id, _)| id);
    partial.begin_update();
    if let Some(id) = first_id {
        partial.upsert(
            id,
            SceneNode::Rect {
                bounds: Rect::from_origin_size(Point::ZERO, Size::new(4.0, 4.0)),
                fill: floem_types::Brush::from(Color::WHITE),
                radius: 0.0,

                shadow: None,
            },
        );
    }
    partial.finish_update();

    group.bench_function("partial_damage_1k_scene", |b| {
        let mut adapter = VelloRendererAdapter::new();
        b.iter(|| {
            render_once(
                &mut adapter,
                &FrameInfo::software(200, 200, 1),
                black_box(&partial),
            );
        });
    });

    group.finish();
}

criterion_group!(benches, bench_scene_render);
criterion_main!(benches);
