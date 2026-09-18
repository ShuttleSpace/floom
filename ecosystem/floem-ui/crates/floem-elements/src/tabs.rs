pub use floem_core_theme::component_tokens::TabsTokens;
pub use floem_core_theme::props::tabs::TabsProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Tabs` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_tabs::CupertinoTabs as Tabs;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_tabs::CupertinoTabsStyle as TabsStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_tabs::FluentTabs as Tabs;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_tabs::FluentTabsStyle as TabsStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_tabs::MdTabs as Tabs;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_tabs::MdTabsStyle as TabsStyle;

/// Runtime-dispatch `Tabs` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::tabs::TabsProps;
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_tabs::MdTabs;
    use floem_widget_cupertino_tabs::CupertinoTabs;
    use floem_widget_fluent_tabs::FluentTabs;

    pub struct Tabs {
        props: TabsProps,
        on_change: Option<Box<dyn Fn(usize)>>,
        theme: Option<MultiTheme>,
    }

    impl Tabs {
        pub fn new() -> Self {
            Self {
                props: TabsProps::default(),
                on_change: None,
                theme: None,
            }
        }

        pub fn tab(mut self, tab: impl Into<String>) -> Self {
            self.props.tabs.push(tab.into());
            self
        }

        pub fn tabs(mut self, tabs: Vec<String>) -> Self {
            self.props.tabs = tabs;
            self
        }

        pub fn selected(mut self, selected: usize) -> Self {
            self.props.selected = selected;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => {
                    let b = MdTabs::new()
                        .tabs(self.props.tabs.clone())
                        .selected(self.props.selected)
                        .on_change(on_change);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoTabs::new()
                        .tabs(self.props.tabs.clone())
                        .selected(self.props.selected)
                        .on_change(on_change);
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentTabs::new()
                        .tabs(self.props.tabs.clone())
                        .selected(self.props.selected)
                        .on_change(on_change);
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

    impl Default for Tabs {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Tabs;
#[cfg(feature = "multi")]
pub use floem_widget_md_tabs::MdTabsStyle as TabsStyle;
