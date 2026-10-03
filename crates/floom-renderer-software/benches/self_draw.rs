//! Benchmarks for self-drawn content at scale (MOTION-GAP A9).
//!
//! A canvas that repaints every frame is the worst case the flat-scene walk
//! has to survive: N vector paths, each flattened, stroked and scanline-
//! rasterised, every frame, with no damage to skip behind. These benchmarks
//! pin the two shapes that matter:
//!
//! * `chart_10k_segments` — a dense line chart, one stroked segment per node.
//!   Dominated by path flattening and stroke outline construction.
//! * `circles_1k` — a thousand filled discs. Dominated by rasterisation area.
//!
//! The scenes are built once outside the timed loop; each iteration renders a
//! complete frame through the same `render_once` entry the host uses.

use std::hint::black_box;
use std::sync::Arc;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use floem_renderer::{FrameInfo, render_once};
use floem_scene::{Scene, SceneNode};
use floem_types::{Color, Path, Point, Rect, Size, Stroke};

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// A chart-like scene: `segments` stroked line segments spread over the frame.
fn chart_scene(segments: usize) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let stroke = Stroke::new(2.0, Color::rgb(30, 90, 200));
    let cols = 1000usize;
    for i in 0..segments {
        let x0 = (i % cols) as f64 * 0.8;
        let y0 = ((i * 7919) % 600) as f64;
        let x1 = x0 + 0.8;
        let y1 = ((i * 104729) % 600) as f64;
        let mut path = Path::new();
        path.move_to(Point::new(x0, y0)).line_to(Point::new(x1, y1));
        scene.insert(SceneNode::stroked_path(Arc::new(path), stroke.clone()));
    }
    scene.finish_update();
    scene
}

/// A scatter-like scene: `count` filled circles of radius 6.
fn circles_scene(count: usize) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    for i in 0..count {
        let x = ((i * 7919) % (WIDTH as usize - 20)) as f64 + 10.0;
        let y = ((i * 104729) % (HEIGHT as usize - 20)) as f64 + 10.0;
        let path = Path::circle(Point::new(x, y), 6.0);
        scene.insert(SceneNode::filled_path(path, Color::rgba(200, 60, 60, 180)));
    }
    scene.finish_update();
    scene
}

fn bench_chart_segments(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_draw");
    for &n in &[1_000usize, 10_000] {
        let scene = chart_scene(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &scene, |b, scene| {
            b.iter(|| {
                let mut sw = floem_renderer_software::SoftwareRenderer::new(WIDTH, HEIGHT);
                render_once(
                    &mut sw,
                    &FrameInfo::software(WIDTH, HEIGHT, 1),
                    black_box(scene),
                );
                black_box(sw.paths_drawn);
            });
        });
    }
    group.finish();
}

fn bench_circles(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_draw");
    let scene = circles_scene(1_000);
    group.bench_function(BenchmarkId::from_parameter("circles_1k"), |b| {
        b.iter(|| {
            let mut sw = floem_renderer_software::SoftwareRenderer::new(WIDTH, HEIGHT);
            render_once(
                &mut sw,
                &FrameInfo::software(WIDTH, HEIGHT, 1),
                black_box(&scene),
            );
            black_box(sw.paths_drawn);
        });
    });
    group.finish();
}

/// A UI-like scene: `count` filled rounded rectangles — the shape a card,
/// button or list row is. Each is a small filled path with an 8 px corner.
fn rounded_rects_scene(count: usize) -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let w = 40.0;
    let h = 24.0;
    for i in 0..count {
        let x = ((i * 7919) % (WIDTH as usize - 60)) as f64 + 10.0;
        let y = ((i * 104729) % (HEIGHT as usize - 40)) as f64 + 10.0;
        let path = Path::rounded_rect(
            Rect::from_origin_size(Point::new(x, y), Size::new(w, h)),
            8.0,
        );
        scene.insert(SceneNode::filled_path(path, Color::rgba(60, 90, 200, 200)));
    }
    scene.finish_update();
    scene
}

fn bench_rounded_rects(c: &mut Criterion) {
    let mut group = c.benchmark_group("self_draw");
    let scene = rounded_rects_scene(1_000);
    group.bench_function(BenchmarkId::from_parameter("rounded_rects_1k"), |b| {
        b.iter(|| {
            let mut sw = floem_renderer_software::SoftwareRenderer::new(WIDTH, HEIGHT);
            render_once(
                &mut sw,
                &FrameInfo::software(WIDTH, HEIGHT, 1),
                black_box(&scene),
            );
            black_box(sw.paths_drawn);
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_chart_segments,
    bench_circles,
    bench_rounded_rects
);
criterion_main!(benches);
