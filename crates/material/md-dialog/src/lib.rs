use std::rc::Rc;

use floem::prelude::*;
use floem::style::CursorStyle;
use floem::event::EventPropagation;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::dialog::DialogProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdDialogStyle {
    pub scrim_color: Color,
    pub container_color: Color,
    pub content_color: Color,
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
}

pub struct MdDialog {
    props: DialogProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl MdDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: theme_signal(),
            props,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.props.title = title.into();
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.props.content = content.into();
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self.open.set(open);
        self
    }

    pub fn on_close(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_close = Some(Box::new(callback));
        self
    }

    pub fn close(&mut self) {
        self.props.open = false;
        self.open.set(false);
        if let Some(callback) = &self.props.on_close {
            callback();
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.get()
    }

    /// Expose the internal open signal so an external trigger (e.g. a button)
    /// can show or hide the dialog.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    pub fn props(&self) -> &DialogProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdDialogStyle {
        let theme = self.theme.get();
        let ComponentTokens::Dialog(tokens) = theme.component(ComponentId::Dialog) else {
            unreachable!("ComponentId::Dialog must resolve to DialogTokens")
        };
        MdDialogStyle {
            scrim_color: theme.colors().scrim,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            max_width: tokens.max_width,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
        }
    }

    /// Build a renderable Material 3 dialog rendered on a full-screen scrim.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let on_close = Rc::new(self.props.on_close);
        let title = self.props.title.clone();
        let content = self.props.content.clone();

        let tokens = match theme.get().component(ComponentId::Dialog) {
            ComponentTokens::Dialog(t) => t,
            _ => unreachable!(),
        };
        let max_width = tokens.max_width;
        let padding = tokens.padding;
        let radius = tokens.border_radius;

        let title_view = text(title).style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(t.typography().headline.small.font_size)
                .font_weight(FontWeight::Medium.to_floem())
        });

        let content_view = text(content).style(move |s| {
            let t = theme.get();
            s.color(with_alpha(t.colors().on_surface, 222).to_peniko())
                .font_size(t.typography().body.medium.font_size)
        });

        let close_btn = text("✕")
            .style(move |s| {
                let t = theme.get();
                s.color(t.colors().on_surface_variant.to_peniko())
                    .font_size(20.0)
                    .padding(4.0)
                    .cursor(CursorStyle::Pointer)
            })
            .on_event(listener::Click, {
                let open = open;
                let on_close = on_close.clone();
                move |_, _| {
                    open.set(false);
                    if let Some(cb) = on_close.as_ref() {
                        cb();
                    }
                    EventPropagation::Continue
                }
            });

        let card = Container::new(
            v_stack((
                h_stack((title_view, close_btn)).style(|s| s.items_center().justify_between()),
                content_view,
            ))
            .style(move |s| {
                let t = theme.get();
                s.gap(16.0)
                    .padding(padding)
                    .max_width(max_width)
                    .width_pct(100.0)
                    .background(t.colors().surface.to_peniko())
                    .border_radius(radius)
                    .color(t.colors().on_surface.to_peniko())
                    .box_shadow_blur(24.0)
                    .box_shadow_spread(1.0)
                    .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 30).to_peniko())
            }),
        )
        // Clicks inside the card must not dismiss the dialog.
        .on_event(listener::Click, |_, _| EventPropagation::Stop);

        let scrim = Container::new(card)
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .height_pct(100.0)
                    .background(with_alpha(t.colors().scrim, 160).to_peniko())
                    .items_center()
                    .justify_center()
                    .padding(24.0)
                    .apply_if(!open.get(), |s| s.hide())
            })
            .on_event(listener::Click, {
                let open = open;
                let on_close = on_close.clone();
                move |_, _| {
                    open.set(false);
                    if let Some(cb) = on_close.as_ref() {
                        cb();
                    }
                    EventPropagation::Continue
                }
            });

        Overlay::new(scrim)
    }
}

impl Default for MdDialog {
    fn default() -> Self {
        Self::new()
    }
}
