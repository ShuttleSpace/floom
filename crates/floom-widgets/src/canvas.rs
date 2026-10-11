//! Self-drawn surfaces: a closure-driven [`Canvas`] and a [`Spinner`].
//!
//! Both exist for the same reason — the host paints a *flat* scene in node
//! order, so a widget that wants to draw a chart, a ring, or a spinner arc
//! cannot push a clip or a transform around its own subtree. What it can do is
//! receive a [`PaintContext`] and call `fill_path` / `stroke_path` (A1/A2), and
//! ask the host to animate one channel of its element (A3) which the widget
//! reads back as `animation_value` and turns into geometry. That is exactly the
//! shape `Canvas` and `Spinner` are.

use floom_core::{
    AnimationSpec, PaintContext, SemanticsContext, SemanticsHint, SemanticsRole, UpdateContext,
    Widget, WidgetAny,
};
use floom_signal::{Effect, RwSignal, SignalGet};
use floom_types::{Color, Easing, ElementId, Path, Point, RepeatSpec, Shader, Size, Stroke};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// A box for the user's draw routine.
///
/// `Canvas` owns the box, not a generic, so the public type stays
/// monomorphic: a caller hands in a closure and never names a lifetime or a
/// signal type. The closure is what reads the caller's state at paint time —
/// see [`Canvas::repaint_on`] for how a change to one of its signals gets a
/// frame scheduled.
type Painter = dyn Fn(&mut dyn PaintContext) + Send + Sync;

/// A widget whose only job is to run a draw routine against a [`PaintContext`].
///
/// `Canvas` is the escape hatch for "none of the primitives draw what I want":
/// a line chart, a waveform, a progress ring, a custom gauge. The routine
/// receives the context, whose `bounds()` is this element's laid-out box, and
/// draws into it with the path/stroke primitives (A1). Because the host paints
/// flat, the routine draws *only* this element — children, if any, are painted
/// by the host around it, not by the closure.
///
/// **Repaint on change.** A `Canvas` is re-painted whenever its element is
/// marked dirty. Two things mark it:
///
/// * A builder mutation (`size`, `clip`, `role`, `label`, …) bumps the
///   revision, and the next `update` reports dirty — the path every widget
///   uses.
/// * [`Canvas::repaint_on`] subscribes a reactive effect to an `RwSignal<()>`
///   ping: bumping the ping from app code enqueues a paint-only repaint
///   request for this element (`floom_core::request_repaint`), which the host
///   folds into the dirty flag at the top of its next frame. That is the
///   whole "my data changed, redraw the chart" path — and it never relayouts.
///
/// The draw routine may also capture signals and read them while it paints,
/// but a signal read inside `paint` schedules nothing by itself — pair it
/// with `repaint_on` to get the frame scheduled.
#[derive(Clone)]
pub struct Canvas {
    painter: Arc<Painter>,
    /// Intrinsic size, in logical px. `None` means "shrink to what the parent
    /// gives me" — the host still reports the laid-out box to `bounds()`, so a
    /// canvas with no intrinsic size fills its slot.
    intrinsic: Option<Size>,
    /// Optional signal ping whose change schedules a repaint.
    repaint: Option<RwSignal<()>>,
    /// Set once the ping's reactive effect exists, so two clones of one
    /// canvas (the caller's and the tree's) cannot create — and double-
    /// subscribe — the effect twice.
    wired: Arc<AtomicBool>,
    /// The clip the canvas declares over its whole subtree, resolved against
    /// its laid-out box by the host. A chart uses this to keep its plot area
    /// from spilling over its axes; without it there is no way for a flat
    /// scene to confine self-drawn geometry.
    clip_shape: Option<floom_types::ClipShape>,
    /// Accessibility role the canvas publishes. Defaults to [`SemanticsRole::Image`]
    /// because a drawn surface is, to assistive tech, an image — the same
    /// default Compose and Flutter landed on after discovering that an
    /// invisible canvas was worse than a labelled one.
    role: SemanticsRole,
    /// Optional accessible label.
    label: Option<String>,
    revision: crate::primitives::Revision,
}

impl Canvas {
    /// A canvas that fills whatever box it is given, drawing with `painter`.
    pub fn new(painter: impl Fn(&mut dyn PaintContext) + Send + Sync + 'static) -> Self {
        Self {
            painter: Arc::new(painter),
            intrinsic: None,
            repaint: None,
            wired: Arc::new(AtomicBool::new(false)),
            clip_shape: None,
            role: SemanticsRole::Image,
            label: None,
            revision: crate::primitives::Revision::default(),
        }
    }

    /// Declares the clip the canvas's whole subtree is confined to, resolved
    /// against its laid-out box by the host ([`Widget::clip`]). A chart uses
    /// this to keep its plot area from spilling over its axes; a flat scene
    /// has no other way to confine self-drawn geometry.
    pub fn clip(mut self, shape: floom_types::ClipShape) -> Self {
        self.clip_shape = Some(shape);
        self.revision.bump();
        self
    }

    /// Intrinsic size in logical px. Without it the canvas fills its parent's
    /// slot; with it the host sizes this element to the request.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.intrinsic = Some(Size::new(width as f64, height as f64));
        self.revision.bump();
        self
    }

    /// Schedules a repaint whenever `ping` is bumped: an effect created on the
    /// next `update` reads the ping (registering the dependency) and, on every
    /// change after that first read, enqueues a paint-only repaint request for
    /// this element, which the host serves on its next frame.
    pub fn repaint_on(mut self, ping: RwSignal<()>) -> Self {
        self.repaint = Some(ping);
        self.revision.bump();
        self
    }

    /// Publishes `role` to assistive tech (default: [`SemanticsRole::Image`]).
    pub fn role(mut self, role: SemanticsRole) -> Self {
        self.role = role;
        self.revision.bump();
        self
    }

    /// A human-readable description of what the canvas shows.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    /// Equivalent to `role(SemanticsRole::None)`: the canvas draws nothing worth
    /// describing to assistive tech.
    pub fn decorative(mut self) -> Self {
        self.role = SemanticsRole::None;
        self.label = None;
        self.revision.bump();
        self
    }

    /// Creates the ping's effect exactly once per canvas instance. Runs from
    /// `update`, where the element id this canvas paints under is known —
    /// the repaint request has to name it. The effect's first run exists to
    /// collect dependencies, not to report a change, so it enqueues nothing;
    /// every subsequent ping change enqueues one request.
    fn wire_repaint(&mut self, id: ElementId) {
        if self.wired.swap(true, Ordering::Relaxed) {
            return;
        }
        let Some(ping) = self.repaint else {
            return;
        };
        Effect::new(move |prev: Option<()>| {
            if prev.is_some() {
                floom_core::request_repaint(id);
            }
            ping.get()
        });
    }

    fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint {
            role: self.role,
            label: self.label.clone(),
            state: floom_core::SemanticsState::empty(),
        }
    }
}

impl Widget for Canvas {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // The ping's dependency lives in its reactive effect (see
        // `wire_repaint`), not here: reading the ping during `update` would
        // register nothing, because the host runs `update` outside any
        // reactive scope — there is no effect to subscribe.
        self.wire_repaint(id);
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let size = self.intrinsic?;
        if size.width <= 0.0 || size.height <= 0.0 {
            return None;
        }
        Some(
            floom_core::LayoutRequest::new()
                .width(size.width as f32)
                .height(size.height as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        (self.painter)(cx);
    }

    fn clip(&self) -> Option<floom_types::ClipShape> {
        self.clip_shape.clone()
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        // The role is published through the semantics channel rather than only
        // through `emit_semantics`' skip-check, so a recorder (and the host)
        // sees *what* the canvas is, not just whether it has a label. A canvas
        // with no role and no label is decorative and publishes nothing.
        if self.role == SemanticsRole::None && self.label.is_none() {
            return;
        }
        cx.set_role(self.role);
        if let Some(label) = self.label.as_deref() {
            cx.set_label(label);
        }
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for Canvas {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        // A decorative canvas publishes nothing; everything else publishes its
        // role (and label) for the host to lower into the accessibility tree.
        if self.role == SemanticsRole::None && self.label.is_none() {
            None
        } else {
            Some(self.semantics_hint())
        }
    }
}

// ---------------------------------------------------------------------------
// ShaderBox — a box whose ink is a caller-supplied WGSL program (A5)
// ---------------------------------------------------------------------------

/// A box filled by a custom WGSL fragment shader — the widget-facing half of
/// the custom-shader gap (A5).
///
/// Where [`Canvas`] hands the caller a [`PaintContext`] to draw paths into,
/// `ShaderBox` hands the box to a *program*: the shader is the element's whole
/// ink, not decoration on top of it, so nothing is filled behind it. That is
/// the shape a gradient-free procedural background, a plasma, a GPU-side
/// noise field or a ported Shadertoy effect needs — content no path or brush
/// can express, but one fragment function can.
///
/// **Animation is free.** The host advances `u.time` by one nominal frame per
/// present (see [`floom_types::Shader`]), so a shader that reads it animates
/// without the widget rebuilding anything: no repaint request, no re-layout,
/// no per-frame allocation. [`Self::set_params`] is for the other case — the
/// caller's own scalars changing — and does ask for a frame.
///
/// **Degradation is recorded, not silent.** A backend with no GPU program
/// pipeline runs nothing here. The software renderer counts the skip
/// (`shaders_skipped`) and vello counts what it actually drew
/// (`shaders_drawn`), so an app can tell "my shader is running" from "my
/// shader was dropped" instead of staring at an empty box.
#[derive(Clone)]
pub struct ShaderBox {
    shader: Shader,
    intrinsic: Option<Size>,
    role: SemanticsRole,
    label: Option<String>,
    revision: crate::primitives::Revision,
}

impl std::fmt::Debug for ShaderBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShaderBox")
            .field("wgsl_len", &self.shader.wgsl.len())
            .field("params", &self.shader.params)
            .finish()
    }
}

impl ShaderBox {
    /// A box whose ink is `shader`.
    pub fn new(shader: Shader) -> Self {
        Self {
            shader,
            intrinsic: None,
            role: SemanticsRole::Image,
            label: None,
            revision: crate::primitives::Revision::default(),
        }
    }

    /// A box from WGSL source and caller scalars, without naming [`Shader`].
    pub fn wgsl(wgsl: impl Into<String>, params: impl Into<Vec<f32>>) -> Self {
        Self::new(Shader::new(wgsl, params))
    }

    /// Intrinsic size in logical px. Without it the box fills its parent's
    /// slot; with it the host sizes this element to the request.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.intrinsic = Some(Size::new(width as f64, height as f64));
        self.revision.bump();
        self
    }

    /// Replaces the caller scalars (`u.params[0]`/`u.params[1]`).
    pub fn params(mut self, params: impl Into<Vec<f32>>) -> Self {
        self.shader.params = params.into();
        self.revision.bump();
        self
    }

    /// Publishes `role` to assistive tech (default: [`SemanticsRole::Image`],
    /// the same default [`Canvas`] landed on — a drawn surface is, to assistive
    /// tech, an image).
    pub fn role(mut self, role: SemanticsRole) -> Self {
        self.role = role;
        self.revision.bump();
        self
    }

    /// A human-readable description of what the shader shows.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    /// Equivalent to `role(SemanticsRole::None)`: the box draws nothing worth
    /// describing to assistive tech.
    pub fn decorative(mut self) -> Self {
        self.role = SemanticsRole::None;
        self.label = None;
        self.revision.bump();
        self
    }

    /// Swaps the whole program on the mounted widget (the tree's copy, reached
    /// through `widget_mut`). Mutating a *builder* is a silent no-op — the
    /// builder is cloned into the tree, so this is the in-place form.
    pub fn set_shader(&mut self, shader: Shader) {
        self.shader = shader;
        self.revision.bump();
    }

    /// Replaces just the caller scalars on the mounted widget.
    pub fn set_params(&mut self, params: impl Into<Vec<f32>>) {
        self.shader.params = params.into();
        self.revision.bump();
    }

    /// The program this box draws.
    pub fn shader(&self) -> &Shader {
        &self.shader
    }

    fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint {
            role: self.role,
            label: self.label.clone(),
            state: floom_core::SemanticsState::empty(),
        }
    }
}

impl Widget for ShaderBox {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let size = self.intrinsic?;
        if size.width <= 0.0 || size.height <= 0.0 {
            return None;
        }
        Some(
            floom_core::LayoutRequest::new()
                .width(size.width as f32)
                .height(size.height as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        // The host drops a zero-area box on its own; asking for a pass over no
        // pixels would only add a skip count nothing can act on.
        if cx.bounds().width() <= 0.0 || cx.bounds().height() <= 0.0 {
            return;
        }
        cx.draw_shader(&self.shader);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        if self.role == SemanticsRole::None && self.label.is_none() {
            return;
        }
        cx.set_role(self.role);
        if let Some(label) = self.label.as_deref() {
            cx.set_label(label);
        }
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for ShaderBox {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        if self.role == SemanticsRole::None && self.label.is_none() {
            None
        } else {
            Some(self.semantics_hint())
        }
    }
}

/// A spinning arc that sweeps one way forever — the loading glyph.
///
/// The arc is self-drawn (A1/A7): the widget declares a repeating,
/// prop-less tween and reads its progress back as `animation_value`, then
/// strokes a partial [`Path::ellipse_arc`] rotated by that progress. Because
/// the host paints a flat scene there is no rotation transform to hand the
/// renderer, so the rotation is geometry — the arc's start angle advances each
/// frame, which is the one motion a spinner needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spinner {
    size: f32,
    line_width: f32,
    color: Color,
    track: Option<Color>,
    period_secs: f32,
    revision: crate::primitives::Revision,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            size: 24.0,
            line_width: 3.0,
            color: Color::rgb(0, 0, 0),
            track: None,
            period_secs: 1.0,
            revision: crate::primitives::Revision::default(),
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self.revision.bump();
        self
    }

    pub fn line_width(mut self, width: f32) -> Self {
        self.line_width = width;
        self.revision.bump();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self.revision.bump();
        self
    }

    /// A faint full circle drawn behind the sweeping arc.
    pub fn track(mut self, color: Color) -> Self {
        self.track = Some(color);
        self.revision.bump();
        self
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Spinner {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        if self.size <= 0.0 {
            return None;
        }
        Some(
            floom_core::LayoutRequest::new()
                .width(self.size)
                .height(self.size),
        )
    }

    fn animation(&self) -> Option<AnimationSpec> {
        // An endless, linear sweep. `prop` is `None` because nothing about the
        // element as a whole moves — only the arc geometry, which the widget
        // recomputes from `animation_value` each frame.
        Some(
            AnimationSpec::repeating(
                0.0,
                1.0,
                self.period_secs as f64,
                RepeatSpec {
                    mode: floom_core::RepeatMode::Restart,
                    iterations: None,
                },
            )
            .with_easing(Easing::Linear),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let center = Point::new(
            bounds.x0 + bounds.width() / 2.0,
            bounds.y0 + bounds.height() / 2.0,
        );
        let radius = (bounds.width().min(bounds.height()) / 2.0) - self.line_width as f64;

        // The track is a full ring so the spinner reads as "progress pending"
        // even before the sweep arrives.
        if let Some(track) = self.track {
            let full = Path::circle(center, radius);
            cx.stroke_path(
                &full,
                &Stroke::new(self.line_width as f64, track).with_cap(floom_types::LineCap::Round),
            );
        }

        // Sweep fraction grows from a sliver to almost a full turn as the cycle
        // advances; restarting each cycle is what gives the "jump back and go
        // again" motion. The start angle advances so the arc travels round.
        let progress = cx.animation_value().unwrap_or(0.0).clamp(0.0, 1.0);
        let sweep = std::f64::consts::TAU * (0.15 + 0.75 * progress);
        let start = progress * std::f64::consts::TAU;
        let arc = Path::ellipse_arc(center, radius, radius, start, sweep);
        cx.stroke_path(
            &arc,
            &Stroke::new(self.line_width as f64, self.color).with_cap(floom_types::LineCap::Round),
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        cx.set_role(SemanticsRole::Image);
        cx.set_label("Loading");
        cx.set_bounds(cx.bounds());
    }
}

impl WidgetAny for Spinner {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        Some(SemanticsHint::labeled(SemanticsRole::Image, "Loading"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{SceneRecorder, SemanticsRecorder};
    use floom_types::Rect;

    fn id() -> ElementId {
        ElementId::new(1, 1)
    }

    fn box_(w: f64, h: f64) -> Rect {
        Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(w, h))
    }

    /// A8: a self-drawn surface must not be invisible to assistive tech. The
    /// default role is `Image` — the same default Compose and Flutter landed
    /// on — published through the semantics channel (`set_role`), not just
    /// through the hint's skip-check.
    #[test]
    fn a_canvas_defaults_to_the_image_role() {
        let canvas = Canvas::new(|_| {});
        let mut rec = SemanticsRecorder::new(box_(100.0, 80.0));
        canvas.semantics(&mut rec, id());
        assert_eq!(rec.role(), SemanticsRole::Image);
        assert_eq!(rec.label(), None);
        // The role also rides the hint, which is what the host lowers into the
        // accessibility tree.
        let hint = WidgetAny::semantics_hint(&canvas).expect("a default canvas is an image");
        assert_eq!(hint.role, SemanticsRole::Image);
    }

    /// Control: the decorative canvas is the one case that publishes nothing.
    #[test]
    fn a_decorative_canvas_publishes_nothing() {
        let canvas = Canvas::new(|_| {}).decorative();
        let mut rec = SemanticsRecorder::new(box_(100.0, 80.0));
        canvas.semantics(&mut rec, id());
        assert_eq!(rec.role(), SemanticsRole::None);
        assert_eq!(rec.label(), None);
        assert!(WidgetAny::semantics_hint(&canvas).is_none());
    }

    #[test]
    fn a_labelled_canvas_reports_both_role_and_label() {
        let canvas = Canvas::new(|_| {}).label("Revenue chart");
        let mut rec = SemanticsRecorder::new(box_(100.0, 80.0));
        canvas.semantics(&mut rec, id());
        assert_eq!(rec.role(), SemanticsRole::Image);
        assert_eq!(rec.label(), Some("Revenue chart"));
    }

    /// The painter runs against the real paint context: whatever it draws is
    /// recorded, so a canvas is not a black hole.
    #[test]
    fn a_canvas_paints_through_its_closure() {
        let canvas = Canvas::new(|cx| {
            let path = Path::rect(cx.bounds());
            cx.fill_path(&path, Color::rgb(1, 2, 3));
        });
        let mut rec = SceneRecorder::new(box_(40.0, 30.0));
        canvas.paint(&mut rec, id());
        rec.finish();
        let ops = rec.ops();
        assert_eq!(ops.len(), 1, "the closure drew exactly one thing: {ops:?}");
        match &ops[0] {
            crate::testkit::PaintOp::Path {
                fill: Some(color),
                stroke: None,
                ..
            } => assert_eq!(*color, Color::rgb(1, 2, 3)),
            other => panic!("expected a filled path op, got {other:?}"),
        }
    }

    /// A7/A3: the spinner is a B3 loop — an endless, prop-less cycle the
    /// widget reads back as `animation_value` and turns into arc geometry.
    #[test]
    fn a_spinner_declares_an_endless_cycle() {
        let spinner = Spinner::new();
        let spec = spinner
            .animation()
            .expect("a spinner is always animating while mounted");
        let repeat = spec.repeat.expect("the sweep repeats");
        assert_eq!(
            repeat.iterations, None,
            "endless — this is what keeps a frame loop awake"
        );
        assert_eq!(repeat.mode, floom_types::RepeatMode::Restart);
        assert!(spec.channels.is_empty(), "the arc moves, not the element");
    }

    /// The sweep is really drawn: one stroked arc, no fill, per frame.
    #[test]
    fn a_spinner_strokes_an_arc_and_nothing_else() {
        let spinner = Spinner::new().color(Color::rgb(9, 8, 7));
        let mut rec = SceneRecorder::new(box_(24.0, 24.0));
        spinner.paint(&mut rec, id());
        rec.finish();
        let ops = rec.ops();
        assert_eq!(ops.len(), 1, "no track by default: {ops:?}");
        match &ops[0] {
            crate::testkit::PaintOp::Path {
                fill: None,
                stroke: Some((width, color)),
                ..
            } => {
                assert_eq!(*color, Color::rgb(9, 8, 7));
                assert!(*width > 0.0);
            }
            other => panic!("expected a stroked path op, got {other:?}"),
        }
    }

    /// The animation value reaches the geometry, through *both* clauses that
    /// depend on it. Movement alone is not enough evidence — the arc's start
    /// angle also advances with the cycle, so an implementation that froze the
    /// sweep length and only rotated would still move. The end of the cycle is
    /// what isolates the sweep: at `progress = 1.0` the sweep covers more than
    /// three quarters of a turn from angle 0, so it reaches every extreme and
    /// the arc's bounds must be the *full circle's* box (within flattening
    /// tolerance). A frozen sweep of half a turn starting at angle 0 covers
    /// only the top half and its box is half as tall.
    #[test]
    fn the_spinner_sweep_reads_the_animation_value() {
        let spinner = Spinner::new();
        let bounds_at = |value: f64| {
            let mut rec = SceneRecorder::new(box_(24.0, 24.0)).with_animation_value(value);
            spinner.paint(&mut rec, id());
            rec.finish();
            match &rec.ops()[0] {
                crate::testkit::PaintOp::Path { bounds, .. } => *bounds,
                other => panic!("expected a path op, got {other:?}"),
            }
        };
        let start = bounds_at(0.0);
        let mid = bounds_at(0.5);
        assert_ne!(
            start, mid,
            "half a cycle later the arc is somewhere else: {start:?} vs {mid:?}"
        );
        // The sweep-length clause, isolated at the end of the cycle.
        let full = bounds_at(1.0);
        let diameter = full.width().max(full.height());
        let other_extent = full.width().min(full.height());
        assert!(
            diameter - other_extent < 1.0,
            "a >3/4-turn sweep covers every extreme, so the arc's box is the              full circle's: {full:?}"
        );
    }

    /// Control for the sweep: with a track declared, the ring behind the arc is
    /// a second, distinct op.
    #[test]
    fn a_spinner_with_a_track_draws_the_ring_behind_the_arc() {
        let spinner = Spinner::new().track(Color::rgb(200, 200, 200));
        let mut rec = SceneRecorder::new(box_(24.0, 24.0));
        spinner.paint(&mut rec, id());
        rec.finish();
        let ops = rec.ops();
        assert_eq!(ops.len(), 2, "track then arc: {ops:?}");
        assert!(
            matches!(
                &ops[0],
                crate::testkit::PaintOp::Path {
                    stroke: Some(_),
                    ..
                }
            ),
            "first op is the track ring"
        );
    }
}
