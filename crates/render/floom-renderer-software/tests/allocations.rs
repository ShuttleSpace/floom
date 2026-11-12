// SPDX-License-Identifier: MIT
//! The renderer's allocation contract.
//!
//! A frame that allocates is a frame that can drop frames: the allocator
//! takes a lock, and on a busy host the lock is the whole budget. So the
//! numbers here are not a curiosity — they are what keeps the common paths
//! cheap.
//!
//! The contract:
//!
//! * a **rect** allocates nothing at all. A frame of a thousand rectangles —
//!   the shape of a real UI — must not touch the heap once;
//! * a **filled path** allocates a small, bounded number per path, because
//!   flattening a curve needs somewhere to put the points;
//! * a **stroked path** allocates more, because a stroke is a fill of its own
//!   outline and the outline is built per path.
//!
//! The ceilings are deliberately loose. They are there to catch a change that
//! makes a path allocate *more* — a copy that was not there before, a
//! per-frame temporary that escaped — not to pin an exact count that a
//! reasonable refactor would move.
//!
//! All three measurements live in one test on purpose: the counting allocator
//! is process-wide, so three tests running in parallel would each count the
//! others' allocations and every number would be wrong.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_renderer::{FrameInfo, render_once};
use floom_scene::{Scene, SceneNode};
use floom_types::{Brush, Color, Path, Point, Rect, Size, Stroke};

/// Counts allocations while the flag is on.
struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);
static COUNTING: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) == 1 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) == 1 {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

const COUNT: usize = 1000;
const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

/// The allocations a frame of `scene` makes, on a renderer that has already
/// drawn a frame (so the first frame's one-off setup is not counted).
fn allocations_per_frame(scene: &Scene) -> usize {
    let mut renderer = floom_renderer_software::SoftwareRenderer::new(WIDTH, HEIGHT);
    render_once(&mut renderer, &FrameInfo::software(WIDTH, HEIGHT, 1), scene);

    let before = ALLOCATIONS.load(Ordering::Relaxed);
    COUNTING.store(1, Ordering::Relaxed);
    render_once(&mut renderer, &FrameInfo::software(WIDTH, HEIGHT, 2), scene);
    COUNTING.store(0, Ordering::Relaxed);
    ALLOCATIONS.load(Ordering::Relaxed) - before
}

/// A scene of a thousand rectangles.
fn rects() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    for index in 0..COUNT {
        let x = (index % 1000) as f64 * 0.8;
        scene.insert(SceneNode::Rect {
            bounds: Rect::from_origin_size(Point::new(x, 100.0), Size::new(0.8, 0.8)),
            fill: Brush::from(Color::rgb(30, 90, 200)),
            radius: 0.0,
            shadow: None,
        });
    }
    scene.finish_update();
    scene
}

/// A scene of a thousand filled paths.
fn filled() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    for index in 0..COUNT {
        let x = (index % 1000) as f64 * 0.8;
        let mut path = Path::new();
        path.move_to(Point::new(x, 100.0))
            .line_to(Point::new(x + 0.8, 100.8));
        scene.insert(SceneNode::filled_path(path, Color::rgb(30, 90, 200)));
    }
    scene.finish_update();
    scene
}

/// A scene of a thousand stroked paths.
fn stroked() -> Scene {
    let mut scene = Scene::new();
    scene.begin_update();
    let stroke = Stroke::new(2.0, Color::rgb(30, 90, 200));
    for index in 0..COUNT {
        let x = (index % 1000) as f64 * 0.8;
        let mut path = Path::new();
        path.move_to(Point::new(x, 100.0))
            .line_to(Point::new(x + 0.8, 100.8));
        scene.insert(SceneNode::stroked_path(Arc::new(path), stroke));
    }
    scene.finish_update();
    scene
}

/// The three contracts, measured one after another.
#[test]
fn the_renderers_allocation_contract_holds() {
    // A frame of rectangles touches the heap not at all. This is the number
    // that matters for a real UI: a thousand rectangles is a screen, and a
    // screen that allocates is a screen that can drop a frame.
    let rects = allocations_per_frame(&rects());
    assert_eq!(
        rects, 0,
        "a frame of {COUNT} rectangles allocated {rects} times"
    );

    // A filled path allocates a bounded number of times: flattening a path
    // needs somewhere to put the points, so it cannot be zero.
    let filled = allocations_per_frame(&filled());
    let per_filled = filled / COUNT;
    assert!(
        per_filled <= 4,
        "a filled path allocated {per_filled} times ({filled} for {COUNT} paths)"
    );

    // A stroked path allocates more, because a stroke is a fill of its own
    // outline and the outline is built per path.
    let stroked = allocations_per_frame(&stroked());
    let per_stroked = stroked / COUNT;
    assert!(
        per_stroked <= 12,
        "a stroked path allocated {per_stroked} times ({stroked} for {COUNT} paths)"
    );
}
