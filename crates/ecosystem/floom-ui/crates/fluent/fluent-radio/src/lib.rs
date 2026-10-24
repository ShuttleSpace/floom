//! Fluent's radio, ported to the 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1 was a `Stack::horizontal` over three `Container`s -- an absolutely
//!   positioned hover halo, a dot and a bordered circle -- driven by five
//!   `on_event` listeners. It is the engine's `Radio` widget now: one `Widget`
//!   with `layout_request` and `paint`, which strokes the ring as an annulus and
//!   fills the dot rather than drawing a bordered box.
//! * The `checked` and `hover` `RwSignal`s are gone, and so were v1's two
//!   missing states: there is no `pressed` or `focused` signal to port, only the
//!   absence of one. The widget owns the selection and an `InteractionState`.
//! * v1's `on_change` was `Fn(bool)` and reported the new value. The engine's
//!   `on_select` is `Fn()` and toggles internally, so `view` reconstructs the
//!   value from the `checked` prop the widget was built with. See the note there.
//! * `cursor(Pointer)` and `gap(8.0)` are gone with the style layer: the engine's
//!   `Radio` has no cursor, and its `INDICATOR_LABEL_GAP` is 6.
//!
//! ## What v1 drew that this port cannot
//!
//! v1 ended its style closure with
//! `.focus(|s| s.outline(2.0).outline_color(colors().primary))`, so a focused
//! Fluent radio drew a 2px `primary` outline around the whole control.
//! `RadioStyle` has no outline field, the halo gates are per state rather than a
//! second layer, and the engine's `Radio` draws nothing else. So the outline is
//! gone, and `halo.on_focus` is `false` here for the honest reason that there is
//! no longer anything to draw on focus.
//!
//! Reported rather than approximated. Putting it back is a `focus_outline` on the
//! engine's `RadioStyle` -- two numbers and a colour for every kit to answer --
//! and it would be a design decision about where focus is drawn that this port
//! has no licence to make for a second kit that might disagree.
//!
//! ## Fluent and Cupertino differ in exactly one number
//!
//! Measured rather than assumed, because "the Fluent radio" is not a thing this
//! crate can check on its own. With the crate name and the design system enum
//! removed, this file and `cupertino-radio`'s are identical but for the halo's
//! tint: `on_surface` at 8 here against `primary` at 10 there. Same four points of
//! padding, same `size + 8.0`, same hover-only gate, same suppression while
//! disabled. So the two are kept as two crates, because the tint is the whole
//! difference and a shared implementation would be a place for it to hide.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::radio::RadioProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{FocusRing, Radio, RadioStyle};

/// v1's `let pad = 4.0`, applied to each side of the halo -- which is also the
/// halo's diameter and, in v1, the control's hit area. Material's halo is a flat
/// 40 instead; that difference is the reason this constant exists at all, and the
/// tests on both sides of it are what keep a future edit from quietly converging
/// the kits.
/// v1 wrote `gap(8.0)`; the engine's shared `INDICATOR_LABEL_GAP` is 6.0.
const LABEL_GAP: f32 = 8.0;

const HALO_PADDING: f32 = 4.0;

/// The halo's alpha, in the units `with_alpha` takes. Fluent tints `on_surface`
/// rather than `primary` and is correspondingly fainter: 8 against Cupertino's
/// 10 against Material's 20.
const HALO_ALPHA: u8 = 8;

/// v1 dimmed the disabled radio to `on_surface` at 38. It wrote that expression
/// twice -- once for the ring, once for the label -- and left the dot at a plain
/// `primary`, so the third layer is a difference the engine's single
/// `disabled_color` cannot carry. See `resolved_style`.
const DISABLED_ALPHA: u8 = 38;
/// v1 wrote `outline(2.0)`; `FocusRing` carries the number as a field.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// Fluent's radio. One option; grouping is the host's job, in this version and in
/// v1's.
pub struct FluentRadio {
    props: RadioProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentRadio {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: RadioProps {
                label: label.into(),
                ..Default::default()
            },
            // A fixed binding, not the live signal -- the same choice v1 made, and
            // the same one `fluent-slider` makes. Only Material's kits follow the
            // app theme here.
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Fluent)),
        }
    }

    /// Grouping value (informational; the host wires mutual exclusion).
    ///
    /// v1 carried it on the props and the engine's `Radio` has nowhere to put
    /// one. It stays here because the *group* is what reads it, and the group is
    /// still the host's to build.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.props.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    /// v1 answered this from an `RwSignal` it kept in step with the prop. There is
    /// no signal now -- the mounted widget owns the value -- so this is the prop
    /// it was built with, which is the same number before the first activation.
    pub fn is_checked(&self) -> bool {
        self.props.checked
    }

    pub fn props(&self) -> &RadioProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Fluent's appearance, as the engine's `RadioStyle`.
    ///
    /// ## Why this composes the disabled colour instead of branching
    ///
    /// `md-slider` had to branch over three colours when disabled, because its
    /// three layers dimmed to three *different* token alphas -- 60, 120 and 120.
    /// Here every layer dims to the same `on_surface`@38, which v1 wrote as one
    /// expression and repeated. So `RadioStyle`'s single `disabled_color` holds
    /// the whole answer and there is nothing to branch on: the only question is
    /// whether there *is* a disabled colour, and the builder's `disabled` flag
    /// answers it.
    ///
    /// Do not add per-layer disabled fields to the style for symmetry. A second
    /// layer that wanted a different colour is the signal that `disabled_color` is
    /// too small a shape, and that is the engine's type to widen -- once, for
    /// every kit -- rather than a field each kit fills in the same way.
    ///
    /// ## The halo and the touch target are one number
    ///
    /// v1 had no hit area of its own: the halo was the only thing wider than the
    /// ring, so `size + 8.0` is simultaneously this kit's halo diameter and its
    /// touch target. Writing it twice from one expression keeps them from drifting
    /// apart the next time the padding changes.
    pub fn resolved_style(&self) -> RadioStyle {
        let theme = self.theme.get();
        let ComponentTokens::Radio(tokens) = theme.component(ComponentId::Radio) else {
            unreachable!("ComponentId::Radio must resolve to RadioTokens")
        };
        let colors = theme.colors();

        // `None` while enabled, and `None` is not "no dim": `RadioStyle`'s doc
        // says a `None` `disabled_color` falls through to the engine's own
        // half-alpha tint. Writing `Some(colors.on_surface)` unconditionally would
        // dim every enabled radio to `on_surface` at 38, which is a different
        // mistake than the one this line is there to avoid.
        let disabled_color = self
            .props
            .disabled
            .then(|| to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)));

        let halo_size = tokens.size + HALO_PADDING * 2.0;

        RadioStyle {
            // --- from `RadioTokens`, as v1 read them ---
            size: tokens.size,
            border_width: tokens.border_width,
            // v1 drew `gap(8.0)` between the ring and the label. The engine's
            // shared `INDICATOR_LABEL_GAP` is 6.0, so restore it explicitly
            // rather than let every kit's label drift two points left.
            label_gap: LABEL_GAP,
            label_font_size: tokens.label_font_size,

            // --- this kit's own numbers ---
            touch_target: halo_size,

            // v1's circle was `.border(bw).border_color(..)` with no
            // `background`, so the middle of every radio showed whatever its
            // parent was painted with. The engine's default fills it white, and a
            // `Some` here would be a white disc inside a transparent ring.
            interior_color: None,

            // v1's `selected_color` was the *current* ring colour --
            // `if checked { primary } else { outline }` -- so it duplicated
            // `unselected_color` and meant neither. Splitting that one
            // conditional into the style's two independent fields is the whole
            // reason the field exists twice.
            selected_color: to_paint(colors.primary),
            unselected_color: to_paint(colors.outline),
            // v1's dot container read `colors().primary` with no disabled branch
            // anywhere, so a *disabled* v1 radio drew a dim ring around a
            // full-strength dot. `disabled_color` covers the dot here as well,
            // because one field for all three layers is the shape the engine chose
            // and nothing in `RadioStyle` can exempt one of them. Reported rather
            // than worked around: reproducing the old pixels would mean a
            // per-layer disabled colour in the engine, which is not this port's
            // to add.
            selected_dot_color: to_paint(colors.primary),

            // v1 had no hover or press colour for the ring at all: its entire
            // interaction answer was the halo. `None` means "no variant of this
            // kit's own", which is a real answer and not the same as the engine's
            // default, where both are `Some` and would repaint a Fluent radio
            // under the pointer in two greys it never had.
            unselected_hovered_color: None,
            unselected_pressed_color: None,

            disabled_color,

            label_color: to_paint(colors.on_surface),

            halo: StateLayer {
                // `on_surface`, not `primary` -- the one number that separates
                // this kit from Cupertino's, and the reason the two crates stay
                // separate rather than sharing an implementation.
                color: to_paint(with_alpha(colors.on_surface, HALO_ALPHA)),
                // v1 read one alpha, from `hover` alone. There is no pressed
                // variant to carry, so `None` -- which means the halo keeps
                // `color` -- rather than a second copy of it.
                color_pressed: None,
                // Relative, four points of padding on each side. Material's is a
                // flat 40, and the two must not converge.
                size: halo_size,
                shape: LayerShape::Circle,
                // v1's condition was `hover && !disabled`, and nothing else ever
                // turned the halo on -- no press, no focus (see the module doc).
                on_hover: true,
                on_press: false,
                on_focus: false,
                suppressed_when_disabled: true,
            },

            // v1 ended this closure with
            // `.focus(|s| s.outline(2.0).outline_color(colors().primary))`, so a
            // focused Fluent radio drew a 2px `primary` ring
            // around the whole control -- indicator and label together. That is
            // exactly a `FocusRing`, and `halo.on_focus` is `false` below so the
            // two mechanisms do not both fire.
            focus_ring: Some(FocusRing {
                width: FOCUS_RING_WIDTH,
                color: to_paint(colors.primary),
                gap: 0.0,
                radius: None,
            }),
        }
    }

    /// The radio, ready to mount.
    ///
    /// ## `on_select` is `Fn()`, and the kit still owes its caller a `Fn(bool)`
    ///
    /// The engine owns the value and says only "the user activated me". The new
    /// value is therefore computed here, from the `checked` prop the widget was
    /// built with: an activation is a `false -> true` edge, so it is `!was`.
    ///
    /// That is correct for a builder-time `selected` prop, which is all a kit
    /// has. It is *not* a bridge to the live value, and none was invented: a
    /// `value_signal` or an `RwSignal` here would be a second source of truth for
    /// a widget that already owns one.
    ///
    /// `RadioProps::on_change` is a `Box<dyn Fn(bool)>` whose `Clone` impl
    /// deliberately drops the callback, so it is moved rather than borrowed --
    /// which works only because `view` consumes the kit.
    ///
    /// One consequence worth stating plainly: the engine fires `on_select` only on
    /// that edge, and only from unselected, so the value a `Fn(bool)` receives here
    /// is always `true`. v1's toggle handler could also report `false`, because
    /// clicking a selected radio cleared it. That is not reachable from this side:
    /// the engine has no unselect, and faking one would mean the kit disagreeing
    /// with the widget about whether it is selected.
    pub fn view(self) -> Radio {
        let style = self.resolved_style();
        let was = self.props.checked;
        let label = self.props.label;
        let on_change = self.props.on_change;

        let mut radio = Radio::new(label)
            .selected(was)
            .disabled(self.props.disabled)
            .style(style);
        if let Some(on_change) = on_change {
            radio = radio.on_select(move || on_change(!was));
        }
        radio
    }
}

impl Default for FluentRadio {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// The engine's `RadioStyle`, under this kit's old name.
///
/// v1's `FluentRadioStyle` carried `selected_color`, `unselected_color`, `size`,
/// `border_width` and `label_font_size` -- and `selected_color` was the current
/// ring colour rather than the selected one, so the struct said the same thing
/// twice and nothing read it anyway. The alias keeps the name resolvable for
/// callers without pretending a struct still exists. The `design_system` field
/// has no successor: the engine's style names no design system, because it is not
/// one.
pub type FluentRadioStyle = RadioStyle;
