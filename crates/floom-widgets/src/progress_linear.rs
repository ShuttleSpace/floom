//! `LinearProgress` — a determinate or indeterminate progress track.
//!
//! ## Why this is in the engine, and why it is mostly parameters
//!
//! Measured across `md_progress_linear`, `cupertino_progress_linear` and
//! `fluent_progress_linear`: **0.17 and 0.19 similarity** — the lowest of any shape
//! ported so far. The three agree on the tokens (`track_height` 4.0,
//! `border_radius` 2.0, one shared table) and on both colours
//! (`surface_variant` track, `primary` fill), and disagree about almost everything
//! else. So this is *not* one component with three skins, and the file is mostly a
//! list of the ways they differ.
//!
//! ## The disagreements, as parameters rather than as prose
//!
//! | | Material | Cupertino | Fluent |
//! |---|---|---|---|
//! | fill placement | absolute overlay | absolute overlay, insets spelled out | **flow child, centred** |
//! | fill corner radius | **none** | track radius | track radius |
//! | value → percent | exact | **truncated to whole percent** | exact |
//! | indeterminate | **sliding 40% segment** | static 40% bar | static 40% segment |
//!
//! Each row is a field on [`LinearProgressStyle`] or an argument, and each is
//! pinned by a test in the kit that wants it. Nothing here is "resolved".
//!
//! ## Two places the kits *look* like they disagree and do not
//!
//! * **Fill height.** Material writes `height_pct(100.0)` on an absolutely
//!   positioned fill; Cupertino writes `height(track_height)`. The track's height
//!   *is* `track_height` in both, so the two spellings cannot produce different
//!   geometry. One parameter, and `the_fill_is_as_tall_as_the_track` is the test
//!   that says so — a claim about layout, made falsifiable rather than asserted.
//! * **Track width.** Material writes `width_full()`, the other two
//!   `width_pct(100.0)`. Same box.
//!
//! ## The sliding segment has no driver, and that is recorded rather than faked
//!
//! v1's Material arm spawned an OS thread that slept 16 ms and wrote an
//! `ArcRwSignal` from it. That is not portable to 2.0 and should not be: a widget
//! that mutates shared state off-thread is exactly the shape 2.0's
//! `Arc<Mutex<..>>` state exists to avoid.
//!
//! So [`Indeterminate::SlidingSegment`] declares the geometry — a segment of
//! `width_pct` travelling across `travel_pct` — and the caller supplies the offset
//! through [`LinearProgress::set_fill_offset_pct`]. Nothing inside this crate
//! advances it. `floom-app` does have a clock (`WidgetHost::tick_animations`), so
//! the driver is a small piece of work that belongs at the host, not here; until it
//! exists, Material's indeterminate bar sits still at the left edge, which is
//! Cupertino's and Fluent's behaviour and is visibly wrong for Material.
//!
//! ## One token reads one way here and another in v1
//!
//! `ProgressLinearTokens::border_radius` is documented "Corner radius of the track
//! **and fill**". Material applies it to the track only. The token's doc is the
//! outlier, and it is left alone — fixing it would change a published token's
//! meaning, and the disagreement that matters is Material's behaviour, which is
//! preserved.

use floom_accessibility::SemanticsRole;
use floom_core::{
    CreateContext, ElementId, LayoutRequest, PaintContext, Rect, SemanticsContext, UpdateContext,
    Widget,
};
use floom_types::Color;

use crate::primitives::{Revision, SemanticsHint, emit_semantics};

/// The width of an indeterminate segment, in percent.
///
/// **40.0 in all three kits**, including Material's *sliding* one — the sliding
/// segment is the same size as the static ones, only its offset moves. Named
/// because it is a constant three kits agree on rather than a token any of them
/// reads, and a kit that wanted a different width would have to say so.
pub const INDETERMINATE_WIDTH_PCT: f64 = 40.0;

/// How far Material's sliding segment travels, in percent.
///
/// Material wrote `inset_left_pct((off.get() * 60.0) as f64)`, so the offset runs
/// 0.0 → 1.0 and maps to 0 → 60 percent. Combined with the 40 percent segment that
/// puts the far end flush with the track's right edge, which is what makes the
/// travel 60 and not 100.
pub const SLIDING_TRAVEL_PCT: f64 = 60.0;

/// How the fill sits inside the track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillPlacement {
    /// A centred flow child: the track centres it on both axes and the fill is as
    /// tall as the track. **Fluent.**
    ///
    /// Expressed as geometry rather than as a layout mode, because the observable
    /// difference is only where the fill lands and how tall it is.
    Centred,
    /// An overlay pinned to the track's top-left corner, as wide as its percentage
    /// of the track and as tall as the track. **Material and Cupertino.**
    Absolute,
}

/// How a `0..=100` value becomes a percentage of the track.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercentRounding {
    /// The value as given. Material and Fluent.
    Exact,
    /// Truncated to a whole percent. **Cupertino**, and the only disagreement here
    /// that shows up as a *different number* rather than a different box: at
    /// `value = 33.7` the track fills to 33.7% under Material and to 33.0% under
    /// Cupertino.
    Truncate,
}

/// What to draw when the progress is not known.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Indeterminate {
    /// Fill by the value. The default, and what `indeterminate(false)` means.
    None,
    /// A fixed-width segment at the start of the track. **Cupertino and Fluent.**
    StaticSegment,
    /// A segment of [`INDETERMINATE_WIDTH_PCT`] travelling across
    /// [`SLIDING_TRAVEL_PCT`]. **Material.**
    ///
    /// The geometry is here; the clock is not. See the module note.
    SlidingSegment,
}

impl Indeterminate {
    /// The segment's width as a percentage, or `None` for [`Indeterminate::None`].
    pub fn segment_width_pct(self) -> Option<f64> {
        match self {
            Indeterminate::None => None,
            Indeterminate::StaticSegment | Indeterminate::SlidingSegment => {
                Some(INDETERMINATE_WIDTH_PCT)
            }
        }
    }
}

/// The width of the fill, as a percentage of the track.
///
/// Clamps to `0..=100` first, then applies `rounding`. Indeterminate ignores both
/// the value and the rounding: all three kits draw 40% whatever else is true, and
/// Cupertino's `if indeterminate { 40.0 } else { .. }` puts that branch *before*
/// the rounding, so the two cannot interact.
pub fn fill_percent(value: f32, indeterminate: Indeterminate, rounding: PercentRounding) -> f64 {
    if let Some(width) = indeterminate.segment_width_pct() {
        return width;
    }
    let clamped = value.clamp(0.0, 100.0);
    match rounding {
        // `clamped.trunc()` rather than v1's `(value / 100.0 * 100.0) as f64`:
        // the value is already clamped non-negative, so `as` on an f32 and `trunc`
        // agree exactly, and the round trip reads as an accident.
        PercentRounding::Exact => clamped as f64,
        PercentRounding::Truncate => clamped.trunc() as f64,
    }
}

/// The resolved appearance. Every disagreement between the kits is a field here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearProgressStyle {
    /// Fill of the track behind the bar.
    pub track_color: Color,
    /// Fill of the bar itself.
    pub fill_color: Color,
    /// Height of the track, which is also the height of the fill.
    pub track_height: f32,
    /// Corner radius of the track. All three kits apply this.
    pub track_radius: f32,
    /// Corner radius of the fill, or `None` for square ends.
    ///
    /// `None` for **Material**, which rounds the track and leaves the bar square —
    /// a disagreement with the token's own doc comment, preserved.
    pub fill_radius: Option<f32>,
    /// Whether the fill is an overlay or a centred child.
    pub fill_placement: FillPlacement,
    /// Whether a fractional percentage keeps its fraction.
    pub percent_rounding: PercentRounding,
}

impl Default for LinearProgressStyle {
    fn default() -> Self {
        // Colours are neutral grey: the three kits read `surface_variant` and
        // `primary`, and a default that matched one of them would make an unstyled
        // progress bar silently that kit's.
        Self {
            track_color: Color::rgb(230, 230, 230),
            fill_color: Color::rgb(103, 80, 216),
            // `ProgressLinearTokens::default()` — the one table all three read.
            track_height: 4.0,
            track_radius: 2.0,
            // `Some`: two kits round the fill and one does not, so the majority is
            // the default and Material has to say otherwise. A default of `None`
            // would have been the choice that hid the disagreement.
            fill_radius: Some(2.0),
            fill_placement: FillPlacement::Absolute,
            percent_rounding: PercentRounding::Exact,
        }
    }
}

/// A progress track with a bar in it.
pub struct LinearProgress {
    value: f32,
    indeterminate: Indeterminate,
    style: LinearProgressStyle,
    /// Where a sliding segment currently is, in percent of the track width.
    ///
    /// Public and caller-driven: see the module note on why nothing here advances
    /// it. `0.0` is the left edge, `SLIDING_TRAVEL_PCT` the right edge of travel.
    fill_offset_pct: f64,
    revision: Revision,
}

impl std::fmt::Debug for LinearProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinearProgress")
            .field("value", &self.value)
            .field("indeterminate", &self.indeterminate)
            .field("fill_percent", &self.fill_percent())
            .finish_non_exhaustive()
    }
}

impl LinearProgress {
    /// A bar showing `value`, a percentage in `0..=100`.
    ///
    /// Not clamped here: clamping happens in [`Self::fill_percent`] so that a value
    /// read back is the value that was set. v1 clamped at the top of `view()`,
    /// which meant the props and the drawing could disagree.
    pub fn new(value: f32) -> Self {
        LinearProgress {
            value,
            indeterminate: Indeterminate::None,
            style: LinearProgressStyle::default(),
            fill_offset_pct: 0.0,
            revision: Revision::default(),
        }
    }

    /// Which indeterminate mode this bar is drawing.
    ///
    /// A getter as well as a builder, because the three kits disagree here and a
    /// caller (or a test) has to be able to ask which one it got without reading
    /// the style struct it built.
    pub fn indeterminate(&self) -> Indeterminate {
        self.indeterminate
    }

    /// Draw `segment` instead of a value.
    pub fn indeterminate_mode(mut self, segment: Indeterminate) -> Self {
        self.indeterminate = segment;
        self.revision.bump();
        self
    }

    pub fn style(mut self, style: LinearProgressStyle) -> Self {
        self.style = style;
        self.revision.bump();
        self
    }

    pub fn resolved_style(&self) -> LinearProgressStyle {
        self.style
    }

    /// The value as set, unclamped.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// The width of the fill, as a percentage of the track.
    pub fn fill_percent(&self) -> f64 {
        fill_percent(self.value, self.indeterminate, self.style.percent_rounding)
    }

    /// Where a sliding segment sits, in percent of the track width.
    pub fn fill_offset_pct(&self) -> f64 {
        self.fill_offset_pct
    }

    /// Moves a sliding segment. Ignored by the other two indeterminate modes,
    /// which have nowhere to put an offset.
    pub fn set_fill_offset_pct(&mut self, offset_pct: f64) {
        if self.fill_offset_pct != offset_pct {
            self.fill_offset_pct = offset_pct;
            self.revision.bump();
        }
    }

    /// The rectangle the fill occupies inside `track`.
    ///
    /// The whole layout question in one function, which is why it is a free
    /// function: both placements, both roundings and the sliding offset are
    /// testable without a host, and a widget whose geometry can only be checked by
    /// mounting is a widget whose geometry does not get checked.
    pub fn fill_rect(&self, track: Rect) -> Rect {
        let percent = self.fill_percent();
        let width = track.width() * percent / 100.0;
        // The fill is as tall as the track in every kit. Material spells this
        // `height_pct(100.0)` and Cupertino `height(track_height)`; the track's
        // height is `track_height` in both, so the spellings cannot differ.
        let height = track.height();
        let (x0, y0) = match self.style.fill_placement {
            FillPlacement::Absolute => {
                // Pinned top-left. Cupertino writes `inset_left(0).inset_top(0)`
                // and Material writes neither; an absolute child with no inset is
                // at the parent's origin, so the two spellings agree.
                //
                // The sliding offset applies **only** to `SlidingSegment`. The
                // first version applied it unconditionally, and
                // `the_offset_does_nothing_for_a_determinate_bar` caught it: a
                // determinate bar slid sideways when a caller set an offset that
                // was never meant for it. The mode has to be the switch.
                let offset = match self.indeterminate {
                    Indeterminate::SlidingSegment => self.fill_offset_pct,
                    _ => 0.0,
                };
                (track.x0 + track.width() * offset / 100.0, track.y0)
            }
            // Fluent's flow child, which the track centres on both axes.
            FillPlacement::Centred => (
                track.x0 + (track.width() - width) / 2.0,
                track.y0 + (track.height() - height) / 2.0,
            ),
        };
        Rect {
            x0,
            y0,
            x1: x0 + width,
            y1: y0 + height,
        }
    }

    /// The fill's corner radius: the style's, or zero for square ends.
    pub fn effective_fill_radius(&self) -> f32 {
        self.style.fill_radius.unwrap_or(0.0)
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        // A progress bar, announced as a percentage. `Indeterminate` announces no
        // value at all, because "40%" would be a lie about how far along it is.
        if self.indeterminate == Indeterminate::None {
            SemanticsHint::labeled(
                SemanticsRole::ProgressBar,
                format!("{}%", self.fill_percent() as i64),
            )
        } else {
            SemanticsHint::labeled(
                SemanticsRole::ProgressBar,
                "Indeterminate",
            )
        }
    }
}

impl Widget for LinearProgress {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // The height is the one dimension this widget knows before it is laid out.
        // The width is not: a track is `width_full()` under Material and
        // `width_pct(100.0)` under the other two, which is the parent's decision
        // either way.
        Some(LayoutRequest::new().height(self.style.track_height))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let track = cx.bounds();
        if track.width() <= 0.0 || track.height() <= 0.0 {
            return;
        }
        cx.fill_rounded_rect(track, self.style.track_radius, self.style.track_color);

        let fill = self.fill_rect(track);
        if fill.width() <= 0.0 || fill.height() <= 0.0 {
            return;
        }
        let radius = self.effective_fill_radius();
        if radius > 0.0 {
            cx.fill_rounded_rect(fill, radius, self.style.fill_color);
        } else {
            // Material: the track is rounded and the bar inside it is not, so a
            // zero radius has to mean "square", not "the same as the track".
            cx.fill_rect(fill, self.style.fill_color);
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
