//! Cupertino switch, ported to the Floom 2.0 widget API.
//!

use std::rc::Rc;

use floom_core::Axis as _AxisUnused;
use floom_signal::{RwSignal, SignalGet, SignalUpdate};
use floom_types::Color as PaintColor;
use floom_widgets::{Switch, SwitchPalette};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, SwitchTokens};
use floom_core_theme::props::switch::SwitchProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "cupertino";

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

/// Cupertino's resolved appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoSwitchStyle {
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

pub struct CupertinoSwitch {
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

impl CupertinoSwitch {
    pub fn new() -> Self {
        Self {
            props: SwitchProps::default(),
            checked: RwSignal::new(false),
            // A **snapshot**, unlike Material's live `theme_signal()`. Taken
            // from the process-global design system at construction, so a later
            // global theme switch does not restyle this switch. v1 did the same,
            // and two of the three kits behaved this way, so it is preserved
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoSwitchStyle {
        let theme = self.theme.get();
        let tokens = switch_tokens(&theme);
        let checked = self.is_checked();
        CupertinoSwitchStyle {
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

impl Default for CupertinoSwitch {
    fn default() -> Self {
        Self::new()
    }
}
