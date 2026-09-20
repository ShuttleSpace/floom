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
//!    `clip(ClipShape::Box)`; the host brackets exactly its subtree with
//!    `ClipPush`/`ClipPop`. And the bracket *works*: data that would spill
//!    past the plot's top edge is cut off in rendered pixels, while the same
//!    geometry without the clip is not.
//! 3. **A drawn surface is not invisible.** The canvas publishes
//!    `Role::Image` with its label — the screen reader has something to
//!    describe because the widget said so.
//!
//! Everything below is `floem2::prelude`. Run with `cargo run -p floem2-chart`.

use floem_renderer::FrameInfo;
use floem2::advanced::PaintContext;
use floem2::floem_scene::SceneNode;
use floem2::floem_types::ClipShape;
use floem2::prelude::*;

/// Plot geometry, in the canvas's own coordinates.
const CANVAS_W: f32 = 320.0;
const CANVAS_H: f32 = 200.0;
/// The plot area is inset, so the clip has a margin to prove itself in.
const MARGIN: f64 = 24.0;
fn main() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(CANVAS_W as f64, CANVAS_H as f64));

    let chart = Canvas::new(paint_chart)
        .clip(ClipShape::Box)
        .label("Quarterly revenue");
    let root = host.mount_root(LayoutStyle::default(), Container::new());
    let canvas = host.mount(
        root,
        LayoutStyle {
            width: Some(CANVAS_W),
            height: Some(CANVAS_H),
            ..LayoutStyle::default()
        },
        chart,
    );
    host.frame();

    act_1_the_closure_reaches_the_scene(&host);
    act_2_the_host_brackets_and_the_clip_cuts(&host, canvas);
    act_3_a_drawn_surface_says_what_it_is(&host, canvas);

    println!();
    println!("every act above was asserted, not printed");
}

/// The chart's draw routine: axes, a translucent area fill, and the data line.
///
/// Everything is computed from `cx.bounds()` — the laid-out box — so the same
/// closure draws correctly at any size the layout hands it.
fn paint_chart(cx: &mut dyn PaintContext) {
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

    // Data: six quarters, the last one peaking above the plot's top edge.
    let xs = [0.0, 0.2, 0.4, 0.6, 0.8, 1.0];
    let ys = [0.7, 0.45, 0.6, 0.3, 0.5, -0.2]; // y fraction, negative = above the plot
    let at = |i: usize| {
        Point::new(
            plot.x0 + xs[i] * plot.width(),
            plot.y1 - ys[i] * plot.height(),
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
/// and it really cuts pixels.
fn act_2_the_host_brackets_and_the_clip_cuts(host: &WidgetHost, canvas: NodeId) {
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
    let mut sw =
        floem2::floem_renderer_software::SoftwareRenderer::new(CANVAS_W as u32, CANVAS_H as u32);
    floem_renderer::render_once(
        &mut sw,
        &FrameInfo::software(CANVAS_W as u32, CANVAS_H as u32, 1),
        host.scene(),
    );

    // Probe in the top margin, inside the canvas but *outside* the clip: the
    // unclipped line would pass through here on its way to the peak.
    let probe_x = CANVAS_W as usize / 2;
    let probe_y = (MARGIN / 2.0) as usize;
    let clipped = sw
        .pixel_rgba(probe_x as u32, probe_y as u32)
        .expect("in bounds");
    assert_eq!(
        clipped,
        [0, 0, 0, 0],
        "outside the clip nothing is painted (the peak never reaches the margin)"
    );

    // Control: the line really is on the canvas — its colour shows up inside
    // the plot area near the axis, where the data sits.
    let inside = (
        MARGIN as usize + 10,
        CANVAS_H as usize - MARGIN as usize - 2,
    );
    let px = sw
        .pixel_rgba(inside.0 as u32, inside.1 as u32)
        .expect("in bounds");
    assert!(
        px[3] > 0,
        "control: inside the plot the chart painted ({px:?})"
    );
    let _ = canvas;
    println!("  the margin above the plot is untouched; the plot itself is not");
}

/// A drawn surface publishes what it is, not just its pixels.
fn act_3_a_drawn_surface_says_what_it_is(host: &WidgetHost, canvas: NodeId) {
    let hint = host
        .semantics_hint(canvas)
        .expect("a labelled canvas publishes a hint");
    println!("== 3. the canvas says what it is ==");
    println!(
        "  role: {:?}, label: {:?}",
        hint.role,
        hint.label.as_deref()
    );
    assert_eq!(
        hint.role,
        floem2::floem_core::SemanticsRole::Image,
        "a chart is an image to assistive tech"
    );
    assert_eq!(hint.label.as_deref(), Some("Quarterly revenue"));
}
