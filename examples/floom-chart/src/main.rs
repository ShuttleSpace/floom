//! Self-drawn content, from the prelude alone.
//!
//! A line chart is the shape that made the S2 slice exist: geometry no
//! primitive draws, on a surface no transform brackets. Each claim below is
//! asserted against what actually happened — the retained scene the host
//! built, and pixels a real rasteriser produced:
//!
//! 1. **A canvas paints through its closure.** The chart's polyline, its
//!    area fill and its axes land in the scene as `Path` nodes — fill and
//!    stroke in the same node where both apply, stroke-only where not.
//! 2. **The clip is a declaration the host honours.** The canvas declares
//!    `clip(ClipShape::Box)`, and a widget's clip is the widget's *own box*:
//!    the canvas sits 24px down the surface, the data peaks above the box's
//!    top edge, and a software render proves the peak is cut exactly at that
//!    edge — the band of surface above the canvas stays untouched, the plot
//!    itself does not.
//! 3. **A drawn surface is not invisible.** The canvas publishes
//!    `Role::Image` with its label — the screen reader has something to
//!    describe because the widget said so.
//! 4. **Data is made geometry, and the ping is what makes it reach the
//!    screen.** The painter reads a reactive value; a value change alone
//!    wakes nothing, the repaint ping wakes the loop, and the drawn
//!    polyline's vertex moves to where the new value says — still cut off by
//!    the clip, which is a property of the surface, not of one dataset.
//!
//! Everything below is `floom::prelude`. Run with `cargo run -p floom-chart`.

use floom_signal::{RwSignal, SignalGet, SignalUpdate, SyncRwSignal};
use floom::advanced::PaintContext;
use floom::floom_layout::AbsoluteInset;
use floom::floom_scene::SceneNode;
use floom::floom_types::{ClipShape, PathSeg};
use floom::prelude::*;

/// The rendered surface. The canvas is 320×200, placed 24px down it, so
/// overshoot above the canvas's top edge lands in surface territory where
/// only the clip decides whether it paints.
const SURFACE_W: f32 = 320.0;
const SURFACE_H: f32 = 224.0;
const CANVAS_W: f32 = 320.0;
const CANVAS_H: f32 = 200.0;
/// The canvas's top edge: where the clip's box begins.
const CANVAS_TOP: f32 = 24.0;
/// The plot area is inset within the canvas, so the axes have breathing room.
const MARGIN: f64 = 24.0;

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(SURFACE_W as f64, SURFACE_H as f64));

    // The chart's data, reactive: act 4 changes it and watches the drawn
    // geometry follow — but only once the repaint ping says so.
    let data = SyncRwSignal::new_sync(vec![0.7, 0.45, 0.6, 0.3, 0.5, 1.3]);
    let ping = RwSignal::new(());
    let chart = Canvas::new(move |cx| paint_chart(cx, data))
        .clip(ClipShape::Box)
        .label("Quarterly revenue")
        .repaint_on(ping);
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    // Absolute, 24px down the surface: the clip box's top edge sits inside
    // the surface, giving the overshoot a visible band to be cut in.
    let canvas = host.mount(
        root,
        LayoutStyle {
            position: floom::floom_layout::Position::Absolute,
            inset: AbsoluteInset {
                left: Some(0.0),
                top: Some(CANVAS_TOP),
                ..AbsoluteInset::default()
            },
            width: Some(Length::Px(CANVAS_W)),
            height: Some(Length::Px(CANVAS_H)),
            ..LayoutStyle::default()
        },
        chart,
    );
    host.frame();
    settle(&mut host);

    act_1_the_closure_reaches_the_scene(&host);
    act_2_the_host_brackets_and_the_clip_cuts(&host);
    act_3_a_drawn_surface_says_what_it_is(&host, canvas);
    act_4_data_made_geometry(&mut host, data, ping);

    println!();
    println!("every act above was asserted, not printed");
}

/// The chart's draw routine: axes, a translucent area fill, and the data line.
///
/// Everything is computed from `cx.bounds()` — the laid-out box — so the same
/// closure draws correctly at any size the layout hands it. The data comes
/// from the reactive signal the closure captured.
fn paint_chart(cx: &mut dyn PaintContext, data: SyncRwSignal<Vec<f64>>) {
    let b = cx.bounds();
    let plot = Rect {
        x0: b.x0 + MARGIN,
        y0: b.y0 + MARGIN,
        x1: b.x1 - MARGIN,
        y1: b.y1 - MARGIN,
    };

    // Axes: two stroked lines.
    let x_axis = Path::new()
        .move_to(Point::new(plot.x0, plot.y1))
        .line_to(Point::new(plot.x1, plot.y1))
        .clone();
    cx.stroke_path(&x_axis, &Stroke::new(1.0, Color::rgb(120, 120, 120)));
    let y_axis = Path::new()
        .move_to(Point::new(plot.x0, plot.y0))
        .line_to(Point::new(plot.x0, plot.y1))
        .clone();
    cx.stroke_path(&y_axis, &Stroke::new(1.0, Color::rgb(120, 120, 120)));

    // Data: six quarters as fractions of the plot height, read live. The last
    // one peaks above the canvas's top edge — outside the widget's own box,
    // which is the edge its declared clip cuts at.
    let values = data.get();
    let xs = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let at = |i: usize| {
        Point::new(
            plot.x0 + xs[i] * plot.width(),
            plot.y1 - values[i] * plot.height(),
        )
    };

    // Area fill under the line, down to the x-axis, translucent.
    let mut area = Path::new();
    area.move_to(Point::new(at(0).x, plot.y1));
    for i in 0..xs.len() {
        area.line_to(at(i));
    }
    area.line_to(Point::new(at(xs.len() - 1).x, plot.y1));
    area.close();
    cx.fill_path(&area, Color::rgba(30, 90, 200, 60));

    // The line itself, stroked on top.
    let mut line = Path::new();
    line.move_to(at(0));
    for i in 1..xs.len() {
        line.line_to(at(i));
    }
    cx.stroke_path(&line, &Stroke::new(2.0, Color::rgb(30, 90, 200)));
}

/// The scene the host retained carries what the closure drew.
fn act_1_the_closure_reaches_the_scene(host: &WidgetHost) {
    let (mut paths, mut fills, mut strokes) = (0usize, 0usize, 0usize);
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path { fill, stroke, .. } = node {
            paths += 1;
            fills += fill.is_some() as usize;
            strokes += stroke.is_some() as usize;
        }
    }
    println!("== 1. the closure reaches the scene ==");
    println!("  {paths} path nodes: {fills} with a fill, {strokes} with a stroke");
    assert_eq!(paths, 4, "two axes + one area fill + one data line");
    assert_eq!(fills, 1, "only the area under the line is filled");
    assert_eq!(
        strokes, 3,
        "the axes and the line are stroked, the area is not"
    );
}

/// The declared clip becomes a marker bracket around the canvas's subtree —
/// and it really cuts pixels, at the widget's own box.
fn act_2_the_host_brackets_and_the_clip_cuts(host: &WidgetHost) {
    let nodes: Vec<&SceneNode> = host.scene().iter().map(|(_, n)| n).collect();
    let pushes = nodes
        .iter()
        .filter(|n| matches!(n, SceneNode::ClipPush { .. }))
        .count();
    let pops = nodes
        .iter()
        .filter(|n| matches!(n, SceneNode::ClipPop))
        .count();
    println!("== 2. the clip is a bracket the host writes ==");
    println!("  {pushes} ClipPush / {pops} ClipPop in the retained scene");
    assert_eq!(pushes, 1, "the canvas declared one clip");
    assert_eq!(pops, 1, "and the host closed it");

    // Bracket order: the push precedes every path node and the pop follows it.
    let first_path = nodes
        .iter()
        .position(|n| matches!(n, SceneNode::Path { .. }))
        .expect("there are path nodes");
    let push_at = nodes
        .iter()
        .position(|n| matches!(n, SceneNode::ClipPush { .. }))
        .unwrap();
    let pop_at = nodes
        .iter()
        .rposition(|n| matches!(n, SceneNode::ClipPop))
        .unwrap();
    assert!(
        push_at < first_path && pop_at > first_path,
        "the bracket surrounds the subtree: push {push_at} < path {first_path} < pop {pop_at}"
    );

    // Pixels. The renderer gets the same scene the host retained.
    let rgba = render_rgba(host);

    // The last data point (fraction 1.3) peaks above the canvas's top edge.
    // Near the right edge the unclipped line and the fill under it cross the
    // 24px band of surface above the canvas box; probe there — inside the
    // surface, outside the clip. (The probe sits just under where the line
    // crosses the band, in the fill that an unclipped render would leave.)
    let probe_x = (MARGIN + 0.98 * (SURFACE_W as f64 - 2.0 * MARGIN)) as u32;
    let probe_y = 20;
    let clipped = rgba.pixel_rgba(probe_x, probe_y).expect("in bounds");
    assert_eq!(
        clipped,
        [0, 0, 0, 0],
        "outside the clip nothing is painted: the peak never reaches the band above the canvas"
    );

    // Control: the chart really is on the canvas — its colour shows up inside
    // the plot area near the axis, where the data sits.
    let inside = (
        (MARGIN + 10.0) as u32,
        (CANVAS_TOP as f64 + CANVAS_H as f64 - MARGIN - 2.0) as u32,
    );
    let px = rgba.pixel_rgba(inside.0, inside.1).expect("in bounds");
    assert!(
        px[3] > 0,
        "control: inside the plot the chart painted ({px:?})"
    );
    println!("  the band above the canvas is untouched; the plot itself is not");
}

/// A drawn surface publishes what it is, not just its pixels.
fn act_3_a_drawn_surface_says_what_it_is(host: &WidgetHost, root: NodeId) {
    let hint = host
        .semantics_hint(root)
        .expect("a labelled canvas publishes a hint");
    println!("== 3. the canvas says what it is ==");
    println!(
        "  role: {:?}, label: {:?}",
        hint.role,
        hint.label.as_deref()
    );
    assert_eq!(
        hint.role,
        floom::floom_core::SemanticsRole::Image,
        "a chart is an image to assistive tech"
    );
    assert_eq!(hint.label.as_deref(), Some("Quarterly revenue"));
}

/// Data made geometry — and the ping is what carries it to the screen.
///
/// The painter reads a reactive value, so this is the full chain with an
/// honest control at every joint: a value change alone wakes nothing, the
/// ping wakes the loop, the drawn vertex moves to where the new value says,
/// and the clip still cuts — a property of the surface, not of one dataset.
fn act_4_data_made_geometry(
    host: &mut WidgetHost,
    data: SyncRwSignal<Vec<f64>>,
    ping: RwSignal<()>,
) {
    println!("== 4. data made geometry ==");
    assert!(!host.needs_frame(), "the loop is parked between acts");
    let before = chart_polyline(host);

    // Control: the value moved, but nothing asked for a frame. The frame loop
    // draws when something requests it, and the chart has not requested.
    data.update(|v| v[2] = 1.3);
    assert!(
        !host.needs_frame(),
        "a data change with no repaint request wakes nothing"
    );

    // The ping turns "the data changed" into "draw a frame".
    ping.set(());
    assert!(
        host.needs_frame(),
        "the ping woke the loop for the new data"
    );
    host.frame();

    // The drawn polyline followed: the third vertex jumped from mid-plot to
    // above the clip's top edge — visible in the scene, held back in pixels.
    let after = chart_polyline(host);
    println!(
        "  third vertex: {} -> {}",
        fmt_point(before[2]),
        fmt_point(after[2])
    );
    assert!(
        after[2].y < CANVAS_TOP as f64,
        "the spike pulled the vertex above the clip's top edge"
    );
    // And it moved to exactly where the new value says: fraction 1.3 of the
    // plot height above the plot's bottom edge.
    let expected_y =
        (CANVAS_TOP + CANVAS_H) as f64 - MARGIN - 1.3 * (CANVAS_H as f64 - 2.0 * MARGIN);
    assert!(
        (after[2].y - expected_y).abs() < 0.5,
        "the vertex really moved up to where the new value says: {} vs {expected_y}",
        after[2].y
    );

    // The clip cuts the new dataset too. Re-render: in the spike's own column
    // the band above the canvas stays clean, just inside the box it painted.
    let rgba = render_rgba(host);
    let probe_x = (MARGIN + 0.4 * (SURFACE_W as f64 - 2.0 * MARGIN)) as u32;
    let clipped = rgba.pixel_rgba(probe_x, 12).expect("in bounds");
    assert_eq!(
        clipped,
        [0, 0, 0, 0],
        "the clip cuts the new data: the spike never reaches the band above the canvas"
    );
    let px = rgba
        .pixel_rgba(probe_x, (CANVAS_TOP + 6.0) as u32)
        .expect("in bounds");
    assert!(
        px[3] > 0,
        "control: just inside the clip it painted ({px:?})"
    );
    println!("  the ping drew the new data; the clip cut it at the same edge");
}

/// The chart's data line: the one stroke-only path with six vertices
/// (the axes have two each, the area fill is not stroked).
fn chart_polyline(host: &WidgetHost) -> Vec<Point> {
    for (_, node) in host.scene().iter() {
        if let SceneNode::Path {
            path,
            fill: None,
            stroke: Some(_),
        } = node
        {
            let vertices: Vec<Point> = path
                .segments()
                .iter()
                .filter_map(|seg| match seg {
                    PathSeg::MoveTo(p) | PathSeg::LineTo(p) => Some(*p),
                    _ => None,
                })
                .collect();
            if vertices.len() == 6 {
                return vertices;
            }
        }
    }
    panic!("the chart's polyline is not in the scene");
}

fn fmt_point(p: Point) -> String {
    format!("[{:.0},{:.0}]", p.x, p.y)
}

/// Render the host's retained scene through the software rasteriser.
fn render_rgba(host: &WidgetHost) -> floom::floom_renderer_software::SoftwareRenderer {
    let mut sw =
        floom::floom_renderer_software::SoftwareRenderer::new(SURFACE_W as u32, SURFACE_H as u32);
    floom_renderer::render_once(
        &mut sw,
        &floom_renderer::FrameInfo::software(SURFACE_W as u32, SURFACE_H as u32, 1),
        host.scene(),
    );
    sw
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
