use floom_core::{CreateContext, ElementId, UpdateContext, Widget};
use floom_legacy_reactive::{RwSignal, SignalGet, SignalUpdate};
use floom_runtime::{Event, Runtime};
use floom_runtime_core::DirtyFlags;
use floom_types::Point;
use std::time::Instant;

struct Leaf;
impl Widget for Leaf {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}
}

fn mount_leaf(rt: &mut Runtime) -> ElementId {
    rt.mount(&Leaf).root
}

#[test]
fn counter_click_updates_only_label() {
    let mut rt = Runtime::new();
    let label = mount_leaf(&mut rt);
    let button = mount_leaf(&mut rt);
    let sibling = mount_leaf(&mut rt);
    let count = RwSignal::new(0i32);

    rt.bind_signal(label, DirtyFlags::PAINT, count);
    // Initial bind runs effect once → label pending dirty.
    rt.tick(Instant::now());
    let label_updates_after_bind = rt.update_count(label);
    assert!(label_updates_after_bind >= 1);
    assert_eq!(rt.update_count(sibling), 0);
    assert_eq!(rt.update_count(button), 0);

    let count_click = count;
    rt.on_click(button, move || {
        count_click.set(count_click.get() + 1);
    });

    rt.dispatch(Event::PointerClick {
        target: button,
        position: Point::ZERO,
    });
    assert!(rt.frame_requested());
    rt.tick(Instant::now());

    assert_eq!(count.get(), 1);
    assert_eq!(
        rt.update_count(label),
        label_updates_after_bind + 1,
        "label should update from signal"
    );
    assert_eq!(
        rt.update_count(sibling),
        0,
        "unrelated sibling must not rebuild"
    );
    assert_eq!(rt.update_count(button), 0);
    assert_eq!(rt.stats().ticks, 2);
}

#[test]
fn frame_stats_instrumentation() {
    let mut rt = Runtime::new();
    let _ = mount_leaf(&mut rt);
    rt.tick(Instant::now());
    assert_eq!(rt.stats().ticks, 1);
}
