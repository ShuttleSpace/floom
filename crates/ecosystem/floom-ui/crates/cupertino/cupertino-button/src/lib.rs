use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::style::CursorStyle;
use floom::views::{Decorators, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

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
    theme: RwSignal<MultiTheme>,
}

impl CupertinoButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            props: ButtonProps {
                label: label.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
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

    /// Build a renderable, interactive Cupertino button.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.props.disabled;
        let on_click = self.props.on_click;
        let label = self.props.label.clone();
        let variant = self.props.variant;
        let size = self.props.size;

        let tokens = match theme.get().component(ComponentId::Button) {
            ComponentTokens::Button(t) => t,
            _ => unreachable!(),
        };
        let height = match size {
            ButtonSize::Small => tokens.height - 8.0,
            ButtonSize::Medium => tokens.height,
            ButtonSize::Large => tokens.height + 8.0,
        };

        Button::new(Label::new(label))
            .style(move |s| {
                let t = theme.get();
                let (bg, fg) = base_colors(&t, variant, disabled);
                s.height(height)
                    .padding_horiz(tokens.padding_horizontal)
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .border_radius(t.radius().lg)
                    .font_weight(t.typography().label.large.font_weight.to_floom())
                    .font_size(t.typography().label.large.font_size)
                    .items_center()
                    .justify_center()
                    .keyboard_navigable()
                    .set_disabled(disabled)
                    .cursor(CursorStyle::Pointer)
                    .hover(|s| s.background(with_alpha(fg, 12).to_peniko()))
                    .active(|s| s.background(with_alpha(fg, 20).to_peniko()))
            })
            .action(move || {
                if let Some(cb) = &on_click {
                    cb();
                }
            })
    }
}

/// Cupertino button fill/foreground colors per variant.
fn base_colors(theme: &MultiTheme, variant: ButtonVariant, disabled: bool) -> (Color, Color) {
    if disabled {
        return (
            with_alpha(theme.colors().on_surface, 12),
            with_alpha(theme.colors().on_surface, 38),
        );
    }
    match variant {
        ButtonVariant::Primary => (theme.colors().primary, theme.colors().on_primary),
        ButtonVariant::Secondary => (
            theme.colors().secondary_container,
            theme.colors().on_secondary_container,
        ),
        ButtonVariant::Danger => (theme.colors().error, theme.colors().on_error),
        ButtonVariant::Ghost => (Color::rgba(0, 0, 0, 0), theme.colors().primary),
        ButtonVariant::Text => (Color::rgba(0, 0, 0, 0), theme.colors().primary),
    }
}
