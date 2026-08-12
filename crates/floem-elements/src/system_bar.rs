pub use floem_core_theme::component_tokens::SystemBarTokens;
pub use floem_core_theme::props::system_bar::SystemBarProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `SystemBar` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_system_bar::CupertinoSystemBar as SystemBar;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_system_bar::CupertinoSystemBarStyle as SystemBarStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_system_bar::FluentSystemBar as SystemBar;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_system_bar::FluentSystemBarStyle as SystemBarStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_system_bar::MdSystemBar as SystemBar;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_system_bar::MdSystemBarStyle as SystemBarStyle;

/// Runtime-dispatch `SystemBar` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::empty;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::system_bar::SystemBarProps;

    use floem_widget_md_system_bar::MdSystemBar;
    use floem_widget_cupertino_system_bar::CupertinoSystemBar;
    use floem_widget_fluent_system_bar::FluentSystemBar;

    pub struct SystemBar {
        props: SystemBarProps,
        child: Option<AnyView>,
    }

    impl SystemBar {
        pub fn new() -> Self {
            Self {
                props: SystemBarProps::default(),
                child: None,
            }
        }

        pub fn height(mut self, height: f32) -> Self {
            self.props.height = height;
            self
        }

        pub fn dark(mut self, dark: bool) -> Self {
            self.props.dark = dark;
            self
        }

        pub fn pad(mut self, pad: f32) -> Self {
            self.props.pad = pad;
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| empty().into_any());
            match ds {
                DesignSystem::Material => MdSystemBar::new()
                    .height(self.props.height)
                    .dark(self.props.dark)
                    .pad(self.props.pad)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoSystemBar::new()
                    .height(self.props.height)
                    .dark(self.props.dark)
                    .pad(self.props.pad)
                    .child(child)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentSystemBar::new()
                    .height(self.props.height)
                    .dark(self.props.dark)
                    .pad(self.props.pad)
                    .child(child)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for SystemBar {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::SystemBar;
#[cfg(feature = "multi")]
pub use floem_widget_md_system_bar::MdSystemBarStyle as SystemBarStyle;
