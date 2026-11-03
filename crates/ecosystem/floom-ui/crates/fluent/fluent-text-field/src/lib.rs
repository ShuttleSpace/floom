//! Fluent outlined text field, ported to the 2.0 widget API.

pub const DESIGN_SYSTEM: &str = "fluent";

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::text_field::TextFieldProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_ui_composites::{FocusRing, TextField, TextFieldStyle};

const BORDER_WIDTH: f32 = 1.0;

const BORDER_WIDTH_FOCUSED: f32 = BORDER_WIDTH;

const DISABLED_ALPHA: u8 = 38;

const DISABLED_FILL_ALPHA: u8 = 4;

const FOCUS_RING_WIDTH: f32 = 2.0;

fn tinted(c: Color, alpha: u8) -> PaintColor {
    to_paint(with_alpha(c, alpha))
}

pub struct FluentTextField {
    props: TextFieldProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentTextField {
    pub fn new() -> Self {
        Self {
            props: TextFieldProps::default(),
            // Material's kits follow the theme signal. Note that
            theme: RwSignal::new(MultiTheme::new_for(DesignSystem::Fluent)),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.props.placeholder = placeholder.into();
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        self
    }

    pub fn password(mut self, is_password: bool) -> Self {
        self.props.is_password = is_password;
        self
    }

    pub fn max_length(mut self, max_length: usize) -> Self {
        self.props.max_length = Some(max_length);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_change(mut self, callback: impl Fn(&str) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &TextFieldProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> TextFieldStyle {
        let theme = self.theme.get();
        let ComponentTokens::TextField(tokens) = theme.component(ComponentId::TextField) else {
            unreachable!("ComponentId::TextField must resolve to TextFieldTokens")
        };
        let colors = theme.colors();

        TextFieldStyle {
            height: Some(tokens.height),
            padding_horizontal: tokens.padding_horizontal,
            border_radius: tokens.border_radius,

            border_width: BORDER_WIDTH,
            border_width_focused: BORDER_WIDTH_FOCUSED,
            border_color: to_paint(colors.outline),
            border_color_focused: to_paint(colors.primary),
            border_color_hovered: None,
            border_color_disabled: tinted(colors.on_surface, DISABLED_ALPHA),

            background_color: to_paint(colors.surface),
            background_color_disabled: tinted(colors.on_surface, DISABLED_FILL_ALPHA),

            text_color: to_paint(colors.on_surface),
            text_color_disabled: tinted(colors.on_surface, DISABLED_ALPHA),
            font_size: theme.typography().body.medium.font_size,

            focus_ring: Some(FocusRing {
                width: FOCUS_RING_WIDTH,
                color: to_paint(colors.primary),
                gap: 0.0,
                radius: None,
            }),
            ..TextFieldStyle::default()
        }
    }

    pub fn view(self) -> TextField {
        let style = self.resolved_style();

        let mut field = TextField::new()
            .value(self.props.value.clone())
            .placeholder(self.props.placeholder.clone())
            .disabled(self.props.disabled)
            .max_length(self.props.max_length)
            .style(style);

        if let Some(on_change) = self.props.on_change {
            field = field.on_change(move |text| on_change(text));
        }
        field
    }
}

impl Default for FluentTextField {
    fn default() -> Self {
        Self::new()
    }
}

pub type FluentTextFieldStyle = TextFieldStyle;
