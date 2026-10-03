use floom_renderer::SceneFrameQueue;
use floom_scene::{Color, Point, Rect, Scene, SceneNode, Size};
use std::thread;
use std::time::Duration;

#[test]
fn render_thread_consumes_immutable_snapshot() {
    let queue = SceneFrameQueue::with_capacity(2);
    let producer = queue.clone();
    let consumer = queue.clone();

    let mut scene = Scene::new();
    scene.begin_update();
    let _ = scene.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(10.0, 10.0)),
        fill: floom_types::Brush::from(Color::BLACK),
        radius: 0.0,

        shadow: None,
    });
    scene.finish_update();

    let handle = thread::spawn(move || {
        let snap = consumer
            .dequeue_timeout(Duration::from_secs(2))
            .expect("snapshot");
        assert_eq!(snap.frame_number, 7);
        assert_eq!(snap.scene.len(), 1);
        // Immutable consume: iterate without mutating.
        assert_eq!(snap.scene.iter().count(), 1);
    });

    producer.enqueue(scene, 7);
    handle.join().unwrap();

    let stats = queue.stats();
    assert_eq!(stats.enqueued, 1);
    assert_eq!(stats.dequeued, 1);
    assert!(stats.total_latency_ns > 0 || stats.total_wait_ns > 0);
}

#[test]
fn bounded_queue_drops_oldest() {
    let queue = SceneFrameQueue::with_capacity(1);
    let mut a = Scene::new();
    a.begin_update();
    let _ = a.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(1.0, 1.0)),
        fill: floom_types::Brush::from(Color::BLACK),
        radius: 0.0,

        shadow: None,
    });
    a.finish_update();
    let mut b = Scene::new();
    b.begin_update();
    let _ = b.insert(SceneNode::Rect {
        bounds: Rect::from_origin_size(Point::ZERO, Size::new(2.0, 2.0)),
        fill: floom_types::Brush::from(Color::WHITE),
        radius: 0.0,

        shadow: None,
    });
    b.finish_update();

    queue.enqueue(a, 1);
    queue.enqueue(b, 2);
    let snap = queue.try_dequeue().unwrap();
    assert_eq!(snap.frame_number, 2);
    assert_eq!(queue.stats().dropped_oldest, 1);
}
