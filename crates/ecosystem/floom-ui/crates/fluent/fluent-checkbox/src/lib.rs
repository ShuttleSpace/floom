//! Fluent's checkbox, ported to the 2.0 widget API.

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

pub const DESIGN_SYSTEM: &str = "fluent";

const LABEL_GAP: f32 = 8.0;

const BORDER_WIDTH: f32 = 2.0;

const MARK_SCALE: f32 = 0.7;

const LAYER_PADDING: f32 = 4.0;

const LAYER_ALPHA: u8 = 8;

const DISABLED_CONTAINER_ALPHA: u8 = 12;
const DISABLED_ALPHA: u8 = 38;

const FOCUS_RING_WIDTH: f32 = 2.0;

const ICON_VIEW_BOX: floom_types::Rect = floom_types::Rect {
    x0: 0.0,
    y0: 0.0,
    x1: 24.0,
    y1: 24.0,
};

pub struct FluentCheckbox {
    props: CheckboxProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentCheckbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: CheckboxProps {
                label: label.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Fluent)),
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

        let layer_size = tokens.size + LAYER_PADDING * 2.0;

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
                color: to_paint(with_alpha(colors.on_surface, LAYER_ALPHA)),
                color_pressed: None,
                size: layer_size,
                shape: LayerShape::Box {
                    padding: LAYER_PADDING,
                },
                on_hover: true,
                on_press: false,
                on_focus: false,
                suppressed_when_disabled: true,
            },

            focus_ring: Some(FocusRing {
                width: FOCUS_RING_WIDTH,
                color: to_paint(colors.primary),
                gap: 0.0,
                radius: None,
            }),
        }
    }

    pub fn view(self) -> Checkbox {
        let style = self.resolved_style();
        let label = self.props.label;
        let on_change = self.props.on_change;

        let path = floom_widget_fluent_icon_paths::fluent_icon_path("check");
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

impl Default for FluentCheckbox {
    fn default() -> Self {
        Self::new(String::new())
    }
}

pub type FluentCheckboxStyle = CheckboxStyle;
