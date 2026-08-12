use floem::prelude::*;
use floem::style::CursorStyle;
use floem::views::Decorators;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
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
    theme: RwSignal<floem_core_theme::theme::MultiTheme>,
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

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::Button(tokens) = theme.component(ComponentId::Button) else {
            unreachable!("ComponentId::Button must resolve to ButtonTokens")
        };
        let (background, foreground) = base_colors(&theme, self.props.variant, self.is_disabled());
        MdButtonStyle {
            background,
            foreground,
            height: match self.props.size {
                ButtonSize::Small => tokens.height - 8.0,
                ButtonSize::Medium => tokens.height,
                ButtonSize::Large => tokens.height + 8.0,
            },
            padding_horizontal: tokens.padding_horizontal,
            border_radius: theme.radius().md,
        }
    }

    /// Build a renderable, interactive Material 3 button.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = self.disabled;
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
        let outlined = matches!(variant, ButtonVariant::Ghost);

        Button::new(text(label))
            .style(move |s| {
                let t = theme.get();
                let (bg, fg) = base_colors(&t, variant, disabled.get());
                let hover_bg = state_layer(&t, variant, disabled.get(), true);
                let press_bg = state_layer(&t, variant, disabled.get(), false);
                let outline_c = t.colors().outline;
                s.height(height)
                    .padding_horiz(tokens.padding_horizontal)
                    .background(bg.to_peniko())
                    .color(fg.to_peniko())
                    .border_radius(t.radius().full)
                    .font_weight(t.typography().label.large.font_weight.to_floem())
                    .font_size(t.typography().label.large.font_size)
                    .items_center()
                    .justify_center()
                    .keyboard_navigable()
                    .set_disabled(disabled.get())
                    .cursor(CursorStyle::Pointer)
                    .apply_if(outlined, |s| {
                        s.border(1.0).border_color(outline_c.to_peniko())
                    })
                    .hover(|s| s.background(hover_bg.to_peniko()))
                    .active(|s| s.background(press_bg.to_peniko()))
                    .focus_visible(|s| {
                        s.outline(2.0).border(2.0).border_color(outline_c.to_peniko())
                    })
                    .disabled(|s| {
                        let d = disabled_colors(&t);
                        s.background(d.0.to_peniko()).color(d.1.to_peniko())
                    })
            })
            .action(move || {
                if let Some(cb) = &on_click {
                    cb();
                }
            })
    }
}

fn base_colors(theme: &MultiTheme, variant: ButtonVariant, disabled: bool) -> (Color, Color) {
    if disabled {
        let d = disabled_colors(theme);
        return (d.0, d.1);
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

/// M3 hover/press state layer: an 8%/12% on-surface (or on-primary) overlay
/// mixed over the base fill.
fn state_layer(theme: &MultiTheme, variant: ButtonVariant, disabled: bool, hover: bool) -> Color {
    if disabled {
        return disabled_colors(theme).0;
    }
    let overlay = match variant {
        ButtonVariant::Primary | ButtonVariant::Danger => theme.colors().on_primary,
        ButtonVariant::Secondary => theme.colors().on_secondary_container,
        ButtonVariant::Ghost | ButtonVariant::Text => theme.colors().primary,
    };
    let base = base_colors(theme, variant, false).0;
    let amt = if hover { 0.08 } else { 0.12 };
    mix(base, overlay, amt)
}

fn disabled_colors(theme: &MultiTheme) -> (Color, Color) {
    (
        with_alpha(theme.colors().on_surface, 12),
        with_alpha(theme.colors().on_surface, 38),
    )
}
