//! Fluent's slider, ported to the 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1's `Stack::horizontal((track_line, fill, thumb))`, its
//!   `LayoutChangedListener` that stashed the measured width in an `RwSignal`,
//!   and its `value_from_local_x` helper are the engine's `Slider` widget now.
//!   The helper is worth a note: it clamped the pointer into `[thumb/2,
//!   width - thumb/2]` before mapping, which is exactly the span the engine's
//!   `thumb_rect` leaves once the thumb's own width is taken off each end, so
//!   the two agree about where a thumb at fraction 0 actually sits.
//! * v1's `value` and `value_signal` `RwSignal`s are gone, and so is
//!   `value_signal` from the builder. The engine's widget owns the value across
//!   paints, so the drag flash-back that made v1 reach for a signal cannot
//!   happen, and a signal bridge would only reintroduce the host rebuild it
//!   was avoiding.
//! * v1's `width` and `dragging` `RwSignal`s are gone: the engine measures the
//!   widget it is laying out and tracks its own `InteractionState`.
//! * The cursor is gone with them. v1 chose `CursorStyle::Pointer` while
//!   enabled and `CursorStyle::Default` while disabled, and the engine's
//!   `Slider` has no cursor, so that choice is no longer this kit's to make.
//!
//! ## This kit is the plain one, and that is the finding
//!
//! Fluent is the only one of the three whose v1 thumb had no halo and no drop
//! shadow: a bare `primary` disc on a `surface_variant` track. Both of the
//! engine's optional treatments are therefore `None` here, and so are all three
//! interaction variants, which v1 also left at the style layer's defaults.
//!
//! ## The track's corner radius did not survive, and it is not a token problem
//!
//! v1 drew both the track line and the fill at `theme.radius().sm`, *not* at
//! `track_height / 2.0` as Material and Cupertino did, and the two are
//! different numbers -- so this is a visible difference, not a coincidence that
//! happened to be invisible. The token is readable, so nothing was lost by
//! accident. What cannot carry it is `SliderStyle`, which has no track-radius
//! field at all, and the engine's `paint`, which draws the track and the fill
//! with `fill_rect` rather than `fill_rounded_rect`. So the track is a
//! square-cornered rect in 2.0 and no kit can change that from here.
//! `thumb_radius` is the *thumb's* corner radius and is a different number; it
//! is not being reused to paper over this. Reported rather than silently
//! substituted, as instructed.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_ui_composites::{Slider, SliderStyle};

/// Fluent's slider.
pub struct FluentSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSlider {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props: SliderProps::default(),
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.props.value = value;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.props.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.props.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.props.step = step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(f32) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &SliderProps {
        &self.props
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Fluent's appearance, as the engine's `SliderStyle`.
    ///
    /// The plainest of the three: no halo, no shadow, no interaction variants,
    /// and no disabled branch. The last is a decision rather than an oversight
    /// -- v1's disabled look came from the style layer's generic dim
    /// (`set_disabled(true)`, which halved a colour's alpha) rather than from
    /// this kit's tokens, and the engine deliberately dropped that rule,
    /// because a hard-coded dim a kit cannot override is the problem
    /// `SliderStyle` exists to fix. Material shows what a kit's own disabled
    /// answer looks like; this kit had none, and inventing one would be a
    /// design decision this port has no licence to make.
    pub fn resolved_style(&self) -> SliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        let colors = theme.colors();

        SliderStyle {
            // --- from `SliderTokens`, as v1 read them ---
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,

            // --- base colours ---
            //
            // An undimmed `surface_variant` track, as in Cupertino and unlike
            // Material, and a `primary` thumb, as in Material and unlike
            // Cupertino. So of the three kits Fluent shares its track with one
            // and its thumb with the other.
            track_color: to_paint(colors.surface_variant),
            filled_color: to_paint(colors.primary),
            thumb_color: to_paint(colors.primary),

            // v1 had no hover or press colour of its own for any of the three
            // boxes. `None` means "no variant of my own", so the base colour is
            // painted -- the faithful answer, and a different answer from the
            // engine's default, which carries its own hover and press blues.
            track_hovered_color: None,
            filled_hovered_color: None,
            filled_pressed_color: None,

            // Neither optional thumb treatment. This is the kit's whole answer
            // to "how is the thumb separated from the track": it is not.
            halo_color: None,
            halo_size: 0.0,
            halo_size_dragging: 0.0,

            shadow_blur: 0.0,
            shadow_spread: 0.0,
            shadow_color: PaintColor::TRANSPARENT,
        }
    }

    /// The slider, ready to mount.
    pub fn view(self) -> Slider {
        let style = self.resolved_style();

        // `SliderProps` speaks `f32` because it is the theme crate's shared
        // struct, still shared with the kits that have not been ported, while
        // the engine's slider is `f64`. So the widening happens here, once.
        let mut slider = Slider::new(
            f64::from(self.props.value),
            f64::from(self.props.min),
            f64::from(self.props.max),
        )
        .step(f64::from(self.props.step))
        .disabled(self.props.disabled)
        .style(style);

        // And the narrowing back, on the way out.
        let on_change = self.props.on_change;
        if let Some(on_change) = on_change {
            slider = slider.on_change(move |value| on_change(value as f32));
        }

        slider
    }
}

impl Default for FluentSlider {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine's `SliderStyle`, under this kit's old name.
///
/// v1's own `FluentSliderStyle` carried `design_system`, `track_color`,
/// `thumb_color` and three geometry numbers, and nothing ever read it -- v1's
/// `view()` resolved the colours from the theme directly. The alias keeps the
/// name resolvable for callers without pretending a struct still exists. The
/// `design_system` field has no successor: the engine's style names no design
/// system, because it is not one.
pub type FluentSliderStyle = SliderStyle;
