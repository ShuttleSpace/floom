use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::hover::HoverProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentHoverStyle {
    pub design_system: &'static str,
    pub highlight_alpha: f32,
    pub transition_ms: f32,
}

pub struct FluentHover {
    props: HoverProps,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
    hovered: RwSignal<bool>,
}

impl FluentHover {
    pub fn new() -> Self {
        Self {
            props: HoverProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            hovered: RwSignal::new(false),
        }
    }

    /// Set the element that reacts to pointer hover.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Fired whenever the hover state changes (true on enter, false on leave).
    pub fn on_hover_change(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.props.on_hover_change = Some(Box::new(callback));
        self
    }

    /// Component-owned hover signal (mirrors `open_signal()`). The host reads this to
    /// drive custom appearance (e.g. reveal an overlay) without the component imposing it.
    pub fn hovered_signal(&self) -> RwSignal<bool> {
        self.hovered
    }

    pub fn resolved_style(&self) -> FluentHoverStyle {
        let theme = self.theme.get();
        let ComponentTokens::Hover(tokens) = theme.component(ComponentId::Hover) else {
            unreachable!("ComponentId::Hover must resolve to HoverTokens")
        };
        FluentHoverStyle {
            design_system: "fluent",
            highlight_alpha: tokens.highlight_alpha,
            transition_ms: tokens.transition_ms,
        }
    }

    pub fn view(self) -> impl View {
        let style = self.resolved_style();
        let theme = self.theme;
        let hovered = self.hovered;
        let on_change: Rc<Option<Box<dyn Fn(bool)>>> = Rc::new(self.props.on_hover_change);
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());

        let on_enter = on_change.clone();
        let on_leave = on_change.clone();

        Container::new(child)
            .style(move |s| {
                let t = theme.get();
                s.apply_if(hovered.get(), |s| {
                    s.background(with_alpha(t.colors().on_surface, style.highlight_alpha as u8).to_peniko())
                })
            })
            .on_event(listener::PointerEnter, move |_, _| {
                hovered.set(true);
                if let Some(cb) = on_enter.as_ref() {
                    cb(true);
                }
                EventPropagation::Continue
            })
            .on_event(listener::PointerLeave, move |_, _| {
                hovered.set(false);
                if let Some(cb) = on_leave.as_ref() {
                    cb(false);
                }
                EventPropagation::Continue
            })
    }
}

impl Default for FluentHover {
    fn default() -> Self {
        Self::new()
    }
}
