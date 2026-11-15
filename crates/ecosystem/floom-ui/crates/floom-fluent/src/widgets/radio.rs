//! Fluent's radio, ported to the 2.0 widget API.

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

const LABEL_GAP: f32 = 8.0;

const HALO_PADDING: f32 = 4.0;

const HALO_ALPHA: u8 = 8;

const DISABLED_ALPHA: u8 = 38;
const FOCUS_RING_WIDTH: f32 = 2.0;

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
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Fluent)),
        }
    }

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

    pub fn resolved_style(&self) -> RadioStyle {
        let theme = self.theme.get();
        let ComponentTokens::Radio(tokens) = theme.component(ComponentId::Radio) else {
            unreachable!("ComponentId::Radio must resolve to RadioTokens")
        };
        let colors = theme.colors();

        let disabled_color = self
            .props
            .disabled
            .then(|| to_paint(with_alpha(colors.on_surface, DISABLED_ALPHA)));

        let halo_size = tokens.size + HALO_PADDING * 2.0;

        RadioStyle {
            size: tokens.size,
            border_width: tokens.border_width,
            label_gap: LABEL_GAP,
            label_font_size: tokens.label_font_size,

            touch_target: halo_size,

            interior_color: None,

            selected_color: to_paint(colors.primary),
            unselected_color: to_paint(colors.outline),
            selected_dot_color: to_paint(colors.primary),

            unselected_hovered_color: None,
            unselected_pressed_color: None,

            disabled_color,

            label_color: to_paint(colors.on_surface),

            halo: StateLayer {
                color: to_paint(with_alpha(colors.on_surface, HALO_ALPHA)),
                color_pressed: None,
                size: halo_size,
                shape: LayerShape::Circle,
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

pub type FluentRadioStyle = RadioStyle;
