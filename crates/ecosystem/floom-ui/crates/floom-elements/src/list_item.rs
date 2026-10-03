pub use floom_core_theme::component_tokens::{ComponentId, ListItemTokens};
pub use floom_core_theme::props::list_item::ListItemProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_list_item::CupertinoListItem as ListItem;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_list_item::FluentListItem as ListItem;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_list_item::MdListItem as ListItem;

/// Runtime-dispatch `ListItem` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::{AnyView, View};
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::list_item::ListItemProps;

    use floom_widget_md_list_item::MdListItem;
    use floom_widget_cupertino_list_item::CupertinoListItem;
    use floom_widget_fluent_list_item::FluentListItem;

    pub struct ListItem {
        title: String,
        subtitle: Option<String>,
        leading: Option<AnyView>,
        trailing: Option<AnyView>,
        disabled: bool,
        nav: bool,
        on_click: Option<Box<dyn Fn()>>,
    }

    impl ListItem {
        pub fn new() -> Self {
            let props = ListItemProps::default();
            Self {
                title: props.title,
                subtitle: props.subtitle,
                leading: None,
                trailing: None,
                disabled: props.disabled,
                nav: props.nav,
                on_click: None,
            }
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.title = title.into();
            self
        }

        pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
            self.subtitle = Some(subtitle.into());
            self
        }

        pub fn leading(mut self, leading: impl IntoView + 'static) -> Self {
            self.leading = Some(leading.into_any());
            self
        }

        pub fn trailing(mut self, trailing: impl IntoView + 'static) -> Self {
            self.trailing = Some(trailing.into_any());
            self
        }

        pub fn disabled(mut self, disabled: bool) -> Self {
            self.disabled = disabled;
            self
        }

        pub fn nav(mut self, nav: bool) -> Self {
            self.nav = nav;
            self
        }

        pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
            self.on_click = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_click = self
                .on_click
                .unwrap_or_else(|| Box::new(|| {}));
            match ds {
                DesignSystem::Material => {
                    let mut b = MdListItem::new()
                        .title(self.title)
                        .disabled(self.disabled)
                        .nav(self.nav)
                        .on_click(on_click);
                    if let Some(s) = self.subtitle {
                        b = b.subtitle(s);
                    }
                    if let Some(l) = self.leading {
                        b = b.leading(l);
                    }
                    if let Some(t) = self.trailing {
                        b = b.trailing(t);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoListItem::new()
                        .title(self.title)
                        .disabled(self.disabled)
                        .nav(self.nav)
                        .on_click(on_click);
                    if let Some(s) = self.subtitle {
                        b = b.subtitle(s);
                    }
                    if let Some(l) = self.leading {
                        b = b.leading(l);
                    }
                    if let Some(t) = self.trailing {
                        b = b.trailing(t);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentListItem::new()
                        .title(self.title)
                        .disabled(self.disabled)
                        .nav(self.nav)
                        .on_click(on_click);
                    if let Some(s) = self.subtitle {
                        b = b.subtitle(s);
                    }
                    if let Some(l) = self.leading {
                        b = b.leading(l);
                    }
                    if let Some(t) = self.trailing {
                        b = b.trailing(t);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for ListItem {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ListItem;
