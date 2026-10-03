use std::time::Duration;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::view::AnyView;
use floom::IntoView;
use floom::views::{Decorators, Empty, Stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentNavigationDrawerStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub scrim: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
}

pub struct FluentNavigationDrawer {
    props: NavigationDrawerProps,
    open: RwSignal<bool>,
    header: Option<AnyView>,
    child: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl FluentNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: RwSignal::new(props.open),
            header: None,
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
            props,
        }
    }
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        self.open.set(open);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.props.width = width;
        self
    }
    pub fn elevation(mut self, elevation: u8) -> Self {
        self.props.elevation = elevation;
        self
    }
    /// Choose the edge the drawer slides in from (`Left` or `Right`).
    pub fn side(mut self, side: NavigationDrawerSide) -> Self {
        self.props.side = side;
        self
    }
    /// Optional header content shown at the top of the drawer panel.
    pub fn header(mut self, header: impl IntoView + 'static) -> Self {
        self.header = Some(header.into_any());
        self
    }
    /// Main drawer content.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }
    pub fn props(&self) -> &NavigationDrawerProps {
        &self.props
    }
    /// Expose the internal open signal so an external trigger can show/hide the drawer.
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
    pub fn resolved_style(&self) -> FluentNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        FluentNavigationDrawerStyle {
            design_system: DESIGN_SYSTEM,
            background: theme.colors().surface,
            scrim: theme.colors().scrim,
            width: self.props.width,
            elevation: self.props.elevation,
            padding: tokens.padding,
        }
    }

    /// Build a renderable Fluent navigation drawer: a panel over a dimmed
    /// scrim; clicking the scrim closes the drawer. Slides in from the chosen
    /// edge. Reactive to the open signal.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let side = self.props.side;
        let header = self.header;
        let content = self.child;

        let tokens = match theme.get().component(ComponentId::NavigationDrawer) {
            ComponentTokens::NavigationDrawer(t) => t,
            _ => unreachable!(),
        };
        let width = self.props.width;
        let elevation = self.props.elevation;
// layout-provider auto-register
        floom::reactive::create_effect(move |_prev: Option<()>| {
            let w = if open.get() { width } else { 0.0 };
            let slot = match side {
                NavigationDrawerSide::Left => floom_core_theme::layout::LayoutSlot::NavDrawerLeft,
                NavigationDrawerSide::Right => floom_core_theme::layout::LayoutSlot::NavDrawerRight,
            };
            floom_core_theme::layout::register_layout(slot, w);
        });

        let padding = tokens.padding;

        let mut panel_children: Vec<AnyView> = Vec::new();
        if let Some(h) = header {
            panel_children.push(h);
        }
        match content {
            Some(c) => panel_children.push(c),
            None => panel_children.push(Empty::new().into_any()),
        }

        let panel = Container::new(list(panel_children))
            .style(move |s| {
                let t = theme.get();
                let mut s = s
                    .width(width)
                    .height_pct(100.0)
                    .padding(padding)
                    .flex_col()
                    .items_start()
                    .background(t.colors().surface.to_peniko())
                    .border_left(3.0)
                    .border_color(t.colors().primary.to_peniko())
                    .apply_if(elevation > 0, |s| {
                        s.box_shadow_blur((elevation as f32) * 3.0)
                            .box_shadow_spread(1.0)
                            .box_shadow_color(with_alpha(BLACK, 30).to_peniko())
                    });
                match side {
                    NavigationDrawerSide::Left => {
                        s = s.inset_left(0.0);
                    }
                    NavigationDrawerSide::Right => {
                        s = s.inset_right(0.0).border_left(0.0).border_right(3.0);
                    }
                }
                s
            })
            // Clicks inside the panel must not dismiss the drawer.
            .on_event(listener::Click, |_, _| EventPropagation::Stop)
            // Slide in from the chosen edge, slide back out on close.
            .animation(move |a| {
                let from = match side {
                    NavigationDrawerSide::Left => -width,
                    NavigationDrawerSide::Right => width,
                };
                a.run_on_create(true)
                    .run_on_remove(true)
                    .duration(Duration::from_millis(260))
                    .keyframe(0, |f| f.style(|s| s.translate_x(from)))
                    .keyframe(100, |f| f.computed_style())
            });

        let scrim_area = Container::new(Empty::new())
            .style(|s| s.flex())
            .on_event(listener::Click, {
                let open = open;
                move |_, _| {
                    open.set(false);
                    EventPropagation::Continue
                }
            });

        let panel_any: AnyView = panel.into_any();
        let scrim_any: AnyView = scrim_area.into_any();
        let children: Vec<AnyView> = match side {
            NavigationDrawerSide::Left => vec![panel_any, scrim_any],
            NavigationDrawerSide::Right => vec![scrim_any, panel_any],
        };

        let overlay = Container::new(Stack::horizontal_from_iter(children)).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .height_pct(100.0)
                .flex_row()
                .background(with_alpha(t.colors().scrim, 160).to_peniko())
                .apply_if(!open.get(), |s| s.hide())
        })
        .animation(move |a| {
            a.run_on_create(true)
                .run_on_remove(true)
                .duration(Duration::from_millis(260))
                .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                .keyframe(100, |f| f.computed_style())
        });

        Overlay::new(overlay)
    }
}

impl Default for FluentNavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}
