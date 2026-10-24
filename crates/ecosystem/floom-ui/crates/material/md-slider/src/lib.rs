//! Material 3 slider, ported to the 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1's `list(vec![filled, halo, thumb])` -- three `Container`s, each with a
//!   closure style re-deriving the value's fraction on its own -- is the
//!   engine's `Slider` widget now: one `Widget` with `layout_request` and
//!   `paint`, computing the fraction once per paint rather than once per box.
//! * v1's `update(ratio)` closure (clamp, quantise, `live.set`, notify) is the
//!   engine's `quantize`/`set_value`, driven by pointer events instead of by
//!   hand-written `PointerDown`/`PointerMove`/`PointerUp` listeners plus
//!   `request_pointer_capture`.
//! * v1's three `RwSignal`s -- `live`, `dragging`, `hover` -- are gone: the
//!   widget owns the value and an `InteractionState`, and reports through
//!   `on_change`.
//! * `value_signal` is gone from the API as well as the implementation, and no
//!   replacement was invented for it. v1 needed it because a host that rebuilt
//!   the view on every `value` change made a drag flash back to the old value;
//!   the engine's widget holds its value across paints, so that failure cannot
//!   happen. Re-publishing the engine's value as a signal would put the rebuild
//!   back, which is the very thing v1 was avoiding.
//!
//! ## What is left here
//!
//! Token translation, and the two things v1 drew that the other two kits do not
//! have: the halo behind the thumb, and a token-derived disabled palette.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::slider::SliderProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::{Slider, SliderStyle};
use floom_widget_runtime::theme_signal;

/// v1 drew the halo 40px across in both states, so the two sizes are one number.
const HALO_SIZE: f32 = 40.0;
/// The hovered halo's alpha, in percent. [`MdSlider::resolved_style`] says
/// where v1's dragging alpha went.
const HALO_ALPHA: u8 = 20;

/// Material's slider.
pub struct MdSlider {
    props: SliderProps,
    theme: RwSignal<MultiTheme>,
}

impl MdSlider {
    pub fn new() -> Self {
        Self {
            props: SliderProps::default(),
            theme: theme_signal(),
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Material's appearance, as the engine's `SliderStyle`.
    ///
    /// Three of v1's numbers are this kit's own: the halo, the disabled
    /// palette, and a track at 97 percent of `on_surface_variant` where the
    /// other two kits use an undimmed `surface_variant`.
    pub fn resolved_style(&self) -> SliderStyle {
        let theme = self.theme.get();
        let ComponentTokens::Slider(tokens) = theme.component(ComponentId::Slider) else {
            unreachable!("ComponentId::Slider must resolve to SliderTokens")
        };
        let colors = theme.colors();
        let primary = colors.primary;

        // `SliderStyle` carries no disabled colours, and that is deliberate --
        // do not add one. The engine paints a disabled slider with whatever
        // colours the style was given, so a kit is the only thing that knows
        // `props.disabled` at the moment it builds the widget. Composing the
        // disabled palette here is that knowledge being used once, where it
        // belongs; v1 composed it the same way, in the three style closures.
        let (track, filled, thumb) = if self.props.disabled {
            (
                to_paint(with_alpha(colors.on_surface_variant, 60)),
                to_paint(with_alpha(primary, 120)),
                to_paint(with_alpha(primary, 120)),
            )
        } else {
            (
                to_paint(with_alpha(colors.on_surface_variant, 97)),
                to_paint(primary),
                to_paint(primary),
            )
        };

        SliderStyle {
            // --- from `SliderTokens`, as v1 read them ---
            track_height: tokens.track_height,
            thumb_size: tokens.thumb_size,
            thumb_radius: tokens.thumb_radius,

            track_color: track,
            filled_color: filled,
            thumb_color: thumb,

            // v1 gave none of the three boxes a hover or press colour of its
            // own -- it left every interaction state to the style layer's
            // defaults. `None` is the faithful answer, and a different answer
            // from `Some`: `None` means "no variant of this kit's own", so the
            // base colour is painted, rather than the engine's hard-coded
            // hover and press blues.
            track_hovered_color: None,
            filled_hovered_color: None,
            filled_pressed_color: None,

            // Material is the only one of the three kits whose thumb carries a
            // halo, and the only one whose v1 slider drew one at all: a 40px
            // `primary` circle, at 20 percent while hovered and at 30 percent
            // while dragging.
            //
            // Only the hovered alpha survives. `SliderStyle` has one
            // `halo_color` for both states -- it varies the halo's *size* by
            // state, not its colour -- so v1's 30 percent has nowhere to go.
            // The engine's own doc comment on the field names 20 percent as
            // this kit's number, which is why that is the one carried.
            halo_color: Some(to_paint(with_alpha(primary, HALO_ALPHA))),
            halo_size: HALO_SIZE,
            halo_size_dragging: HALO_SIZE,

            // v1's Material slider drew no shadow on its thumb.
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

impl Default for MdSlider {
    fn default() -> Self {
        Self::new()
    }
}

/// The engine's `SliderStyle`, under this kit's old name.
///
/// v1's own `MdSliderStyle` carried `design_system`, `track_color`,
/// `thumb_color` and three geometry numbers, and nothing ever read it -- v1's
/// `view()` resolved the colours from the theme directly. The alias keeps the
/// name resolvable for callers without pretending a struct still exists. The
/// `design_system` field has no successor: the engine's style names no design
/// system, because it is not one.
pub type MdSliderStyle = SliderStyle;
