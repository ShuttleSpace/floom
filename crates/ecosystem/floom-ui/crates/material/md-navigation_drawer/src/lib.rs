use std::time::Duration;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom::IntoView;
use floom::views::{Container, Decorators, Empty, Overlay, Stack, v_stack};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::*;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdNavigationDrawerStyle {
    pub design_system: &'static str,
    pub panel_color: Color,
    pub content_color: Color,
    pub scrim_color: Color,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
}

pub struct MdNavigationDrawer {
    props: NavigationDrawerProps,
    open: RwSignal<bool>,
    header: Option<AnyView>,
    content: Option<AnyView>,
    theme: RwSignal<MultiTheme>,
}

impl MdNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: RwSignal::new(props.open),
            header: None,
            content: None,
            theme: theme_signal(),
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

    /// Optional header widget shown at the top of the drawer.
    pub fn header(mut self, header: impl IntoView + 'static) -> Self {
        self.header = Some(header.into_any());
        self
    }

    /// Main scrollable content of the drawer.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.content = Some(child.into_any());
        self
    }

    /// Expose the internal open signal so an external trigger can open/close the drawer.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }

    pub fn resolved_style(&self) -> MdNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        MdNavigationDrawerStyle {
            design_system: "material",
            panel_color: theme.colors().surface,
            content_color: theme.colors().on_surface,
            scrim_color: theme.colors().scrim,
            width: if self.props.width > 0.0 {
                self.props.width
            } else {
                tokens.width
            },
            elevation: self.props.elevation,
            padding: tokens.padding,
        }
    }

    /// Build a renderable Material 3 navigation drawer shown over a click-to-close scrim.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let side = self.props.side;
        let header = self.header;
        let content = self.content;

        let tokens = match theme.get().component(ComponentId::NavigationDrawer) {
            ComponentTokens::NavigationDrawer(t) => t,
            _ => unreachable!(),
        };
        let width = if self.props.width > 0.0 {
            self.props.width
        } else {
            tokens.width
        };
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

        let header_view = header.unwrap_or_else(|| Empty::new().into_any());
        let content_view = content.unwrap_or_else(|| Empty::new().into_any());

        let panel = Container::new(
            Stack::vertical((header_view, content_view))
                .style(move |s| s.width_full().height_full().gap(8.0).items_start()),
        )
        .style(move |s| {
            let t = theme.get();
            let mut s = s
                .absolute()
                .inset_top(0.0)
                .height_pct(100.0)
                .width(width)
                .padding(padding)
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .box_shadow_blur(2.0 + 3.0 * (elevation as f64))
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 40).to_peniko());
            match side {
                NavigationDrawerSide::Left => {
                    s = s.inset_left(0.0);
                }
                NavigationDrawerSide::Right => {
                    s = s.inset_right(0.0);
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

        let scrim = Container::new(panel)
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .height_pct(100.0)
                    .background(with_alpha(t.colors().scrim, 150).to_peniko())
                    .apply_if(!open.get(), |s| s.hide())
            })
            .animation(move |a| {
                a.run_on_create(true)
                    .run_on_remove(true)
                    .duration(Duration::from_millis(260))
                    .keyframe(0, |f| f.style(|s| s.opacity(0.0)))
                    .keyframe(100, |f| f.computed_style())
            })
            .on_event(listener::Click, {
                let open = open;
                move |_, _| {
                    open.set(false);
                    EventPropagation::Continue
                }
            });

        Overlay::new(scrim)
    }
}

impl Default for MdNavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}
