//! Material 3 linear progress, ported to the Floom 2.0 widget API.
//!
//! ## The lowest-similarity shape in the port, and why that is the point
//!
//! `md_progress_linear` / `cupertino_progress_linear` / `fluent_progress_linear`
//! measure **0.17 and 0.19** similarity — the lowest of any shape ported so far.
//! They agree on the tokens (one shared table, `track_height` 4.0,
//! `border_radius` 2.0) and on both colours (`surface_variant` track, `primary`
//! fill). They disagree about everything else, and the disagreements are the
//! substance:
//!
//! | | Material | Cupertino | Fluent |
//! |---|---|---|---|
//! | fill placement | absolute overlay | absolute overlay | centred child |
//! | fill corner radius | **none** | track radius | track radius |
//! | value → percent | exact | **truncated** | exact |
//! | indeterminate | **sliding 40% segment** | static 40% bar | static 40% segment |
//!
//! All four are parameters on [`LinearProgressStyle`] or
//! [`floom_widgets::Indeterminate`], and each kit says which it wants. Nothing is
//! resolved.
//!
//! ## What this kit uniquely wants
//!
//! * **`fill_radius: None`.** Material rounds the track and leaves the bar square.
//!   `ProgressLinearTokens::border_radius` is documented "Corner radius of the
//!   track **and fill**", so Material disagrees with its own token's doc. The doc
//!   is left alone — changing a published token's meaning is a design decision,
//!   and the disagreement that matters is Material's behaviour.
//! * **`Indeterminate::SlidingSegment`.** v1 spawned an OS thread that slept
//!   16 ms and wrote an `ArcRwSignal` from it, sliding a 40% segment across 60% of
//!   the track. See "What is lost" below.
//!
//! ## What is lost, and stated rather than faked
//!
//! **The sliding animation has no driver.** v1's was a spawned thread mutating a
//! signal off-thread, which is exactly the shape 2.0's shared `Arc<Mutex<..>>`
//! state exists to avoid, so it is not portable and should not be. The *geometry*
//! survives — a 40% segment travelling across `SLIDING_TRAVEL_PCT` — and the offset
//! is the caller's to supply through `LinearProgress::set_fill_offset_pct`.
//! Nothing in the engine advances it. Until a host-side driver exists, an
//! indeterminate Material bar sits at the left edge, which is Cupertino's and
//! Fluent's behaviour and is visibly wrong for Material.
//!
//! ## Two API differences kept, one added
//!
//! * **Kept: `height`, not `track_height`.** Material's style struct names the
//!   field `height` where the other two say `track_height`. That is an API
//!   difference rather than a visual one, and it costs nothing to keep.
//! * **Added: `props()`, `theme()`, `with_theme()`.** v1's Material arm had none of
//!   these; Cupertino's and Fluent's did. Deriving all three from one file meant
//!   either losing two accessors two kits had, or giving Material two it did not.
//!   Adding them is the smaller change, and it is an addition rather than a
//!   behaviour change.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{FillPlacement, Indeterminate, LinearProgress, LinearProgressStyle, PercentRounding};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ProgressLinearTokens};
use floom_core_theme::props::progress_linear::ProgressLinearProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "material";

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn progress_linear_tokens(theme: &MultiTheme) -> ProgressLinearTokens {
    match theme.component(ComponentId::ProgressLinear) {
        ComponentTokens::ProgressLinear(t) => t,
        _ => unreachable!("ComponentId::ProgressLinear must resolve to ProgressLinearTokens"),
    }
}

/// Material's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdProgressLinearStyle {
    pub design_system: &'static str,
    /// Fill of the track behind the bar.
    pub track_color: Color,
    /// Fill of the bar.
    pub fill_color: Color,
    /// Height of the track. **Named `height`,** where Cupertino and Fluent say
    /// `track_height` — an API difference kept rather than tidied away.
    pub height: f32,
    pub border_radius: f32,
    /// Material rounds the track and not the bar, so there is no fill radius.
    /// Not a field: its absence *is* the disagreement.
    pub track_only_radius: bool,
}

pub struct MdProgressLinear {
    props: ProgressLinearProps,
    theme: RwSignal<MultiTheme>,
}

impl MdProgressLinear {
    pub fn new() -> Self {
        Self {
            props: ProgressLinearProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    /// Show a sliding segment instead of a value.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.props.indeterminate = indeterminate;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.props.height = height;
        self
    }

    pub fn props(&self) -> &ProgressLinearProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdProgressLinearStyle {
        let theme = self.theme.get();
        let tokens = progress_linear_tokens(&theme);
        let height = if self.props.height > 0.0 {
            self.props.height
        } else {
            tokens.track_height
        };
        MdProgressLinearStyle {
            design_system: DESIGN_SYSTEM,
            track_color: theme.colors().surface_variant,
            fill_color: theme.colors().primary,
            height,
            border_radius: tokens.border_radius,
            track_only_radius: true,
        }
    }

    /// The engine's style, which is the whole translation.
    ///
    /// Four disagreements in eight lines, and each one is a value rather than a
    /// branch: the engine asks four questions and this answers them.
    pub fn linear_progress_style(&self) -> LinearProgressStyle {
        let style = self.resolved_style();
        LinearProgressStyle {
            track_color: to_paint(style.track_color),
            fill_color: to_paint(style.fill_color),
            track_height: style.height,
            track_radius: style.border_radius,
            // The disagreement: Cupertino and Fluent round the bar, Material does
            // not. `None` means square, not "same as the track" — the engine test
            // `a_none_fill_radius_means_square_ends_not_the_track_radius` is the
            // one that keeps those apart.
            fill_radius: None,
            fill_placement: FillPlacement::Absolute,
            percent_rounding: PercentRounding::Exact,
        }
    }

    /// Which indeterminate mode this kit asks for.
    ///
    /// v1's `indeterminate` was a `bool` on the props and stays one; the engine
    /// wants three states, and Material is the kit that needs the third.
    pub fn indeterminate_mode(&self) -> Indeterminate {
        if self.props.indeterminate {
            Indeterminate::SlidingSegment
        } else {
            Indeterminate::None
        }
    }

    /// The bar as an engine widget.
    ///
    /// The returned widget's `set_fill_offset_pct` is what a host-side animation
    /// driver would call. Nothing here calls it, and that is the recorded loss
    /// above rather than an oversight — see the module note.
    pub fn view(self) -> LinearProgress {
        LinearProgress::new(self.props.value)
            .indeterminate_mode(self.indeterminate_mode())
            .style(self.linear_progress_style())
    }
}

impl Default for MdProgressLinear {
    fn default() -> Self {
        Self::new()
    }
}
