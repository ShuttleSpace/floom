use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use crate::widgets::icon::MdIcon;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::checkbox::CheckboxProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::state::{LayerShape, StateLayer};
use floom_ui_composites::{Checkbox, CheckboxStyle};
use floom_widget_runtime::theme_signal;

const LABEL_GAP: f32 = 8.0;

const BORDER_WIDTH: f32 = 2.0;

const MARK_SCALE: f32 = 0.7;

const LAYER_SIZE: f32 = 40.0;

/// The layer's alpha while hovered or focused, and while pressed. Material is
/// the only kit of the three with a pressed tint at all.
const LAYER_ALPHA: u8 = 20;
const LAYER_ALPHA_PRESSED: u8 = 30;

const DISABLED_CONTAINER_ALPHA: u8 = 12;
const DISABLED_ALPHA: u8 = 38;

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

    pub fn resolved_style(&self) -> CheckboxStyle {
        let theme = self.theme.get();
        let ComponentTokens::Checkbox(tokens) = theme.component(ComponentId::Checkbox) else {
            unreachable!("ComponentId::Checkbox must resolve to CheckboxTokens")
        };
        let colors = theme.colors();

        CheckboxStyle {
            size: tokens.size,
            border_radius: tokens.border_radius,
            border_width: BORDER_WIDTH,
            label_gap: LABEL_GAP,
            label_font_size: theme.typography().body.medium.font_size,

            container_color: to_paint(colors.primary),
            container_color_unchecked: to_paint(Color::rgba(0, 0, 0, 0)),
            container_color_disabled: to_paint(with_alpha(
                colors.on_surface,
                DISABLED_CONTAINER_ALPHA,
            )),
            border_color: to_paint(colors.primary),
            border_color_unchecked: to_paint(colors.outline),
            border_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),

            mark_color: to_paint(colors.on_primary),
            mark_color_unchecked: to_paint(Color::rgba(0, 0, 0, 0)),
            mark_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),
            mark_scale: MARK_SCALE,

            label_color: to_paint(colors.on_surface),
            label_color_disabled: to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)),

            layer: StateLayer {
                color: to_paint(with_alpha(colors.primary, LAYER_ALPHA)),
                color_pressed: Some(to_paint(with_alpha(colors.primary, LAYER_ALPHA_PRESSED))),
                size: LAYER_SIZE,
                shape: LayerShape::Circle,
                on_hover: true,
                on_press: true,
                on_focus: true,
                suppressed_when_disabled: false,
            },

            focus_ring: None,
        }
    }

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

pub type MdCheckboxStyle = CheckboxStyle;
