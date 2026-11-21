//! Cupertino's checkbox, ported to the 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::checkbox::CheckboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Checkbox, CheckboxStyle, FocusRing};
use floom_widgets::VectorIcon;

/// v1's style struct carried this as a field; the engine's `CheckboxStyle` has no
/// such field, because the engine is not a design system. It is a const here so a
/// caller that needs to say which kit it is holding still can.
pub const DESIGN_SYSTEM: &str = "cupertino";

/// v1 wrote `gap(8.0)` between the indicator and the label; the engine's shared
/// `INDICATOR_LABEL_GAP` is 6.0. Restored explicitly rather than let every kit's
/// label drift two points left.
const LABEL_GAP: f32 = 8.0;

/// The rim. All three v1 kits wrote `.border(2.0)`; the engine drew none at all.
const BORDER_WIDTH: f32 = 2.0;

/// The checkmark's edge as a fraction of the indicator. v1 sized its `svg` view
/// `size * 0.7` on both axes.
const MARK_SCALE: f32 = 0.7;

/// v1's `let pad = 4.0`, applied to each side of the hover box. Material's layer
/// is a flat 40 instead; that difference is the reason this constant exists at
/// all, and the tests on both sides of it are what keep a future edit from
/// quietly converging the kits.
const LAYER_PADDING: f32 = 4.0;

/// The layer's alpha. `primary` at 10, against Material's `primary` at 20 and Fluent's
/// `on_surface` at 8. `primary`, not `on_surface`, is the one number that
/// separates this kit from Fluent's.
const LAYER_ALPHA: u8 = 10;

/// v1's three disabled colours: `on_surface` at 12 for the fill, and at 38 for
/// the rim, the mark and the label. Four writes of two numbers, and one of them
/// is a different number -- which is why `CheckboxStyle` has a disabled field
/// per layer rather than one `disabled_color`.
const DISABLED_CONTAINER_ALPHA: u8 = 12;
const DISABLED_ALPHA: u8 = 38;

/// v1 ended its style closure with
/// `.focus(|s| s.outline(2.0).outline_color(colors().primary))`; `FocusRing`
/// carries the number as a field.
const FOCUS_RING_WIDTH: f32 = 2.0;

/// The view box the icon set is authored in. Every glyph in it is 24x24.
const ICON_VIEW_BOX: floom_types::Rect = floom_types::Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

/// Cupertino's checkbox. One boolean; grouping is the host's job, in this version
/// and in v1's.
pub struct CupertinoCheckbox {
    props: CheckboxProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: CheckboxProps {
                label: label.into(),
                ..Default::default()
            },
            // A fixed binding, not the live signal -- the same choice v1 made,
            // and the same one `cupertino-radio` makes. Only Material's kits
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Cupertino)),
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

    /// Cupertino's appearance, as the engine's `CheckboxStyle`.
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
    /// ## The layer's size is written once
    ///
    /// v1 had no hit area of its own: the hover box was the only thing wider
    /// than the indicator. Unlike the radio port there is no `touch_target` field
    /// on `CheckboxStyle` to keep in step, so one expression is the whole story.
    pub fn resolved_style(&self) -> CheckboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Checkbox(tokens) = theme.component(ComponentId::Checkbox) else {
            unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens")
        };
        let colors = theme.colors();

        let layer_size = tokens.size + LAYER_PADDING * 2.0;

        CheckboxStyle {
            // --- from `CheckboxTokens`, as v1 read them ---
            size: tokens.size,
            border_radius: tokens.border_radius,
            border_width: BORDER_WIDTH,
            // v1 drew `gap(8.0)`; the engine's shared gap is 6.0.
            label_gap: LABEL_GAP,
            // v1 read `typography().body.medium.font_size`, which is not a
            // `CheckboxTokens` field -- the tokens carry only `size` and
            label_font_size: theme.typography().body.medium.font_size,

            // v1's `bg` was `if checked { primary } else { transparent }`, then
            // overwritten with `on_surface`@12 while disabled. Splitting that
            // one conditional into three independent fields is the whole reason
            container_color: to_paint(colors.primary),
            container_color_unchecked: to_paint(Color::TRANSPARENT),
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

            mark_color: to_paint(colors.on_primary),
            mark_color_unchecked: to_paint(Color::TRANSPARENT),
            mark_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),
            mark_scale: MARK_SCALE,

            label_color: to_paint(colors.on_surface),
            label_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),

            layer: StateLayer {
                color: to_paint(with_alpha(colors.primary, LAYER_ALPHA)),
                // v1 read one alpha, from `hover` alone. There is no pressed
                // variant to carry, so `None` -- which means the layer keeps
                color_pressed: None,
                // Relative: four points of padding on each side. Material's is a
                size: layer_size,
                shape: LayerShape::Box {
                    padding: LAYER_PADDING,
                },
                // v1's condition was `hover && !disabled`, and nothing else ever
                // turned the layer on -- no press, and no focus (see below).
                on_hover: true,
                on_press: false,
                on_focus: false,
                suppressed_when_disabled: true,
            },

            // `.focus(|s| s.outline(2.0).outline_color(colors().primary))`, so a
            // focused checkbox drew a 2px `primary` ring around the whole
            // control -- indicator and label together, because the closure was on
            // the container holding both. That is exactly a `FocusRing`, and
            // `layer.on_focus` is `false` above so the two mechanisms do not both
            focus_ring: Some(FocusRing {
                width: FOCUS_RING_WIDTH,
                color: to_paint(colors.primary),
                gap: 0.0,
                radius: None,
            }),
        }
    }

    /// The checkbox, ready to mount.
    ///
    /// ## The checkmark is content, so it is a builder
    ///
    /// v1 handed `cupertino_icon_path("check")` to an `svg` view. 2.0 has no SVG pipeline,
    /// so the path is parsed once into a `VectorIcon` and handed to
    /// `Checkbox::mark` -- a builder rather than a style field, because a mark is
    /// content the control draws and not a design system's opinion about it.
    ///
    /// The icon carries no colour of its own here: the engine recolours the mark
    /// from the style at paint, so there is no second answer sitting in the
    /// widget for a reader to find.
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

        let path = crate::widgets::icon_paths::cupertino_icon_path("check");
        let mark = VectorIcon::from_path_data(path, ICON_VIEW_BOX).color(style.mark_color);

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

impl Default for CupertinoCheckbox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

/// The engine's `CheckboxStyle`, under this kit's old name.
///
/// v1's `CupertinoCheckboxStyle` carried `design_system`, `container_color`,
/// `mark_color`, `outline_color`, `size` and `border_radius` -- and nothing ever
/// read it, because every `view()` resolved its own colours inline. The alias
/// keeps the name resolvable for callers without pretending a struct still
/// exists. The `design_system` field has no successor: the engine's style names
/// no design system, because it is not one, so the string is [`DESIGN_SYSTEM`]
/// instead.
pub type CupertinoCheckboxStyle = CheckboxStyle;
