pub use floem_core_theme::component_tokens::ListTokens;
pub use floem_core_theme::props::list::ListProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_list::CupertinoList as List;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_list::CupertinoListStyle as ListStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_list::FluentList as List;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_list::FluentListStyle as ListStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_list::MdList as List;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_list::MdListStyle as ListStyle;

/// Runtime-dispatch `List` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::menu::MenuItem;
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_list::MdList;
    use floem_widget_cupertino_list::CupertinoList;
    use floem_widget_fluent_list::FluentList;

    pub struct List {
        items: Vec<MenuItem>,
        theme: Option<MultiTheme>,
    }

    impl List {
        pub fn new() -> Self {
            Self {
                items: Vec::new(),
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

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => {
                    let mut c = MdList::new().items(self.items);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut c = CupertinoList::new().items(self.items);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut c = FluentList::new().items(self.items);
                    if let Some(th) = self.theme {
                        c = c.with_theme(th);
                    }
                    c.view().into_any()
                }
            }
        }
    }

    impl Default for List {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::List;
#[cfg(feature = "multi")]
pub use floem_widget_md_list::MdListStyle as ListStyle;
