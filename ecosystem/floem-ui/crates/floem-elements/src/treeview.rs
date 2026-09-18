pub use floem_core_theme::component_tokens::TreeviewTokens;
pub use floem_core_theme::props::treeview::{TreeviewNode, TreeviewProps};

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_treeview::CupertinoTreeview as Treeview;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_treeview::CupertinoTreeviewStyle as TreeviewStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_treeview::FluentTreeview as Treeview;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_treeview::FluentTreeviewStyle as TreeviewStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_treeview::MdTreeview as Treeview;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_treeview::MdTreeviewStyle as TreeviewStyle;

/// Runtime-dispatch `Treeview` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::treeview::TreeviewNode;

    use floem_widget_md_treeview::MdTreeview;
    use floem_widget_cupertino_treeview::CupertinoTreeview;
    use floem_widget_fluent_treeview::FluentTreeview;

    pub struct Treeview {
        nodes: Vec<TreeviewNode>,
        on_node_click: Option<Box<dyn Fn(String)>>,
    }

    impl Treeview {
        pub fn new() -> Self {
            Self {
                nodes: Vec::new(),
                on_node_click: None,
            }
        }

        pub fn nodes(mut self, nodes: Vec<TreeviewNode>) -> Self {
            self.nodes = nodes;
            self
        }

        pub fn on_node_click(mut self, callback: impl Fn(String) + 'static) -> Self {
            self.on_node_click = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_node_click = self
                .on_node_click
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdTreeview::new()
                    .nodes(self.nodes)
                    .on_node_click(on_node_click)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoTreeview::new()
                    .nodes(self.nodes)
                    .on_node_click(on_node_click)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentTreeview::new()
                    .nodes(self.nodes)
                    .on_node_click(on_node_click)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Treeview {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Treeview;
#[cfg(feature = "multi")]
pub use floem_widget_md_treeview::MdTreeviewStyle as TreeviewStyle;
