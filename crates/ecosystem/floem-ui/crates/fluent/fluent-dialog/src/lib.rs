use std::rc::Rc;
use std::time::Duration;

use floem::prelude::*;
use floem::style::CursorStyle;
use floem::event::EventPropagation;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::dialog::DialogProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_core_theme::tokens::typography::FontWeight;
use floem_widget_fluent_icon::FluentIcon;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentDialogStyle {
    pub design_system: &'static str,
    pub scrim_color: Color,
    pub container_color: Color,
    pub content_color: Color,
    pub max_width: f32,
    pub padding: f32,
    pub border_radius: f32,
}

pub struct FluentDialog {
    props: DialogProps,
    open: RwSignal<bool>,
    theme: RwSignal<MultiTheme>,
}

impl FluentDialog {
    pub fn new() -> Self {
        let props = DialogProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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

    /// Add an action button rendered at the bottom of the dialog. Clicking it
    /// fires `callback` and closes the dialog.
    pub fn action(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
        self.props.actions.push((label.into(), Box::new(callback)));
        self
    }
    pub fn props(&self) -> &DialogProps {
        &self.props
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

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> FluentDialogStyle {
        let theme = self.theme.get();
        let ComponentTokens::Dialog(tokens) = theme.component(ComponentId::Dialog) else {
            unreachable!("ComponentId::Dialog must resolve to DialogTokens")
        };
        FluentDialogStyle {
            design_system: DESIGN_SYSTEM,
            scrim_color: theme.colors().scrim,
            container_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            max_width: tokens.max_width,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
        }
    }

    /// Build a renderable Fluent dialog rendered on a full-screen scrim.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let on_close = Rc::new(self.props.on_close);
        let title = self.props.title.clone();
        let content = self.props.content.clone();
        let actions = self.props.actions;

        let tokens = match theme.get().component(ComponentId::Dialog) {
            ComponentTokens::Dialog(t) => t,
            _ => unreachable!(),
        };
        let max_width = tokens.max_width;
        let padding = tokens.padding;
        let radius = theme.get().radius().sm;

        // Action buttons rendered at the bottom of the dialog.
        let actions_view = {
            let buttons: Vec<AnyView> = actions
                .into_iter()
                .map(|(label, cb)| {
                    let open = open;
                    Container::new(Label::new(label))
                        .style(move |s| {
                            let t = theme.get();
                            let hover = with_alpha(t.colors().primary, 10).to_peniko();
                            s.color(t.colors().primary.to_peniko())
                                .font_size(t.typography().label.large.font_size)
                                .font_weight(FontWeight::SemiBold.to_floem())
                                .padding_horiz(12.0)
                                .padding_vert(10.0)
                                .border_radius(8.0)
                                .cursor(CursorStyle::Pointer)
                                .hover(move |s| s.background(hover))
                        })
                        .on_event(listener::Click, move |_, _| {
                            cb();
                            open.set(false);
                            EventPropagation::Continue
                        })
                        .into_any()
                })
                .collect();
            Stack::horizontal_from_iter(buttons)
                .style(|s| s.gap(8.0).justify_end().items_center())
                .into_any()
        };

        let title_view = Label::new(title).style(move |s| {
            let t = theme.get();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(t.typography().headline.small.font_size)
                .font_weight(FontWeight::SemiBold.to_floem())
        });

        let content_view = Label::new(content).style(move |s| {
            let t = theme.get();
            s.color(with_alpha(t.colors().on_surface, 222).to_peniko())
                .font_size(t.typography().body.medium.font_size)
        });

        let close_btn = FluentIcon::new("close")
            .size(20.0)
            .view()
            .style(|s| s.padding(4.0).cursor(CursorStyle::Pointer))
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
            Stack::vertical((
                Stack::horizontal((title_view, close_btn)).style(|s| s.items_center().justify_between()),
                content_view,
                actions_view,
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
        .on_event(listener::Click, |_, _| EventPropagation::Stop)
        // Pop in with a subtle scale, pop out on close.
        .animation(|a| {
            a.run_on_create(true)
                .run_on_remove(true)
                .duration(Duration::from_millis(220))
                .keyframe(0, |f| f.style(|s| s.scale(95.pct())))
                .keyframe(100, |f| f.style(|s| s.scale(100.pct())))
        });

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
            })
            // Fade the scrim in/out alongside the card.
            .animation(|a| {
                a.run_on_create(true)
                    .run_on_remove(true)
                    .duration(Duration::from_millis(220))
                    .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                    .keyframe(100, |f| f.computed_style())
            });

        Overlay::new(scrim)
    }
}

impl Default for FluentDialog {
    fn default() -> Self {
        Self::new()
    }
}
