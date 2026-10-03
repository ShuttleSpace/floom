pub use floom_core_theme::component_tokens::MenuTokens;
pub use floom_core_theme::props::menu::{MenuItem, MenuProps};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_menu::CupertinoMenu as Menu;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_menu::CupertinoMenuStyle as MenuStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_menu::FluentMenu as Menu;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_menu::FluentMenuStyle as MenuStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_menu::MdMenu as Menu;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_menu::MdMenuStyle as MenuStyle;

/// Runtime-dispatch `Menu` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::{RwSignal, SignalGet, SignalUpdate};
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::menu::MenuItem;
    use floom_core_theme::theme::MultiTheme;

    use floom_widget_md_menu::MdMenu;
    use floom_widget_cupertino_menu::CupertinoMenu;
    use floom_widget_fluent_menu::FluentMenu;

    pub struct Menu {
        items: Vec<MenuItem>,
        open: RwSignal<bool>,
        theme: Option<MultiTheme>,
    }

    impl Menu {
        pub fn new() -> Self {
            Self {
                items: Vec::new(),
                open: RwSignal::new(false),
                theme: None,
            }
        }

        pub fn item(mut self, label: impl Into<String>, callback: impl Fn() + 'static) -> Self {
            self.items.push(MenuItem {
                label: label.into(),
                on_select: Some(Box::new(callback)),
            });
            self
        }

        pub fn items(mut self, items: Vec<MenuItem>) -> Self {
            self.items = items;
            self
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
        /// (built separately, e.g. on the same page) can show/hide the menu.
        pub fn with_open_signal(mut self, open: RwSignal<bool>) -> Self {
            self.open = open;
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdMenu::new().with_open_signal(self.open);
                    for it in self.items {
                        c = c.item(it.label, it.on_select.unwrap_or_else(|| Box::new(|| {})));
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoMenu::new().with_open_signal(self.open);
                    for it in self.items {
                        c = c.item(it.label, it.on_select.unwrap_or_else(|| Box::new(|| {})));
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentMenu::new().with_open_signal(self.open);
                    for it in self.items {
                        c = c.item(it.label, it.on_select.unwrap_or_else(|| Box::new(|| {})));
                    }
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for Menu {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Menu;
#[cfg(feature = "multi")]
pub use floom_widget_md_menu::MdMenuStyle as MenuStyle;
