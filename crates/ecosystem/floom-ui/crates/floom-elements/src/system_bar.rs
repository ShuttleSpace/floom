pub use floom_core_theme::component_tokens::SystemBarTokens;
pub use floom_core_theme::props::system_bar::SystemBarProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `SystemBar` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_system_bar::CupertinoSystemBar as SystemBar;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_system_bar::CupertinoSystemBarStyle as SystemBarStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_system_bar::FluentSystemBar as SystemBar;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_system_bar::FluentSystemBarStyle as SystemBarStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_system_bar::MdSystemBar as SystemBar;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_system_bar::MdSystemBarStyle as SystemBarStyle;

/// Runtime-dispatch `SystemBar` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::system_bar::SystemBarProps;

    use floom_widget_md_system_bar::MdSystemBar;
    use floom_widget_cupertino_system_bar::CupertinoSystemBar;
    use floom_widget_fluent_system_bar::FluentSystemBar;

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
            self.props.height = Some(height);
            self
        }

        pub fn dark(mut self, dark: bool) -> Self {
            self.props.dark = dark;
            self
        }

        pub fn pad(mut self, pad: f32) -> Self {
            self.props.pad = Some(pad);
            self
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            // `height`/`pad` are only forwarded when the caller set them
            // explicitly; otherwise each design system resolves its own
            // `SystemBarTokens` (M3 24/8, Cupertino 22/6, Fluent 28/8).
            let (height, dark, pad) = (self.props.height, self.props.dark, self.props.pad);
            match ds {
                DesignSystem::Material => {
                    let mut b = MdSystemBar::new().dark(dark);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoSystemBar::new().dark(dark);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentSystemBar::new().dark(dark);
                    if let Some(h) = height {
                        b = b.height(h);
                    }
                    if let Some(p) = pad {
                        b = b.pad(p);
                    }
                    b.child(child).view().into_any()
                }
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
pub use floom_widget_md_system_bar::MdSystemBarStyle as SystemBarStyle;
