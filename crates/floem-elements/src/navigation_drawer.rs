pub use floem_core_theme::component_tokens::NavigationDrawerTokens;
pub use floem_core_theme::props::navigation_drawer::{NavigationDrawerProps, NavigationDrawerSide};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_navigation_drawer::CupertinoNavigationDrawer as NavigationDrawer;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_navigation_drawer::CupertinoNavigationDrawerStyle as NavigationDrawerStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_navigation_drawer::FluentNavigationDrawer as NavigationDrawer;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_navigation_drawer::FluentNavigationDrawerStyle as NavigationDrawerStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_navigation_drawer::MdNavigationDrawer as NavigationDrawer;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_navigation_drawer::MdNavigationDrawerStyle as NavigationDrawerStyle;

/// Runtime-dispatch `NavigationDrawer` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use std::rc::Rc;
    use floem::IntoView;
    use floem::reactive::{RwSignal, SignalGet, SignalUpdate};
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::{empty, Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_navigation_drawer::MdNavigationDrawer;
    use floem_widget_cupertino_navigation_drawer::CupertinoNavigationDrawer;
    use floem_widget_fluent_navigation_drawer::FluentNavigationDrawer;
    use floem_core_theme::props::navigation_drawer::NavigationDrawerSide;

    pub struct NavigationDrawer {
        open: RwSignal<bool>,
        width: f32,
        elevation: u8,
        side: NavigationDrawerSide,
        header: Option<Rc<dyn Fn() -> AnyView>>,
        child: Option<Rc<dyn Fn() -> AnyView>>,
    }

    impl NavigationDrawer {
        pub fn new() -> Self {
            Self {
                open: RwSignal::new(false),
                width: 304.0,
                elevation: 4,
                side: NavigationDrawerSide::Left,
                header: None,
                child: None,
            }
        }

        pub fn open(mut self, open: bool) -> Self {
            self.open.set(open);
            self
        }

        /// Expose the internal open signal so an external trigger can show/hide.
        pub fn open_signal(&self) -> RwSignal<bool> {
            self.open
        }

        /// Replace the internal open signal with an external one so a trigger
        /// (built separately, e.g. on the same page) can show/hide the drawer.
        pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
            self.open = open;
            self
        }

        pub fn width(mut self, width: f32) -> Self {
            self.width = width;
            self
        }

        pub fn elevation(mut self, elevation: u8) -> Self {
            self.elevation = elevation;
            self
        }

        /// Choose the edge the drawer slides in from (`Left` or `Right`).
        pub fn side(mut self, side: NavigationDrawerSide) -> Self {
            self.side = side;
            self
        }

        /// Accept a header factory so the content can be rebuilt for each
        /// concrete DS in the `view()` match below.
        pub fn header<F, IV>(mut self, header: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.header = Some(Rc::new(move || header().into_any()));
            self
        }

        /// Accept a child factory so the content can be rebuilt for each
        /// concrete DS in the `view()` match below.
        pub fn child<F, IV>(mut self, child: F) -> Self
        where
            F: Fn() -> IV + 'static,
            IV: IntoView + 'static,
        {
            self.child = Some(Rc::new(move || child().into_any()));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let header = self.header;
            let child = self.child;
            let header_factory = move || {
                header
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            let child_factory = move || {
                child
                    .clone()
                    .map(|f| f())
                    .unwrap_or_else(|| Empty::new().into_any())
            };
            match ds {
                DesignSystem::Material => MdNavigationDrawer::new()
                    .with_open_signal(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .side(self.side)
                    .header(header_factory())
                    .child(child_factory())
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoNavigationDrawer::new()
                    .with_open_signal(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .side(self.side)
                    .header(header_factory())
                    .child(child_factory())
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentNavigationDrawer::new()
                    .with_open_signal(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .side(self.side)
                    .header(header_factory())
                    .child(child_factory())
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for NavigationDrawer {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::NavigationDrawer;
#[cfg(feature = "multi")]
pub use floem_widget_md_navigation_drawer::MdNavigationDrawerStyle as NavigationDrawerStyle;
