use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::overlay::OverlayProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdOverlayStyle {
    pub scrim_color: Color,
    pub content_padding: f32,
}

pub struct MdOverlay {
    props: OverlayProps,
    show: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
}

impl MdOverlay {
    pub fn new() -> Self {
        let props = OverlayProps::default();
        Self {
            show: RwSignal::new(false),
            props,
            theme: theme_signal(),
            child: None,
        }
    }

    /// Show or hide the overlay.
    pub fn show(self, show: bool) -> Self {
        self.show.set(show);
        self
    }

    /// Supply the content rendered above the scrim.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Fired when the backdrop (scrim) is clicked — typically wired to dismiss.
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.props.on_click = Some(Box::new(callback));
        self
    }

    /// Expose the internal show signal so an external trigger can show/hide it.
    pub fn show_signal(&self) -> RwSignal<bool> {
        self.show
    }

    /// Set the internal show signal from an external signal.
    pub fn with_show_signal(mut self, show: RwSignal<bool>) -> Self {
        self.show = show;
        self
    }

    pub fn props(&self) -> &OverlayProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdOverlayStyle {
        let theme = self.theme.get();
        let ComponentTokens::Overlay(tokens) = theme.component(ComponentId::Overlay) else {
            unreachable!("ComponentId::Overlay must resolve to OverlayTokens")
        };
        MdOverlayStyle {
            scrim_color: theme.colors().scrim,
            content_padding: tokens.content_padding,
        }
    }

    /// Build a Material 3 overlay: a translucent scrim with the supplied
    /// content centered above it. Hidden when `show` is false.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let show = self.show;
        let on_click = Rc::new(self.props.on_click);
        let opacity = self.props.opacity;
        let pad = match theme.get().component(ComponentId::Overlay) {
            ComponentTokens::Overlay(t) => t.content_padding,
            _ => unreachable!(),
        };
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let content = Container::new(child)
            .style(move |s| {
                let t = theme.get();
                s.padding(pad)
                    .background(t.colors().surface.to_peniko())
                    .color(t.colors().on_surface.to_peniko())
                    .border_radius(12.0)
            })
            // Clicks inside the content must not dismiss the overlay.
            .on_event(listener::Click, |_, _| EventPropagation::Stop);

        let scrim = Container::new(content)
            .style(move |s| {
                let t = theme.get();
                let alpha = (opacity.clamp(0.0, 1.0) * 255.0) as u8;
                s.width_pct(100.0)
                    .height_pct(100.0)
                    .background(with_alpha(t.colors().scrim, alpha).to_peniko())
                    .items_center()
                    .justify_center()
                    .apply_if(!show.get(), |s| s.hide())
            })
            .on_event(listener::Click, {
                let on_click = on_click.clone();
                move |_, _| {
                    show.set(false);
                    if let Some(cb) = on_click.as_ref() {
                        cb();
                    }
                    EventPropagation::Continue
                }
            });

        Overlay::new(scrim)
    }
}

impl Default for MdOverlay {
    fn default() -> Self {
        Self::new()
    }
}
