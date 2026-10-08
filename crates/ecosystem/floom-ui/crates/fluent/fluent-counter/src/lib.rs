//! Fluent counter, ported to the Floom 2.0 widget API.
//!
//! ## The shortest possible port, and why that is a finding
//!
//! `fluent-counter`, `cupertino-counter` and `fluent-counter` are 151, 148 and 148
//! lines, and after normalising the kit names away the three differ in **five
//! lines — all of them the theme signal**. Every design value comes from
//! `CounterTokens`, and the single literal (`font_size(16.0)` on the value readout)
//! is the same literal in all three.
//!
//! So this is not "one component with three skins", and that is worth stating
//! plainly: there is **no disagreement to preserve here**. Most shapes in this port
//! turned out to disagree somewhere, and the disagreements are the substance. This
//! one does not, and the honest report is "the kits agree" rather than a table of
//! two rows.
//!
//! Everything that is left here is the translation from design tokens to
//! [`CounterStyle`], plus the one value that is *not* a token — the readout's
//! `16.0`.
//!
//! ## What changed from v1
//!
//! * **The state became shared.** v1 held a `RwSignal<i64>` created inside
//!   `view()`, so a caller could set the value before mounting and read it
//!   afterwards only by holding the signal — and `view()` returned `impl View`, so
//!   it could not hand it back. 2.0 needs `Arc<Mutex<CounterState>>` for the reason
//!   every other 2.0 shared value does: a slot is `Send + Sync` and `RwSignal` is
//!   `Rc`-based. [`FluentCounter::view`] therefore hands the widget back, and
//!   [`floom_widgets::Counter::state`] is how a caller reads it.
//! * **`CursorStyle::Pointer` is gone.** 2.0's `PaintContext` has no cursor
//!   vocabulary. The buttons are still `Pressable`s, so they are still reachable by
//!   keyboard, which the cursor was not. Recorded rather than worked around.
//! * **`clamp_value` moved into the engine**, unchanged, including which bound wins
//!   when the range is inverted. See `floom_widgets::counter`.
//! * **`on_change` still fires when a press cannot move the value.** v1 called back
//!   unconditionally, and that is preserved — including the fact that the engine's
//!   own `Slider` does the opposite.

use floom_legacy_reactive::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Counter, CounterStyle};

use floom_widget_runtime::theme_signal;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CounterTokens};
use floom_core_theme::props::counter::CounterProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The design system this crate reads.
const DESIGN_SYSTEM: &str = "fluent";

/// The readout's font size, in px.
///
/// **Not a token.** v1 wrote `s.font_size(16.0)` inline in all three kits, and all
/// three wrote the same 16.0 — so it is a literal here too, named rather than
/// inlined so the three crates can be compared against each other by reading this
/// line. The kits' agreement on it is measured, not assumed; see
/// `tests/counter.rs::the_three_kits_agree_on_the_readout_font_size`.
pub const READOUT_FONT_SIZE: f32 = 16.0;

/// Design-system colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

fn counter_tokens(theme: &MultiTheme) -> CounterTokens {
    match theme.component(ComponentId::Counter) {
        ComponentTokens::Counter(t) => t,
        _ => unreachable!("ComponentId::Counter must resolve to CounterTokens"),
    }
}

/// Material's resolved counter appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentCounterStyle {
    pub design_system: &'static str,
    /// Fill of the two step buttons. v1 read `surface_variant`.
    pub button_background: Color,
    /// Ink of the two glyphs. v1 read `on_surface`.
    pub button_foreground: Color,
    /// Ink of the readout. v1 read `on_surface` — the *same* token as the glyphs,
    /// which is why the engine carries one `value_font_size` for both and a kit
    /// that wanted them to differ would need a second field.
    pub value_color: Color,
    /// Outline of the band. v1 read `outline`.
    pub border_color: Color,
    pub button_size: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub gap: f32,
    pub border: f32,
}

pub struct FluentCounter {
    props: CounterProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentCounter {
    pub fn new() -> Self {
        Self {
            props: CounterProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
        }
    }

    pub fn value(mut self, v: i64) -> Self {
        self.props.value = v;
        self
    }

    pub fn min(mut self, m: i64) -> Self {
        self.props.min = Some(m);
        self
    }

    pub fn max(mut self, m: i64) -> Self {
        self.props.max = Some(m);
        self
    }

    pub fn step(mut self, s: i64) -> Self {
        self.props.step = s;
        self
    }

    /// The change callback, held until [`Self::view`].
    ///
    /// `Send + Sync` where v1's was not, because the engine's callback crosses a
    /// slot boundary. A caller that captured an `Rc` cannot; that is the port's
    /// one hard requirement on callers, and it is the third time this port has had
    /// to state it.
    pub fn on_change(mut self, callback: impl Fn(i64) + Send + Sync + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &CounterProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentCounterStyle {
        let theme = self.theme.get();
        let tokens = counter_tokens(&theme);
        FluentCounterStyle {
            design_system: DESIGN_SYSTEM,
            button_background: theme.colors().surface_variant,
            button_foreground: theme.colors().on_surface,
            value_color: theme.colors().on_surface,
            border_color: theme.colors().outline,
            button_size: tokens.button_size,
            width: tokens.width,
            height: tokens.height,
            radius: tokens.radius,
            gap: tokens.gap,
            border: tokens.border,
        }
    }

    /// The engine's style, which is the whole translation.
    pub fn counter_style(&self) -> CounterStyle {
        let style = self.resolved_style();
        CounterStyle {
            button_size: style.button_size,
            width: style.width,
            height: style.height,
            radius: style.radius,
            gap: style.gap,
            border: style.border,
            border_color: to_paint(style.border_color),
            button_background: to_paint(style.button_background),
            button_foreground: to_paint(style.button_foreground),
            value_font_size: READOUT_FONT_SIZE,
            value_color: to_paint(style.value_color),
        }
    }

    /// The counter as an engine widget.
    ///
    /// Returns the widget rather than `impl View`, so a caller can read the value
    /// off it afterwards — v1's `view()` returned an opaque view and left the value
    /// unreachable.
    pub fn view(self) -> Counter {
        let mut counter = Counter::new(self.props.value).style(self.counter_style());
        if let Some(min) = self.props.min {
            counter = counter.min(min);
        }
        if let Some(max) = self.props.max {
            counter = counter.max(max);
        }
        counter = counter.step_size(self.props.step);
        if let Some(on_change) = self.props.on_change {
            counter = counter.on_change(move |v| on_change(v));
        }
        counter
    }
}

impl Default for FluentCounter {
    fn default() -> Self {
        Self::new()
    }
}
