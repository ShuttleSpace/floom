//! Self-drawn surfaces, from the prelude alone.
//!
//! Three acts, each one a capability that did not exist before S2 and each
//! one asserted against the *scene the host painted* — not against a print,
//! because "it draws" is the second-easiest claim in this codebase to write
//! down and never check (after "it animates").
//!
//! 1. **A line chart is data made geometry.** The canvas strokes a polyline
//!    through the values; change a value, bump the repaint ping, and the
//!    polyline's last vertex moves to where the new value says. That is the
//!    whole A1 + A7 + repaint-bridge story in one observable.
//! 2. **A progress ring is geometry made by a fraction.** The same arc drawn
//!    at a quarter and at three quarters differs in exactly the way the
//!    fraction says: the flattened curve is longer, its end point further
//!    round. And a data change with *no* ping never wakes the frame loop —
//!    the bridge is what turns "the data changed" into "draw a frame".
//! 3. **A spinner animates itself.** It declares an endless cycle and the
//!    host runs it: every frame samples a new sweep, the arc travels, and
//!    the loop stays awake without any app code driving it.
//!
//! Run with `cargo run -p floem2-paths`.

use floem2::prelude::*;
use floem2::floem_types::{Color, LineCap, Path, PathSeg, Stroke};
use floem_legacy_reactive::{RwSignal, SignalGet, SignalUpdate, SyncRwSignal};
use floem_scene::SceneNode;

/// One repaint ping per act, so an act's assertions cannot be satisfied by
/// another act's frame.
fn main() {
    act_1_the_line_chart_is_data_made_geometry();
    act_2_the_progress_ring_is_geometry_made_by_a_fraction();
    act_3_the_spinner_animates_itself();

    println!();
    println!("every act above was asserted against the painted scene");
}

/// The `Path` node stroked (not filled) by a canvas, found by its `LineTo`
/// count so a second decorative path in the scene cannot confuse the probe.
fn stroked_path_with_line_tos(host: &WidgetHost, expected: usize) -> Vec<floem2::floem_types::Point> {
    let mut found = Vec::new();
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path { path, fill: None, stroke: Some(_) } = node {
            let line_tos: Vec<_> = path
                .segments()
                .iter()
                .filter_map(|seg| match seg {
                    PathSeg::LineTo(to) => Some(*to),
                    _ => None,
                })
                .collect();
            if line_tos.len() == expected {
                found = line_tos;
            }
        }
    }
    found
}

fn act_1_the_line_chart_is_data_made_geometry() {
    // Seven weekly values; the chart maps them into a 200×100 box.
    let data = SyncRwSignal::new_sync(vec![10.0, 30.0, 20.0, 40.0, 35.0, 50.0, 45.0]);
    let ping = RwSignal::new(());
    let painter_data = data;
    let canvas = Canvas::new(move |cx| {
        let bounds = cx.bounds();
        let pad = 10.0;
        let values = painter_data.get();
        let max = values.iter().cloned().fold(0.0_f64, f64::max).max(1.0);
        let step = (bounds.width() - 2.0 * pad) / (values.len() - 1) as f64;
        let y = |v: f64| bounds.y0 + bounds.height() - pad - (v / max) * (bounds.height() - 2.0 * pad);
        let x = |i: usize| bounds.x0 + pad + i as f64 * step;

        // The axis: one stroked line along the bottom.
        let mut axis = Path::new();
        axis.move_to(floem2::floem_types::Point::new(bounds.x0 + pad, bounds.y0 + bounds.height() - pad));
        axis.line_to(floem2::floem_types::Point::new(
            bounds.x0 + bounds.width() - pad,
            bounds.y0 + bounds.height() - pad,
        ));
        cx.stroke_path(&axis, &Stroke::new(1.0, Color::rgb(180, 180, 180)));

        // The polyline through the values: the chart itself.
        let mut line = Path::new();
        for (i, v) in values.iter().enumerate() {
            let p = floem2::floem_types::Point::new(x(i), y(*v));
            if i == 0 {
                line.move_to(p);
            } else {
                line.line_to(p);
            }
        }
        cx.stroke_path(
            &line,
            &Stroke::new(2.0, Color::rgb(30, 120, 220)).with_cap(LineCap::Round),
        );
    })
    .size(200.0, 100.0)
    .label("Revenue chart")
    .repaint_on(ping);

    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 140.0));
    let chart = host.mount_root(LayoutStyle::default(), canvas);
    host.frame();
    settle(&mut host);

    // The chart is in the scene: a stroked polyline of six segments, sitting
    // above the axis, and it is *this* shape — the last vertex tracks the
    // last value (45 of max 50, near the top of the plot).
    let vertices = stroked_path_with_line_tos(&host, 6);
    assert_eq!(vertices.len(), 6, "the chart is one six-segment polyline");
    assert!(vertices[5].y < vertices[0].y, "the last value is high on the chart");

    // A8: the chart is not invisible to assistive tech — it published its
    // role and label through the semantics channel.
    let hint = host
        .semantics_hint(chart)
        .expect("the chart publishes a semantics hint");
    assert_eq!(hint.role, floem2::advanced::SemanticsRole::Image);
    assert_eq!(hint.label.as_deref(), Some("Revenue chart"));

    // Now the data moves and the repaint ping sends it to the screen.
    data.set(vec![10.0, 30.0, 20.0, 40.0, 35.0, 50.0, 5.0]);
    ping.set(());
    assert!(host.needs_frame(), "the ping woke the loop for the new data");
    host.frame();
    let moved = stroked_path_with_line_tos(&host, 6);
    assert!(
        moved[5].y > moved[0].y,
        "the last vertex now sits low on the chart, where the new value says"
    );
    println!("act 1 — the line chart is data made geometry: ok");
}

fn act_2_the_progress_ring_is_geometry_made_by_a_fraction() {
    let progress = SyncRwSignal::<f64>::new_sync(0.25);
    let ping = RwSignal::new(());
    let painter_progress = progress;
    let canvas = Canvas::new(move |cx| {
        let bounds = cx.bounds();
        let center = floem2::floem_types::Point::new(
            bounds.x0 + bounds.width() / 2.0,
            bounds.y0 + bounds.height() / 2.0,
        );
        let radius = bounds.width().min(bounds.height()) / 2.0 - 6.0;
        let sweep = std::f64::consts::TAU * painter_progress.get().clamp(0.0, 1.0);
        let arc = Path::ellipse_arc(
            center,
            radius,
            radius,
            -std::f64::consts::FRAC_PI_2,
            sweep,
        );
        cx.stroke_path(
            &arc,
            &Stroke::new(4.0, Color::rgb(40, 160, 90)).with_cap(LineCap::Round),
        );
    })
    .size(80.0, 80.0)
    .label("Progress")
    .repaint_on(ping);

    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 120.0));
    host.mount_root(LayoutStyle::default(), canvas);
    host.frame();
    settle(&mut host);

    // A quarter turn flattens to a handful of curve steps.
    let quarter = flattened_point_count(&host);
    assert!(quarter > 2, "an arc is really drawn, not a point");

    // Control: changing the fraction alone never wakes the loop — the frame
    // loop draws when something asks it to, and the ring has not asked.
    progress.set(0.75);
    assert!(
        !host.needs_frame(),
        "a data change with no repaint request wakes nothing"
    );
    progress.set(0.25);

    // Three quarters, announced by the ping, flattens longer: more curve,
    // exactly the way the fraction says.
    progress.set(0.75);
    ping.set(());
    host.frame();
    let three_quarters = flattened_point_count(&host);
    assert!(
        three_quarters > quarter,
        "a bigger sweep is a longer flattened curve: {three_quarters} > {quarter}"
    );
    println!("act 2 — the progress ring is geometry made by a fraction: ok");
}

fn act_3_the_spinner_animates_itself() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(120.0, 60.0));
    host.mount_root(LayoutStyle::default(), Spinner::new().color(Color::rgb(90, 90, 240)));
    host.frame();

    // The spinner declared an endless cycle, so the loop never goes idle and
    // the host owns an animation nobody started by hand.
    assert!(
        host.animations_running() >= 1,
        "the spinner's declared animation is running"
    );
    assert!(host.needs_frame(), "an endless sweep keeps the loop awake");

    // And it visibly travels: the arc's start angle is the sampled progress,
    // so consecutive frames put the arc's first point in different places.
    // (Point *counts* are too blunt a probe here: one frame is six degrees
    // of arc, and flattening a short sweep needs no extra segment yet.)
    let a = arc_start(&host);
    host.frame();
    let b = arc_start(&host);
    host.frame();
    let c = arc_start(&host);
    assert!(a != b && b != c, "the sweep advances: three frames, three arcs");
    println!("act 3 — the spinner animates itself: ok");
}

/// The first point of the first flattened path on screen — where a stroked
/// arc begins, which is exactly what a spinner's advancing sweep moves.
fn arc_start(host: &WidgetHost) -> floem2::floem_types::Point {
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path { path, .. } = node {
            if let Some(sub) = path.flatten(floem2::floem_types::DEFAULT_FLATTEN_TOLERANCE).first() {
                if let Some(p) = sub.points.first() {
                    return *p;
                }
            }
        }
    }
    panic!("no path on screen");
}

/// Total flattened points across every stroked path in the scene — a crude
/// but honest "how much curve is on screen" gauge, and exactly what changes
/// when an arc's sweep or a chart's shape changes.
fn flattened_point_count(host: &WidgetHost) -> usize {
    let mut total = 0;
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path { path, .. } = node {
            for sub in path.flatten(floem2::floem_types::DEFAULT_FLATTEN_TOLERANCE) {
                total += sub.points.len();
            }
        }
    }
    total
}

/// Frames until the host is quiet: a frame that changed anything leaves
/// damage behind to be presented, so settling takes one more quiet frame.
fn settle(host: &mut WidgetHost) {
    for _ in 0..10 {
        if !host.needs_frame() {
            return;
        }
        host.frame();
    }
    panic!("the host did not settle");
}
