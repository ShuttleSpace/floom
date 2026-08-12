use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem::style::CursorStyle;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::banner::{BannerVariant, BannerProps};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdBannerStyle {
    pub accent: Color,
    pub text_color: Color,
    pub height: f32,
    pub padding: f32,
    pub font_size: f32,
    pub action_font_size: f32,
    pub gap: f32,
    pub icon_size: f32,
}

pub fn banner_accent(theme: &MultiTheme, variant: BannerVariant) -> Color {
    match variant {
        BannerVariant::Error => theme.colors().error,
        BannerVariant::Info => Color::rgb(33, 150, 243),
        BannerVariant::Success => Color::rgb(76, 175, 80),
        BannerVariant::Warning => Color::rgb(255, 152, 0),
    }
}

pub struct MdBanner {
    props: BannerProps,
    theme: RwSignal<MultiTheme>,
}

impl MdBanner {
    pub fn new() -> Self {
        Self {
            props: BannerProps::default(),
            theme: theme_signal(),
        }
    }

    pub fn variant(mut self, variant: BannerVariant) -> Self {
        self.props.variant = variant;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.props.icon = Some(icon.into());
        self
    }

    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.props.action_label = Some(label.into());
        self
    }

    pub fn on_action(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_action = Some(Box::new(callback));
        self
    }

    pub fn single_line(mut self, single_line: bool) -> Self {
        self.props.single_line = single_line;
        self
    }

    pub fn resolved_style(&self) -> MdBannerStyle {
        let theme = self.theme.get();
        let accent = banner_accent(&theme, self.props.variant);
        let ComponentTokens::Banner(tokens) = theme.component(ComponentId::Banner) else {
            unreachable!("ComponentId::Banner must resolve to BannerTokens")
        };
        MdBannerStyle {
            accent,
            text_color: theme.colors().on_surface,
            height: tokens.height,
            padding: tokens.padding,
            font_size: tokens.font_size,
            action_font_size: tokens.action_font_size,
            gap: tokens.gap,
            icon_size: tokens.icon_size,
        }
    }

    /// Build a tinted, single-row Material 3 banner with an optional action.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let variant = props.variant;
        let message = props.message.clone();
        let icon = props.icon.clone();
        let action_label = props.action_label.clone();
        let on_action: Rc<Option<Box<dyn Fn()>>> = Rc::new(props.on_action);

        let tokens = match theme.get().component(ComponentId::Banner) {
            ComponentTokens::Banner(t) => t,
            _ => unreachable!(),
        };
        let padding = tokens.padding;
        let gap = tokens.gap;
        let font_size = tokens.font_size;
        let action_font_size = tokens.action_font_size;
        let icon_size = tokens.icon_size;

        let icon_view: AnyView = if let Some(ic) = icon {
            Container::new(text(ic))
                .style(move |s| {
                    let a = banner_accent(&theme.get(), variant);
                    s.font_size(icon_size).color(a.to_peniko())
                })
                .into_any()
        } else {
            empty().into_any()
        };

        let message_view = text(message)
            .style(move |s| {
                let t = theme.get();
                s.font_size(font_size).color(t.colors().on_surface.to_peniko())
            })
            .into_any();

        let left_group = h_stack((icon_view, message_view))
            .style(move |s| s.gap(gap).items_center())
            .into_any();

        let right: AnyView = if let Some(al) = action_label {
            Container::new(text(al))
                .style(move |s| {
                    let t = theme.get();
                    s.font_size(action_font_size)
                        .color(t.colors().primary.to_peniko())
                        .padding(4.0)
                        .cursor(CursorStyle::Pointer)
                })
                .on_event(listener::Click, {
                    let on_action = on_action.clone();
                    move |_, _| {
                        if let Some(cb) = on_action.as_ref() {
                            cb();
                        }
                        EventPropagation::Continue
                    }
                })
                .into_any()
        } else {
            empty().into_any()
        };

        Container::new(h_stack((left_group, right)).style(|s| s.items_center().justify_between()))
            .style(move |s| {
                let a = banner_accent(&theme.get(), variant);
                s.padding(padding)
                    .gap(gap)
                    .items_center()
                    .background(with_alpha(a, 26).to_peniko())
                    .min_height(tokens.height)
            })
    }
}

impl Default for MdBanner {
    fn default() -> Self {
        Self::new()
    }
}
