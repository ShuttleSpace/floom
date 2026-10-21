//! Material 3's checkbox, ported to the 2.0 widget API.
//!
//! ## What the port changed
//!
//! * v1 was a `Stack::horizontal` over a `Container` holding an absolutely
//!   positioned halo and an `svg` mark, plus a `Label`, driven by six `on_event`
//!   listeners. It is the engine's `Checkbox` widget now: one `Widget` with
//!   `layout_request` and `paint`, which strokes the rim over the fill and fills
//!   the checkmark as a path rather than as a document.
//! * The `checked` `RwSignal` is gone, and so are the `hover`, `pressed` and
//!   `focused` ones: the widget owns the value and an `InteractionState`, and a
//!   kit asking whether it is hovered was reaching past its own layer.
//!   `disabled` was a builder prop rather than a signal, so it survives and is
//!   still readable at style-resolution time.
//! * v1's `on_change` was `Fn(bool)` and so is the engine's `on_toggle`, but the
//!   engine flips the value itself and reports the one it moved *to* -- so the
//!   closure a host wrote for v1 crosses unchanged. That is the one thing here
//!   that needed no translating.
//! * `cursor(Pointer)` is gone with the style layer: the engine answers the
//!   cursor from `disabled` rather than from a style.
//!
//! ## The dead `resolved_style`, and the 97 this crate does not use
//!
//! v1's `MdCheckboxStyle` carried one answer for the rim -- `on_surface` at
//! **97** while disabled -- and *nothing ever read it*: every `view()` resolved
//! its own colours inline. All three kits' `resolved_style` functions were
//! byte-identical apart from the type name, and all three disagreed with their
//! own `view()`. The number v1 actually painted is 38, and that is the one this
//! port carries; 97 is pinned in the test as a dead value so the trap cannot
//! re-arm itself.
//!
//! The struct goes with it. `resolved_style` returns the engine's
//! `CheckboxStyle` now, which has a field per state rather than one answer and a
//! conditional.
//!
//! ## Material's state layer is drawn while disabled
//!
//! v1's halo condition was `hover || pressed || focused`, with no disabled check
//! anywhere in it -- unlike the other two kits, which wrote `!hover || disabled`.
//! It also guarded each of those three signals with `if !disabled` on the way
//! in, so a v1 Material checkbox never actually reached a state that would have
//! drawn the halo while disabled either: the two readings agree on every pixel
//! v1 drew and disagree only about the flag.
//!
//! This port sets the flag the way the style is written rather than the way the
//! event handlers happened to guard, because the flag is a claim about the
//! design system and the guards were a claim about the pointer. It is pinned by
//! a test that says so, which is better than a comment nobody reads.
//!
//! ## Material's layer is a flat 40, and Material answers focus with it
//!
//! Two numbers against the other two kits' one shape: the layer is
//! `.size(40.0, 40.0)` -- absolute, not `size + pad * 2.0` -- and it is gated on
//! focus as well as hover and press, so a focused Material checkbox shows its
//! layer and carries no `FocusRing`. Cupertino and Fluent draw a `size + 8` box
//! on hover alone and answer focus with a 2px ring. The three do not converge,
//! and the tests on both sides of each number are what keep a "harmonise the
//! kits" edit from quietly landing.

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::checkbox::CheckboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Checkbox, CheckboxStyle};
use floom_widget_md_icon::MdIcon;
use floom_widget_runtime::theme_signal;

/// v1 wrote `gap(8.0)` between the indicator and the label; the engine's shared
/// `INDICATOR_LABEL_GAP` is 6.0. Restored explicitly rather than let every kit's
/// label drift two points left.
const LABEL_GAP: f32 = 8.0;

/// The rim. All three v1 kits wrote `.border(2.0)`; the engine drew none at all.
const BORDER_WIDTH: f32 = 2.0;

/// The checkmark's edge as a fraction of the indicator. v1 sized its `svg` view
/// `size * 0.7` on both axes.
const MARK_SCALE: f32 = 0.7;

/// v1's halo: `.size(40.0, 40.0)`. An absolute number, not `size + pad * 2.0` as
/// the other two kits wrote -- see the module doc, and the test that pins the
/// difference.
const LAYER_SIZE: f32 = 40.0;

/// The layer's alpha while hovered or focused, and while pressed. Material is
/// the only kit of the three with a pressed tint at all.
const LAYER_ALPHA: u8 = 20;
const LAYER_ALPHA_PRESSED: u8 = 30;

/// v1's three disabled colours: `on_surface` at 12 for the fill, and at 38 for
/// the rim, the mark and the label. Four writes of two numbers, and one of them
/// is a different number -- which is why `CheckboxStyle` has a disabled field
/// per layer rather than one `disabled_color`.
const DISABLED_CONTAINER_ALPHA: u8 = 12;
const DISABLED_ALPHA: u8 = 38;


/// Material's checkbox. One boolean; grouping is the host's job, in this version
/// and in v1's.
pub struct MdCheckbox {
    props: CheckboxProps,
    theme: RwSignal<MultiTheme>,
}

impl MdCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: CheckboxProps {
                label: label.into(),
                ..Default::default()
            },
            // The app-wide signal, not a fixed binding: Material is the one
            // design system whose kits follow the live theme rather than
            // constructing their own. `md-radio` and `md-slider` do the same.
            theme: theme_signal(),
        }
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

    /// v1 answered this from an `RwSignal` it kept in step with the prop. There
    /// is no signal now -- the mounted widget owns the value -- so this is the
    /// prop it was built with, which is the same number before the first
    /// activation.
    pub fn is_checked(&self) -> bool {
        self.props.checked
    }

    pub fn props(&self) -> &CheckboxProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Material's appearance, as the engine's `CheckboxStyle`.
    ///
    /// ## Every state is a field, so `disabled` is not a branch
    ///
    /// v1 resolved three colours inside its style closure and then overwrote two
    /// of them when disabled. `CheckboxStyle` names each state instead, so
    /// `disabled` is not branched on here at all: every layer's disabled colour
    /// is written out, and the engine's own resolution -- which checks disabled
    /// *first* and covers both values -- is what makes a disabled *checked* box
    /// `on_surface`@12 rather than `primary` dimmed.
    ///
    /// ## What is not here
    ///
    /// v1's `MdCheckboxStyle` had no `design_system` field, so neither does this
    /// port: there is no `DESIGN_SYSTEM` const on this crate, unlike Cupertino's
    /// and Fluent's, because there was never one to carry across. The engine's
    /// style names no design system in any case, because it is not one.
    pub fn resolved_style(&self) -> CheckboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Checkbox(tokens) = theme.component(ComponentId::Checkbox) else {
            unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens")
        };
        let colors = theme.colors();

        CheckboxStyle {
            // --- from `CheckboxTokens`, as v1 read them ---
            size: tokens.size,
            border_radius: tokens.border_radius,
            border_width: BORDER_WIDTH,
            // v1 drew `gap(8.0)`; the engine's shared gap is 6.0.
            label_gap: LABEL_GAP,
            // v1 read `typography().body.medium.font_size`, which is not a
            // `CheckboxTokens` field -- the tokens carry only `size` and
            // `border_radius`.
            label_font_size: theme.typography().body.medium.font_size,

            // --- the indicator, resolved per state ---
            // v1's `bg` was `if checked { primary } else { transparent }`, then
            // overwritten with `on_surface`@12 while disabled. Splitting that
            // one conditional into three independent fields is the whole reason
            // the field exists three times.
            container_color: to_paint(colors.primary),
            container_color_unchecked: to_paint(Color::rgba(0, 0, 0, 0)),
            container_color_disabled: to_paint(with_alpha(
                colors.on_surface,
                DISABLED_CONTAINER_ALPHA,
            )),
            // v1's rim was `if checked { primary } else { outline }`, then
            // overwritten with `on_surface`@38. The 97 is what the dead struct
            // claimed and what v1 never painted -- see the module doc.
            border_color: to_paint(colors.primary),
            border_color_unchecked: to_paint(colors.outline),
            border_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),

            // --- the mark ---
            mark_color: to_paint(colors.on_primary),
            mark_color_unchecked: to_paint(Color::rgba(0, 0, 0, 0)),
            mark_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),
            mark_scale: MARK_SCALE,

            // --- the label ---
            label_color: to_paint(colors.on_surface),
            label_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),

            layer: StateLayer {
                color: to_paint(with_alpha(colors.primary, LAYER_ALPHA)),
                // The only kit of the three whose pressed tint differs from its
                // resting one.
                color_pressed: Some(to_paint(with_alpha(colors.primary, LAYER_ALPHA_PRESSED))),
                // Absolute, not `size + 8`: see `LAYER_SIZE`, and the test that
                // pins the difference from the other two kits.
                size: LAYER_SIZE,
                // v1 wrote `border_radius(9999.0)`, which is a circle -- and a
                // flat 40 circle, so it does not grow with `size` either.
                shape: LayerShape::Circle,
                // v1's `active` was `hover || pressed || focused` -- all three
                // gates, and no fourth one for the disabled state.
                on_hover: true,
                on_press: true,
                on_focus: true,
                suppressed_when_disabled: false,
            },

            // Material answers focus with its layer instead -- see `on_focus`
            // above -- so it must not also carry a ring, or a focused Material
            // checkbox would show both at once.
            focus_ring: None,
        }
    }

    /// The checkbox, ready to mount.
    ///
    /// ## The checkmark is content, so it is a builder
    ///
    /// v1 handed `md_icon_svg("check")` to an `svg` view. 2.0 has no SVG
    /// pipeline, so the icon is parsed once into a `VectorIcon` and handed to
    /// `Checkbox::mark` -- a builder rather than a style field, because a mark is
    /// content the control draws and not a design system's opinion about it.
    ///
    /// `MdIcon::resolve` reads the theme for the icon's *own* colour, which is
    /// `on_surface_variant` and has nothing to do with a checkbox. The engine
    /// recolours the mark from the style at paint, so the resolved colour is
    /// overwritten here rather than left as a second answer sitting in the
    /// widget.
    ///
    /// ## `on_toggle` is `Fn(bool)` and reports the value it moved to
    ///
    /// Which is what v1's `on_change` did too, so the callback crosses
    /// unchanged. Unlike the radio ports there is no `!was` reconstruction and no
    /// edge to reason about: the engine flips the value and fires, and a checkbox
    /// really does report both directions.
    pub fn view(self) -> Checkbox {
        let style = self.resolved_style();
        let label = self.props.label;
        let on_change = self.props.on_change;

        let mark = MdIcon::new("check")
            .with_theme(self.theme.get())
            .resolve()
            .color(style.mark_color);

        let mut checkbox = Checkbox::new(label)
            .checked(self.props.checked)
            .disabled(self.props.disabled)
            .style(style)
            .mark(mark);
        if let Some(on_change) = on_change {
            checkbox = checkbox.on_toggle(on_change);
        }
        checkbox
    }
}

impl Default for MdCheckbox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// The engine's `CheckboxStyle`, under this kit's old name.
///
/// v1's `MdCheckboxStyle` carried `container_color`, `mark_color`,
/// `outline_color`, `size` and `border_radius` -- and nothing ever read it,
/// because every `view()` resolved its own colours inline. It had no
/// `design_system` field and it claimed a disabled alpha of 97 that no screen
/// ever showed. The alias keeps the name resolvable for callers without
/// pretending a struct still exists.
pub type MdCheckboxStyle = CheckboxStyle;
