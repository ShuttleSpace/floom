pub use floom_core_theme::component_tokens::ListGroupTokens;
pub use floom_core_theme::props::list_group::ListGroupProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ListGroup` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_list_group::CupertinoListGroup as ListGroup;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_list_group::FluentListGroup as ListGroup;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_list_group::MdListGroup as ListGroup;

/// Runtime-dispatch `ListGroup` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::list_group::ListGroupProps;

    use floom_widget_md_list_group::MdListGroup;
    use floom_widget_cupertino_list_group::CupertinoListGroup;
    use floom_widget_fluent_list_group::FluentListGroup;

    pub struct ListGroup {
        props: ListGroupProps,
        items: Vec<String>,
    }

    impl ListGroup {
        pub fn new() -> Self {
            Self {
                props: ListGroupProps::default(),
                items: Vec::new(),
            }
        }

        pub fn item(mut self, label: impl Into<String>) -> Self {
            self.items.push(label.into());
            self
        }

        pub fn title(mut self, title: impl Into<String>) -> Self {
            self.props.title = Some(title.into());
            self
        }

        pub fn expanded(mut self, expanded: bool) -> Self {
            self.props.expanded = expanded;
            self
        }

        pub fn inset(mut self, inset: bool) -> Self {
            self.props.inset = inset;
            self
        }

        pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
            self.props.on_toggle = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            let items = self.items;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdListGroup::new();
                    for it in items {
                        b = b.item(it);
                    }
                    b = b.expanded(props.expanded).inset(props.inset);
                    if let Some(t) = props.title {
                        b = b.title(t);
                    }
                    if let Some(cb) = props.on_toggle {
                        b = b.on_toggle(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoListGroup::new();
                    for it in items {
                        b = b.item(it);
                    }
                    b = b.expanded(props.expanded).inset(props.inset);
                    if let Some(t) = props.title {
                        b = b.title(t);
                    }
                    if let Some(cb) = props.on_toggle {
                        b = b.on_toggle(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentListGroup::new();
                    for it in items {
                        b = b.item(it);
                    }
                    b = b.expanded(props.expanded).inset(props.inset);
                    if let Some(t) = props.title {
                        b = b.title(t);
                    }
                    if let Some(cb) = props.on_toggle {
                        b = b.on_toggle(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for ListGroup {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ListGroup;
