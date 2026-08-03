use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdButtonStyle {
    pub background: Color,
    pub foreground: Color,
    pub height: f32,
    pub padding_horizontal: f32,
    pub border_radius: f32,
}

pub struct MdButton {
    props: ButtonProps,
    disabled: RwSignal<bool>,
    loading: RwSignal<bool>,
    pressed: RwSignal<bool>,
    hovered: RwSignal<bool>,
    focused: RwSignal<bool>,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdButton {
    pub fn new(label: impl Into<String>) -> Self {
        let props = ButtonProps {
            label: label.into(),
            ..Default::default()
        };
        Self {
            disabled: RwSignal::new(props.disabled),
            loading: RwSignal::new(props.loading),
            pressed: RwSignal::new(false),
            hovered: RwSignal::new(false),
            focused: RwSignal::new(false),
            theme: theme_signal(),
            props,
        }
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.props.size = size;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self.disabled.set(disabled);
        self
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self.loading.set(loading);
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    pub fn click(&self) {
        if !self.is_disabled() && !self.is_loading() {
            if let Some(callback) = &self.props.on_click {
                callback();
            }
        }
    }

    pub fn props(&self) -> &ButtonProps {
        &self.props
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.get()
    }

    pub fn is_loading(&self) -> bool {
        self.loading.get()
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed.get()
    }

    pub fn is_hovered(&self) -> bool {
        self.hovered.get()
    }

    pub fn is_focused(&self) -> bool {
        self.focused.get()
    }

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        MdButtonStyle {
            background: button_background(&theme, self.props.variant, self.is_disabled()),
            foreground: button_foreground(&theme, self.props.variant, self.is_disabled()),
            height: match self.props.size {
                ButtonSize::Small => tokens.height - 8.0,
                ButtonSize::Medium => tokens.height,
                ButtonSize::Large => tokens.height + 8.0,
            },
            padding_horizontal: tokens.padding_horizontal,
            border_radius: theme.radius().md,
        }
    }
}

fn button_background(
    theme: &floem_core_theme::theme::MaterialTheme,
    variant: ButtonVariant,
    disabled: bool,
) -> Color {
    if disabled {
        return with_alpha(theme.colors().on_surface, 31);
    }
    match variant {
        ButtonVariant::Primary => theme.colors().primary,
        ButtonVariant::Secondary => theme.colors().secondary_container,
        ButtonVariant::Danger => theme.colors().error,
        ButtonVariant::Ghost | ButtonVariant::Text => Color::rgba(0, 0, 0, 0),
    }
}

fn button_foreground(
    theme: &floem_core_theme::theme::MaterialTheme,
    variant: ButtonVariant,
    disabled: bool,
) -> Color {
    if disabled {
        return with_alpha(theme.colors().on_surface, 97);
    }
    match variant {
        ButtonVariant::Primary => theme.colors().on_primary,
        ButtonVariant::Secondary => theme.colors().on_secondary_container,
        ButtonVariant::Danger => theme.colors().on_error,
        ButtonVariant::Ghost | ButtonVariant::Text => theme.colors().primary,
    }
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color { a: alpha, ..color }
}
