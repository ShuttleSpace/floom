use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::alert::{AlertProps, AlertVariant};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_md_icon::MdIcon;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, PartialEq)]
pub struct MdAlertStyle {
    pub design_system: &'static str,
    pub accent: Color,
    pub container_color: Color,
    pub text_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub font_size: f32,
    pub title_font_size: f32,
}

/// Semantic accent colour for each alert variant. Material has no built-in
/// info/success/warning tokens, so those are derived from fixed semantic RGB;
/// error reuses the theme's `error` role.
pub fn alert_accent(theme: &MultiTheme, variant: AlertVariant) -> Color {
    match variant {
        AlertVariant::Error => theme.colors().error,
        AlertVariant::Info => Color::rgb(33, 150, 243),
        AlertVariant::Success => Color::rgb(76, 175, 80),
        AlertVariant::Warning => Color::rgb(255, 152, 0),
    }
}

pub struct MdAlert {
    props: AlertProps,
    theme: RwSignal<MultiTheme>,
}

impl MdAlert {
    pub fn new() -> Self {
        Self {
            props: AlertProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.props.dismissible = dismissible;
        self
    }

    pub fn on_dismiss(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_dismiss = Some(Box::new(callback));
        self
    }

    pub fn resolved_style(&self) -> MdAlertStyle {
        let theme = self.theme.get();
        let accent = alert_accent(&theme, self.props.variant);
        let ComponentTokens::Alert(tokens) = theme.component(ComponentId::Alert) else {
            unreachable!("ComponentId::Alert must resolve to AlertTokens")
        };
        MdAlertStyle {
            design_system: "material",
            accent,
            container_color: with_alpha(accent, 26),
            text_color: theme.colors().on_surface,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            font_size: tokens.font_size,
            title_font_size: tokens.title_font_size,
        }
    }

    /// Build a tinted, rounded Material 3 alert banner with an optional dismiss button.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let title = self.props.title.clone();
        let message = self.props.message.clone();
        let variant = self.props.variant;
        let dismissible = self.props.dismissible;
        let on_dismiss: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.props.on_dismiss);

        let tokens = match theme.get().component(ComponentId::Alert) {
            ComponentTokens::Alert(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let radius = tokens.border_radius;
        let font_size = tokens.font_size;
        let title_font_size = tokens.title_font_size;

        let dot = Container::new(Label::new("".to_string())).style(move |s| {
            let a = alert_accent(&theme.get(), variant);
            s.width(10.0)
                .height(10.0)
                .border_radius(9999.0)
                .background(a.to_peniko())
        });

        let body = Stack::vertical((
            Label::new(title).style(move |s| {
                s.font_size(title_font_size)
                    .font_weight(FontWeight::Bold.to_floem())
                    .color(theme.get().colors().on_surface.to_peniko())
            }),
            Label::new(message).style(move |s| {
                s.font_size(font_size)
                    .color(theme.get().colors().on_surface_variant.to_peniko())
            }),
        ))
        .style(|s| s.gap(4.0));

        let content = Stack::horizontal((dot, body))
            .style(|s| s.gap(12.0).items_center())
            .into_any();

        let dismiss = if dismissible {
            MdIcon::new("close")
                .size(20.0)
                .view()
                .style(|s| s.padding(2.0).cursor(CursorStyle::Pointer))
                .on_event(listener::Click, move |_, _| {
                    if let Some(cb) = on_dismiss.as_ref() {
                        cb();
                    }
                    EventPropagation::Continue
                })
                .into_any()
        } else {
            Label::new("".to_string()).into_any()
        };

        Container::new(Stack::horizontal((content, dismiss)).style(|s| s.gap(8.0).items_center()))
            .style(move |s| {
                let accent = alert_accent(&theme.get(), variant);
                s.padding(padding)
                    .border_radius(radius)
                    .background(with_alpha(accent, 26).to_peniko())
                    .items_center()
            })
    }
}

impl Default for MdAlert {
    fn default() -> Self {
        Self::new()
    }
}
