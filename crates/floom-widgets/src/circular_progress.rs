//! `CircularProgress` — a determinate or indeterminate progress ring.
//!
//! ## Why this is in the engine, and why it is mostly parameters
//!
//! Measured across `md_progress_circular`, `cupertino_progress_circular` and
//! `fluent_progress_circular`: the three agree on the tokens (`size`, and
//! `stroke_width` in all but the Cupertino override) and on both colours
//! (`surface_variant` track, `primary` fill), and disagree about the shape of
//! the *indeterminate* arc and where it starts. So this is not one component
//! with three skins, and the file is mostly a list of the ways they differ.
//!
//! ## The disagreements, as parameters rather than as prose
//!
//! | | Material | Cupertino | Fluent |
//! |---|---|---|---|
//! | indeterminate arc | **30%, rotating** | 25%, static | 25%, static |
//! | indeterminate start | 12 o'clock | 12 o'clock | **3 o'clock** |
//! | determinate start | 12 o'clock | 12 o'clock | 12 o'clock |
//!
//! Each row is a field on [`CircularProgressStyle`], and each is pinned by a
//! test in the kit that wants it.
//!
//! ## The rotation is the caller's, not this crate's
//!
//! Material's indeterminate arc rotates continuously, and v1 did it by spawning
//! a thread that wrote to a signal every 16 milliseconds -- which is how a
//! background thread ends up driving a widget. 2.0 has an animation channel
//! that a *host* drives, and this widget does not own a clock, so the rotation
//! is a value the caller sets through [`CircularProgress::set_rotation`].
//!
//! Nothing inside this crate advances it, for the same reason
//! [`crate::progress_linear`] does not advance its sliding segment: the driver
//! belongs at the host, not here. Until one exists, Material's ring sits at
//! twelve o'clock, which is Cupertino's and Fluent's behaviour -- and is
//! visibly wrong for Material, in the same way and for the same reason.
//!
//! ## The radius
//!
//! v1's Cupertino wrote `r = (size - sw).max(1.0) / 2.0` and the other two
//! wrote `r = size / 2.0 - sw / 2.0`. The two agree for any ring wider than its
//! stroke, which is every ring a person sees; they differ only for a ring
//! narrower than its own stroke, where the other two go negative and the
//! engine's `ellipse_arc` takes the absolute value. The clamped form is used
//! here, so the degenerate case is a hairline rather than an inverted ring.

use floom_accessibility::SemanticsRole;
use floom_core::{
    CreateContext, ElementId, LayoutRequest, PaintContext, SemanticsContext, UpdateContext, Widget,
};
use floom_types::{Color, LineCap, Path, Point, Stroke};

use crate::primitives::{Revision, SemanticsHint, emit_semantics};

/// The knobs the three kits disagree about.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CircularProgressStyle {
    /// The fraction of the ring the indeterminate arc covers.
    ///
    /// 0.25 for Cupertino and Fluent, 0.30 for Material.
    pub indeterminate_fraction: f64,
    /// Where the *determinate* arc starts, in radians.
    ///
    /// Zero is at three o'clock -- the positive x axis, with y pointing down --
    /// and a quarter turn before zero is twelve o'clock, which is where all
    /// three kits start a determinate arc.
    pub start_rad: f64,
    /// Where the *indeterminate* arc starts, in radians.
    ///
    /// Twelve o'clock for Material and Cupertino, three o'clock for Fluent.
    pub indeterminate_start_rad: f64,
}

impl Default for CircularProgressStyle {
    fn default() -> Self {
        Self {
            indeterminate_fraction: 0.25,
            start_rad: -std::f64::consts::FRAC_PI_2,
            indeterminate_start_rad: -std::f64::consts::FRAC_PI_2,
        }
    }
}

/// A progress ring: a track, and an arc over it showing how far along it is.
#[derive(Debug, Clone, PartialEq)]
pub struct CircularProgress {
    /// The fraction complete, in `0.0..=1.0`. `None` is the indeterminate ring.
    value: Option<f64>,
    size: f64,
    stroke_width: f64,
    track_color: Color,
    fill_color: Color,
    style: CircularProgressStyle,
    /// The arc's rotation, in turns. Advanced by the caller.
    rotation: f64,
    revision: Revision,
}

impl CircularProgress {
    /// A determinate ring, `value` of the way round.
    pub fn determinate(value: f64) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            size: 48.0,
            stroke_width: 4.0,
            track_color: Color::rgba(0, 0, 0, 32),
            fill_color: Color::rgb(0x2B, 0x6C, 0xB0),
            style: CircularProgressStyle::default(),
            rotation: 0.0,
            revision: Revision::default(),
        }
    }

    /// The indeterminate ring: an arc of no particular length.
    pub fn indeterminate() -> Self {
        Self {
            value: None,
            ..Self::determinate(0.0)
        }
    }

    /// The ring's diameter, in logical pixels.
    pub fn size(mut self, size: f64) -> Self {
        self.size = size;
        self.revision.bump();
        self
    }

    /// The ring's stroke thickness, in logical pixels.
    pub fn stroke_width(mut self, stroke_width: f64) -> Self {
        self.stroke_width = stroke_width;
        self.revision.bump();
        self
    }

    /// The colour of the ring behind the arc.
    pub fn track_color(mut self, color: Color) -> Self {
        self.track_color = color;
        self.revision.bump();
        self
    }

    /// The colour of the arc.
    pub fn fill_color(mut self, color: Color) -> Self {
        self.fill_color = color;
        self.revision.bump();
        self
    }

    /// The style: the knobs the kits disagree about.
    pub fn style(mut self, style: CircularProgressStyle) -> Self {
        self.style = style;
        self.revision.bump();
        self
    }

    /// The fraction complete, or `None` while indeterminate.
    pub fn value(&self) -> Option<f64> {
        self.value
    }

    /// Sets the fraction complete. `None` makes the ring indeterminate.
    pub fn set_value(&mut self, value: Option<f64>) {
        let value = value.map(|v| v.clamp(0.0, 1.0));
        if value != self.value {
            self.value = value;
            self.revision.bump();
        }
    }

    /// The arc's rotation, in turns.
    pub fn rotation(&self) -> f64 {
        self.rotation
    }

    /// Rotates the arc. The caller drives this; nothing in this crate does.
    pub fn set_rotation(&mut self, turns: f64) {
        if turns != self.rotation {
            self.rotation = turns;
            self.revision.bump();
        }
    }

    /// The ring's radius: half the size, less half the stroke, and never less
    /// than a hairline.
    pub fn radius(&self) -> f64 {
        (self.size - self.stroke_width).max(1.0) / 2.0
    }

    /// The arc's start angle, in radians.
    ///
    /// The determinate arc and the indeterminate one start in different places,
    /// because Fluent's indeterminate arc starts at three o'clock while its
    /// determinate one starts at twelve.
    pub fn start_rad(&self) -> f64 {
        let base = match self.value {
            Some(_) => self.style.start_rad,
            None => self.style.indeterminate_start_rad,
        };
        base + std::f64::consts::TAU * self.rotation
    }

    /// The arc's sweep, in radians.
    ///
    /// A determinate ring sweeps its fraction of the circle; an indeterminate
    /// one sweeps the style's `indeterminate_fraction`.
    pub fn sweep_rad(&self) -> f64 {
        let fraction = match self.value {
            Some(value) => value,
            None => self.style.indeterminate_fraction,
        };
        std::f64::consts::TAU * fraction.clamp(0.0, 1.0)
    }

    /// The track: the whole ring, behind the arc.
    pub fn track_path(&self, center: Point) -> Path {
        Path::ellipse_arc(center, self.radius(), self.radius(), 0.0, std::f64::consts::TAU)
    }

    /// The arc: the part of the ring that shows the progress.
    pub fn arc_path(&self, center: Point) -> Path {
        Path::ellipse_arc(center, self.radius(), self.radius(), self.start_rad(), self.sweep_rad())
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        // A progress bar, announced as a percentage. The indeterminate ring
        // announces no value at all, because "25%" would be a lie about how far
        // along it is.
        match self.value {
            Some(value) => SemanticsHint::labeled(
                SemanticsRole::ProgressBar,
                format!("{}%", (value * 100.0) as i64),
            ),
            None => SemanticsHint::labeled(SemanticsRole::ProgressBar, "Indeterminate"),
        }
    }
}

impl Widget for CircularProgress {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width(self.size as f32)
                .height(self.size as f32),
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

        // The track: the whole ring, in the muted colour.
        cx.stroke_path(
            &self.track_path(center),
            &Stroke::new(self.stroke_width, self.track_color),
        );

        // The arc: the progress, with rounded ends -- v1's
        // `stroke-linecap="round"`.
        let sweep = self.sweep_rad();
        if sweep > 0.0 {
            let mut stroke = Stroke::new(self.stroke_width, self.fill_color);
            stroke.cap = LineCap::Round;
            cx.stroke_path(&self.arc_path(center), &stroke);
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
