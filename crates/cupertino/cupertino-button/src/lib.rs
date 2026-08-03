use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_core_theme::tokens::color::Color;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoButtonStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub height: f32,
}

pub struct CupertinoButton {
    props: ButtonProps,
    theme: RwSignal<CupertinoTheme>,
}

impl CupertinoButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: ButtonProps {
                label: label.into(),
                ..Default::default()
            },
            theme: RwSignal::new(CupertinoTheme::new()),
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
        self
    }
    pub fn loading(mut self, loading: bool) -> Self {
        self.props.loading = loading;
        self
    }
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }
    pub fn props(&self) -> &ButtonProps {
        &self.props
    }

    pub fn with_theme(mut self, theme: CupertinoTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        CupertinoButtonStyle {
            design_system: DESIGN_SYSTEM,
            background: match self.props.variant {
                ButtonVariant::Primary => theme.colors().primary,
                ButtonVariant::Secondary => theme.colors().secondary_container,
                ButtonVariant::Danger => theme.colors().error,
                ButtonVariant::Ghost | ButtonVariant::Text => Color::rgba(0, 0, 0, 0),
            },
            foreground: match self.props.variant {
                ButtonVariant::Primary => theme.colors().on_primary,
                ButtonVariant::Secondary => theme.colors().on_secondary_container,
                ButtonVariant::Danger => theme.colors().on_error,
                ButtonVariant::Ghost | ButtonVariant::Text => theme.colors().primary,
            },
            height: tokens.height,
        }
    }
}
