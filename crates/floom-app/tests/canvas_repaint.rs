//! A7 — the repaint bridge behind `Canvas::repaint_on`.
//!
//! The host paints every mounted node on every frame it draws; what is lazy
//! is *drawing a frame at all*. So "my data changed" has exactly one hard
//! problem: waking the loop. The bridge is a reactive effect that turns the
//! signal bump into `floom_core::request_repaint(element)`, the host folds
//! the request into that element's dirty flag at the top of its next frame,
//! and `needs_frame` polls the channel so a runner keying off it wakes —
//! once: the request is consumed by the frame that serves it.
//!
//! Accordingly, the observables asserted here are:
//!
//! * the wake (`needs_frame` before the frame), the serve (the new data is
//!   in the scene after it), and the settle (nothing left to wake for);
//! * that the request is paint-only: it dirties exactly the requested
//!   element and relays out nothing;
//! * the negative space — without `repaint_on`, a data change never wakes
//!   the loop, which is the difference between a chart that redraws and one
//!   that redraws *when poked*.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_signal::{RwSignal, SignalGet, SignalUpdate, SyncRwSignal};
use floom_scene::SceneNode;
use floom_types::{Point, Rect, Size};
use floom_widgets::Canvas;

/// A canvas that draws one bar whose width is `data`, in logical px.
///
/// The signal is the `Sync` storage variant because the painter closure must
/// be `Send + Sync` (the host stores widgets it may paint from any embedder
/// thread), and the default `RwSignal` storage is `Rc`-based. The ping in
/// `repaint_on` never enters the closure — it lives only in the UI-thread
/// effect — so it stays on the default storage.
fn bar(data: SyncRwSignal<f64>, runs: Arc<AtomicUsize>) -> Canvas {
    Canvas::new(move |cx| {
        runs.fetch_add(1, Ordering::Relaxed);
        let width = data.get().clamp(0.0, cx.bounds().width());
        let rect = Rect::from_origin_size(
            Point::new(cx.bounds().x0, cx.bounds().y0),
            Size::new(width, 10.0),
        );
        cx.fill_rect(rect, floom_types::Color::rgb(30, 30, 30));
    })
}

/// The width of the first rect the host painted.
fn bar_width(host: &WidgetHost) -> f64 {
    host.scene()
        .iter()
        .find_map(|(_, node)| match node {
            SceneNode::Rect { bounds, .. } => Some(bounds.width()),
            _ => None,
        })
        .expect("the canvas drew a bar")
}

/// One bar bound to `data` (and, optionally, woken by `ping`), plus a
/// *settled* host: frames run until `needs_frame` goes false, so the initial
/// width is on screen and nothing is pending. Without the settle, a frame
/// that changed anything leaves non-empty damage behind — "the scene moved
/// since the last presented frame" — which is itself a wake reason.
fn host_with_bar(
    data: SyncRwSignal<f64>,
    ping: Option<RwSignal<()>>,
) -> (WidgetHost, Arc<AtomicUsize>) {
    let runs = Arc::new(AtomicUsize::new(0));
    let mut canvas = bar(data, runs.clone());
    if let Some(ping) = ping {
        canvas = canvas.repaint_on(ping);
    }
    let mut host = WidgetHost::new();
    host.set_size(Size::new(200.0, 60.0));
    host.mount_root(LayoutStyle::default(), canvas);
    host.frame();
    for _ in 0..10 {
        if !host.needs_frame() {
            break;
        }
        host.frame();
    }
    assert!(!host.needs_frame(), "the host did not settle");
    (host, runs)
}

#[test]
fn a_ping_bump_wakes_the_loop_and_the_next_frame_carries_the_new_data() {
    let data = SyncRwSignal::new_sync(40.0);
    let ping = RwSignal::new(());
    let (mut host, _runs) = host_with_bar(data, Some(ping));
    assert_eq!(bar_width(&host), 40.0);

    data.set(120.0);
    ping.set(());
    assert!(
        host.needs_frame(),
        "the pending repaint request is a reason to draw a frame"
    );

    let stats = host.frame();
    assert_eq!(bar_width(&host), 120.0, "the scene shows the new data");
    assert_eq!(stats.updated, 1, "exactly the requested element is dirty");
    // The serving frame left the bar's change in the damage queue (it must be
    // presented), so the settle takes one more frame — one, and then quiet.
    host.frame();
    assert!(
        !host.needs_frame(),
        "the request repaints at most one frame — the loop settles"
    );
}

/// The control that justifies the whole bridge: data changed, nothing woke.
/// A manual `frame()` would still paint it (the host paints what it is told
/// to paint) — which is precisely why runners key on `needs_frame` instead
/// of drawing continuously.
#[test]
fn without_repaint_on_a_data_change_never_wakes_the_loop() {
    let data = SyncRwSignal::new_sync(40.0);
    let (host, _) = host_with_bar(data, None);

    data.set(120.0);
    assert!(
        !host.needs_frame(),
        "no repaint request, no animation, no dirty flag: nothing to wake for"
    );
}

#[test]
fn a_repaint_request_never_relays_out() {
    let data = SyncRwSignal::new_sync(40.0);
    let ping = RwSignal::new(());
    let (mut host, _) = host_with_bar(data, Some(ping));

    let settled = host.frame();
    let layout_nodes = settled.layout_nodes;

    data.set(120.0);
    ping.set(());
    let stats = host.frame();
    assert!(!stats.relaid, "a repaint request is paint-only");
    assert_eq!(stats.layout_nodes, layout_nodes, "layout did not move");
}

#[test]
fn bursts_coalesce_into_one_repaint() {
    let data = SyncRwSignal::new_sync(40.0);
    let ping = RwSignal::new(());
    let (mut host, _) = host_with_bar(data, Some(ping));

    data.set(60.0);
    ping.set(());
    data.set(90.0);
    ping.set(());
    ping.set(());
    let stats = host.frame();
    assert_eq!(stats.updated, 1, "one frame, one dirty report");
    assert_eq!(bar_width(&host), 90.0, "the latest data is what shows");
}

#[test]
fn a_request_for_an_unknown_element_is_dropped_without_a_sound() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(100.0, 100.0));
    host.mount_root(LayoutStyle::default(), floom_widgets::Text::new("static"));
    host.frame();

    // No host minted this id; the drain must ignore it, not panic.
    floom_core::request_repaint(floom_types::ElementId::new(9_999, 9));
    let stats = host.frame();
    assert_eq!(stats.updated, 0, "an unknown id matches nothing");
    assert!(!host.needs_frame(), "and the channel is empty again");
}
