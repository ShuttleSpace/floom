pub use floom_core_theme::component_tokens::ToolbarTokens;
pub use floom_core_theme::props::toolbar::ToolbarProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Toolbar` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_toolbar::CupertinoToolbar as Toolbar;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_toolbar::CupertinoToolbarStyle as ToolbarStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_toolbar::FluentToolbar as Toolbar;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_toolbar::FluentToolbarStyle as ToolbarStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_toolbar::MdToolbar as Toolbar;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_toolbar::MdToolbarStyle as ToolbarStyle;

/// Runtime-dispatch `Toolbar` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::AnyView;
    use floom::view::View;
    use floom::views::{empty, Empty};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::toolbar::ToolbarProps;
    use floom_core_theme::theme::MultiTheme;

    use floom_widget_md_toolbar::MdToolbar;
    use floom_widget_cupertino_toolbar::CupertinoToolbar;
    use floom_widget_fluent_toolbar::FluentToolbar;

    pub struct Toolbar {
        props: ToolbarProps,
        actions: Option<AnyView>,
        theme: Option<MultiTheme>,
    }

    impl Toolbar {
        pub fn new() -> Self {
            Self {
                props: ToolbarProps::default(),
                actions: None,
                theme: None,
            }
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.props.title = title.into();
            self
        }

        pub fn leading(mut self, icon: impl Into<String>) -> Self {
            self.props.leading = Some(icon.into());
            self
        }

        pub fn elevation(mut self, elevation: u8) -> Self {
            self.props.elevation = elevation;
            self
        }

        pub fn height(mut self, height: f32) -> Self {
            self.props.height = height;
            self
        }

        pub fn actions(mut self, child: impl IntoView + 'static) -> Self {
            self.actions = Some(child.into_any());
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let actions = self.actions.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => {
                    let b = MdToolbar::new()
                        .title(self.props.title.clone())
                        .leading(self.props.leading.clone().unwrap_or_default())
                        .elevation(self.props.elevation)
                        .height(self.props.height)
                        .actions(actions);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoToolbar::new()
                        .title(self.props.title.clone())
                        .leading(self.props.leading.clone().unwrap_or_default())
                        .elevation(self.props.elevation)
                        .height(self.props.height)
                        .actions(actions);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentToolbar::new()
                        .title(self.props.title.clone())
                        .leading(self.props.leading.clone().unwrap_or_default())
                        .elevation(self.props.elevation)
                        .height(self.props.height)
                        .actions(actions);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for Toolbar {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Toolbar;
#[cfg(feature = "multi")]
pub use floom_widget_md_toolbar::MdToolbarStyle as ToolbarStyle;
