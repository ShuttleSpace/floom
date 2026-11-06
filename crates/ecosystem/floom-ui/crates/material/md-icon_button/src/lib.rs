use floom_signal::{RwSignal, SignalGet};
use std::sync::Arc;

use floom_types::Color as PaintColor;

use floom_ui_composites::{IconButton, IconButtonPaint, IconButtonStyle, IconButtonVariant};
use floom_widget_md_icon::MdIcon;

use floom_widget_md_icon::MdIconStyle;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::icon_button::{IconButtonProps, IconButtonVariant as PropsVariant};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

/// Fully transparent, which is what the text variant's fill and outline both are.
const CLEAR: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 0,
};

pub struct MdIconButton {
    props: IconButtonProps,
    theme: RwSignal<MultiTheme>,
    on_click: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

impl MdIconButton {
    pub fn new(icon: impl Into<String>) -> Self {
        Self {
            props: IconButtonProps {
                icon: icon.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            on_click: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.props.size = Some(size);
        self
    }

    pub fn variant(mut self, variant: PropsVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn on_click(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        let shared: Arc<dyn Fn() + Send + Sync + 'static> = Arc::new(callback);
        self.props.on_click = Some(Box::new({
            let shared = shared.clone();
            move || shared()
        }));
        self.on_click = Some(shared);
        self
    }

    pub fn props(&self) -> &IconButtonProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn engine_variant(&self) -> IconButtonVariant {
        match self.props.variant {
            PropsVariant::Text => IconButtonVariant::Text,
            PropsVariant::Contained => IconButtonVariant::Contained,
            PropsVariant::Outlined => IconButtonVariant::Outlined,
        }
    }

    pub fn resolved_style(&self) -> IconButtonStyle {
        let theme = self.theme.get();
        let ComponentTokens::IconButton(tokens) = theme.component(ComponentId::IconButton) else {
            unreachable!("ComponentId::IconButton must resolve to IconButtonTokens")
        };
        let on_surface = theme.colors().on_surface;

        IconButtonStyle {
            size: self.props.size.unwrap_or(tokens.size),
            icon_size: tokens.icon_size,
            radius: tokens.radius,
            border_width: 1.0,

            foreground: to_paint(theme.colors().on_surface_variant),
            disabled_foreground: to_paint(with_alpha(on_surface, 38)),
            disabled_background: to_paint(with_alpha(on_surface, 12)),
            disabled_border: to_paint(with_alpha(on_surface, 38)),

            text: IconButtonPaint {
                background: to_paint(CLEAR),
                border: to_paint(CLEAR),
                ripple_ink: to_paint(with_alpha(on_surface, 24)),
            },
            contained: IconButtonPaint {
                background: to_paint(theme.colors().primary),
                border: to_paint(theme.colors().primary),
                ripple_ink: to_paint(with_alpha(theme.colors().on_primary, 24)),
            },
            outlined: IconButtonPaint {
                background: to_paint(CLEAR),
                border: to_paint(theme.colors().outline),
                ripple_ink: to_paint(with_alpha(on_surface, 24)),
            },
        }
    }

    /// The icon's resolved style, for a caller that wants to read the button's
    /// colours rather than mount it.
    pub fn icon_style(&self) -> MdIconStyle {
        MdIcon::new(self.props.icon.clone())
            .size(self.resolved_style().icon_size)
            .with_theme(self.theme.get())
            .resolved_style()
    }

    pub fn view(self) -> IconButton {
        let style = self.resolved_style();
        let icon_color = if self.props.disabled {
            style.disabled_foreground
        } else {
            style.foreground
        };
        let icon = MdIcon::new(self.props.icon.clone())
            .size(style.icon_size)
            .with_theme(self.theme.get())
            .resolve()
            .color(icon_color);

        let mut button = IconButton::new(icon)
            .variant(self.engine_variant())
            .disabled(self.props.disabled)
            .style(style);

        if let Some(on_click) = &self.on_click {
            let on_click = on_click.clone();
            button = button.on_click(move || on_click());
        }

        button
    }
}
