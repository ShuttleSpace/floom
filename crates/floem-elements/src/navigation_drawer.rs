pub use floem_core_theme::component_tokens::NavigationDrawerTokens;
pub use floem_core_theme::props::navigation_drawer::NavigationDrawerProps;

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
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::empty;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floem_widget_md_navigation_drawer::MdNavigationDrawer;
    use floem_widget_cupertino_navigation_drawer::CupertinoNavigationDrawer;
    use floem_widget_fluent_navigation_drawer::FluentNavigationDrawer;

    pub struct NavigationDrawer {
        open: bool,
        width: f32,
        elevation: u8,
        header: Option<AnyView>,
        child: Option<AnyView>,
    }

    impl NavigationDrawer {
        pub fn new() -> Self {
            Self {
                open: false,
                width: 304.0,
                elevation: 4,
                header: None,
                child: None,
            }
        }

        pub fn open(mut self, open: bool) -> Self {
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

        pub fn header(mut self, header: impl IntoView + 'static) -> Self {
            self.header = Some(header.into_any());
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let header = self.header.unwrap_or_else(|| empty().into_any());
            let child = self.child.unwrap_or_else(|| empty().into_any());
            match ds {
                DesignSystem::Material => MdNavigationDrawer::new()
                    .open(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .header(header)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoNavigationDrawer::new()
                    .open(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .header(header)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentNavigationDrawer::new()
                    .open(self.open)
                    .width(self.width)
                    .elevation(self.elevation)
                    .header(header)
                    .child(child)
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
