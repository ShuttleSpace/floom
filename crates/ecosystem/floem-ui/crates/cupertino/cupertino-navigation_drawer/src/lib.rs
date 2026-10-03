use std::time::Duration;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem::IntoView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoNavigationDrawerStyle {
    pub design_system: &'static str,
    pub width: f32,
    pub elevation: u8,
    pub padding: f32,
    pub background: Color,
    pub header_color: Color,
    pub content_color: Color,
    pub scrim_color: Color,
}

pub struct CupertinoNavigationDrawer {
    props: NavigationDrawerProps,
    theme: RwSignal<MultiTheme>,
    open: RwSignal<bool>,
    header: Option<AnyView>,
    content: Option<AnyView>,
}

impl CupertinoNavigationDrawer {
    pub fn new() -> Self {
        let props = NavigationDrawerProps::default();
        Self {
            open: RwSignal::new(props.open),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            props,
            header: None,
            content: None,
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
    /// Optional header view rendered at the top of the drawer.
    pub fn header(mut self, header: impl IntoView + 'static) -> Self {
        self.header = Some(header.into_any());
        self
    }
    /// The main content view of the drawer.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.content = Some(child.into_any());
        self
    }
    pub fn props(&self) -> &NavigationDrawerProps {
        &self.props
    }
    /// Expose the internal open signal so an external trigger (e.g. a menu
    /// button) can show or hide the drawer.
    pub fn open_signal(&self) -> RwSignal<bool> {
        self.open
    }

    /// Set the internal `open` signal from an external signal.
    pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
        self.open = open;
        self
    }
    pub fn is_open(&self) -> bool {
        self.open.get()
    }
    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }
    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }
    pub fn resolved_style(&self) -> CupertinoNavigationDrawerStyle {
        let theme = self.theme.get();
        let ComponentTokens::NavigationDrawer(tokens) =
            theme.component(ComponentId::NavigationDrawer)
        else {
            unreachable!("ComponentId::NavigationDrawer must resolve to NavigationDrawerTokens")
        };
        CupertinoNavigationDrawerStyle {
            design_system: DESIGN_SYSTEM,
            width: self.props.width.max(0.0),
            elevation: self.props.elevation,
            padding: tokens.padding,
            background: theme.colors().surface,
            header_color: theme.colors().on_surface,
            content_color: theme.colors().on_surface_variant,
            scrim_color: with_alpha(theme.colors().scrim, 150),
        }
    }

    /// Build a renderable Cupertino navigation drawer over a dimmed scrim.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let open = self.open;
        let side = self.props.side;
        let header = self.header.unwrap_or_else(|| Empty::new().into_any());
        let content = self.content.unwrap_or_else(|| Empty::new().into_any());

        let tokens = match theme.get().component(ComponentId::NavigationDrawer) {
            ComponentTokens::NavigationDrawer(t) => t,
            _ => unreachable!(),
        };
        let drawer_width = self.props.width.max(0.0);
        let padding = tokens.padding;
        let elevation = self.props.elevation;
// layout-provider auto-register
        floem::reactive::create_effect(move |_prev: Option<()>| {
            let w = if open.get() { drawer_width } else { 0.0 };
            let slot = match side {
                NavigationDrawerSide::Left => floem_core_theme::layout::LayoutSlot::NavDrawerLeft,
                NavigationDrawerSide::Right => floem_core_theme::layout::LayoutSlot::NavDrawerRight,
            };
            floem_core_theme::layout::register_layout(slot, w);
        });


        let panel = Container::new(
            Stack::vertical((header, content))
                .style(move |s| s.gap(8.0).items_start().flex_grow(1.0)),
        )
        .style(move |s| {
            let t = theme.get();
            let elevation_blur = (elevation as f32) * 4.0;
            let mut s = s
                .absolute()
                .inset_top(0.0)
                .inset_bottom(0.0)
                .width(drawer_width)
                .padding(padding)
                .background(with_alpha(t.colors().surface, 235).to_peniko())
                // translucent blurred-like surface
                .border_right(0.5)
                .border_color(with_alpha(t.colors().on_surface, 30).to_peniko())
                .apply_if(elevation > 0, |s| {
                    s.box_shadow_blur(elevation_blur)
                        .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 50).to_peniko())
                })
                .items_start()
                .apply_if(!open.get(), |s| s.hide());
            match side {
                NavigationDrawerSide::Left => {
                    s = s.inset_left(0.0);
                }
                NavigationDrawerSide::Right => {
                    s = s.inset_right(0.0);
                    s = s.border_right(0.0).border_left(0.5);
                }
            }
            s
        })
        // Slide in from the chosen edge, slide back out on close.
        .animation(move |a| {
            let from = match side {
                NavigationDrawerSide::Left => -drawer_width,
                NavigationDrawerSide::Right => drawer_width,
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

impl Default for CupertinoNavigationDrawer {
    fn default() -> Self {
        Self::new()
    }
}
