use floem::event::EventPropagation;
use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem::IntoView;
use floem::views::{Container, Decorators, Empty, Overlay, Stack, v_stack};
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::navigation_drawer::NavigationDrawerProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::color_ext::*;
use floem_widget_runtime::theme_signal;

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
        floem::reactive::create_effect(move |_prev: Option<()>| {
            let w = if open.get() { width } else { 0.0 };
            floem_core_theme::layout::register_layout(
                floem_core_theme::layout::LayoutSlot::NavDrawerLeft,
                w,
            );
        });

        let padding = tokens.padding;

        let header_view = header.unwrap_or_else(|| Empty::new().into_any());
        let content_view = content.unwrap_or_else(|| Empty::new().into_any());

        let panel = Container::new(
            v_stack((header_view, content_view))
                .style(move |s| s.width_full().height_full().gap(8.0).items_start()),
        )
        .style(move |s| {
            let t = theme.get();
            s.absolute()
                .inset_top(0.0)
                .inset_left(0.0)
                .height_pct(100.0)
                .width(width)
                .padding(padding)
                .background(t.colors().surface.to_peniko())
                .color(t.colors().on_surface.to_peniko())
                .box_shadow_blur(2.0 + 3.0 * (elevation as f64))
                .box_shadow_spread(1.0)
                .box_shadow_color(with_alpha(Color::rgba(0, 0, 0, 255), 40).to_peniko())
        })
        // Clicks inside the panel must not dismiss the drawer.
        .on_event(listener::Click, |_, _| EventPropagation::Stop);

        let scrim = Container::new(panel)
            .style(move |s| {
                let t = theme.get();
                s.width_pct(100.0)
                    .height_pct(100.0)
                    .background(with_alpha(t.colors().scrim, 150).to_peniko())
                    .apply_if(!open.get(), |s| s.hide())
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
