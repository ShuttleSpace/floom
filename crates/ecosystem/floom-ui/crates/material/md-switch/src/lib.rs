//! Material 3 switch, ported to the Floom 2.0 widget API.
//!
//! ## The three kits do not disagree on a switch
//!
//! Measured twice, because the first measurement was wrong. With naming and the
//! `design_system` field removed, the three v1 `resolved_style()` bodies are
//! byte-identical, and the three `view()` bodies differ only in the kit name,
//! the design system enum, and a doc comment that calls Cupertino's a "pill".
//! Their tokens are identical too — one `SwitchTokens` serves all three design
//! systems through `ComponentId::Switch`.
//!
//! (Two crates carried an older `tests/api.rs` claiming 51 x 31 and 40 x 20.
//! Those assertions are stale and contradict the theme layer; see
//! `the_track_is_the_shared_switch_token`.)
//!
//! So there is no per-kit style parameter here, and inventing one would be
//! inventing a disagreement.
//!
//! What the kits *do* disagree with the engine is four things, and each one is
//! a reason the engine grew a parameter:
//!
//! 1. **Three independent numbers.** `SwitchTokens` gives width 52, height 32
//!    and thumb 24. None of them satisfies the engine's derivation (half the
//!    width would be 26, and a knob as tall as the track would be 32), so the
//!    engine takes `track_size` and `knob_size` rather than scaling.
//! 2. **A 2px gap.** v1 wrote `padding(2.0)` on the track with the knob centred
//!    inside it, so the knob never touched either end. The engine's `knob_inset`
//!    is that padding, promoted from decoration to geometry.
//! 3. **Colours come from the theme.** The engine hard-codes a green track and
//!    a white knob; a kit has to be able to supply its own.
//! 4. **No hover or press feedback.** v1 styled only the resting and disabled
//!    states. The engine's `tints(false)` turns its feedback off, rather than a
//!    kit passing four near-identical colours to get the same result.
//!
//! ## `thumb_offset` is dead, and that is worth knowing
//!
//! v1's style struct carried a `thumb_offset` of `width - height` = 20, which
//! looks like it would drop the knob off-centre. It does not: `view()` never
//! read the field, and positioned the knob with `justify_start`/`justify_end`
//! inside the 2px padding instead. This crate therefore does not reproduce it.
//! `a_knob_inside_the_track_with_2px_gaps` is the assertion for where the knob
//! actually goes.

use std::rc::Rc;

use floom_signal::{RwSignal, SignalGet, SignalUpdate};
use floom_types::Color as PaintColor;
use floom_widgets::{Switch, SwitchPalette};
use floom_core::Axis as _AxisUnused;

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, SwitchTokens};
use floom_core_theme::props::switch::SwitchProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "material";

/// v1's disabled alphas, as parts of 255.
///
/// These are not a "half alpha" rule the engine could have guessed: the track
/// is `on_surface` at 12/255 and the thumb is `outline` at 38/255, two different
/// roles at two different strengths. Only the knob's 38 is near the engine's
/// default of half.
const DISABLED_TRACK_ALPHA: u8 = 12;
const DISABLED_THUMB_ALPHA: u8 = 38;

fn with_alpha(c: Color, alpha: u8) -> Color {
    Color { a: alpha, ..c }
}

fn switch_tokens(theme: &MultiTheme) -> SwitchTokens {
    match theme.component(ComponentId::Switch) {
        ComponentTokens::Switch(t) => t,
        _ => unreachable!("ComponentId::Switch must resolve to SwitchTokens"),
    }
}

/// Material's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSwitchStyle {
    pub design_system: &'static str,
    pub track_color: Color,
    pub thumb_color: Color,
    pub track_disabled: Color,
    pub thumb_disabled: Color,
    pub width: f32,
    pub height: f32,
    pub thumb_size: f32,
    /// The gap v1 got from `padding(2.0)`.
    pub inset: f32,
}

pub struct MdSwitch {
    props: SwitchProps,
    checked: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
    /// A shared handle on `props.on_change`.
    ///
    /// `SwitchProps::on_change` is a `Box<dyn Fn(bool)>` and is not `Clone` --
    /// its `Clone` impl deliberately drops the callback -- so it cannot be
    /// borrowed into the engine's `'static` callback closure. An `Rc` around the
    /// whole `Option` is the same move `md-checkbox` makes, and matches v1,
    /// which handed the props to the view closure.
    on_change: Rc<Option<Box<dyn Fn(bool)>>>,
}

impl MdSwitch {
    pub fn new() -> Self {
        Self {
            props: SwitchProps::default(),
            checked: RwSignal::new(false),
            // The **live** signal, unlike Cupertino and Fluent.
            theme: theme_signal(),
            on_change: Rc::new(None),
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked.set(checked);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, cb: impl Fn(bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(cb));
        self.on_change = Rc::new(self.props.on_change.take());
        self
    }

    pub fn props(&self) -> &SwitchProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn resolved_style(&self) -> MdSwitchStyle {
        let theme = self.theme.get();
        let tokens = switch_tokens(&theme);
        let checked = self.is_checked();
        MdSwitchStyle {
            design_system: DESIGN_SYSTEM,
            track_color: if checked {
                theme.colors().primary
            } else {
                theme.colors().surface_variant
            },
            thumb_color: if checked {
                theme.colors().on_primary
            } else {
                theme.colors().outline
            },
            track_disabled: with_alpha(theme.colors().on_surface, DISABLED_TRACK_ALPHA),
            thumb_disabled: with_alpha(theme.colors().outline, DISABLED_THUMB_ALPHA),
            width: tokens.width,
            height: tokens.height,
            thumb_size: tokens.thumb_size,
            inset: 2.0,
        }
    }

    /// The engine's switch, which is the whole translation.
    pub fn switch(&self) -> Switch {
        // `SwitchProps::on_change` is a `Box<dyn Fn(bool)>` and is not `Clone`
        // (its `Clone` impl deliberately drops it), so it cannot be borrowed
        // into the engine's `'static` callback. Moving the whole `Option` into
        // an `Rc` and cloning that `Rc` is what `md-checkbox` already does, and
        // matches v1, which passed the props themselves into the view closure.
        let checked = self.checked;
        let on_change = self.on_change.clone();
        let style = self.resolved_style();
        Switch::new("")
            .on(self.is_checked())
            .disabled(self.props.disabled)
            .track_size(style.width as f64, style.height as f64)
            .knob_size(style.thumb_size as f64)
            .knob_inset(style.inset as f64)
            // v1 had no hover or press styling, so the engine's feedback is
            // turned off rather than approximated.
            .tints(false)
            .palette(SwitchPalette {
                track_on: to_paint(theme_color(self.theme.get(), true, true)),
                track_off: to_paint(theme_color(self.theme.get(), false, true)),
                thumb_on: to_paint(theme_color(self.theme.get(), true, false)),
                track_knob_off: to_paint(theme_color(self.theme.get(), false, false)),
                track_disabled: Some(to_paint(style.track_disabled)),
                thumb_disabled: Some(to_paint(style.thumb_disabled)),
            })
            .on_toggle(move |next| {
                checked.set(next);
                if let Some(cb) = on_change.as_ref() {
                    cb(next);
                }
            })
    }
}

/// The engine palette needs both resting colours at once, so this reads the
/// theme twice where `resolved_style` reads it once per call.
fn theme_color(theme: MultiTheme, checked: bool, track: bool) -> Color {
    match (checked, track) {
        (true, true) => theme.colors().primary,
        (true, false) => theme.colors().on_primary,
        (false, true) => theme.colors().surface_variant,
        (false, false) => theme.colors().outline,
    }
}

fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}

impl Default for MdSwitch {
    fn default() -> Self {
        Self::new()
    }
}
