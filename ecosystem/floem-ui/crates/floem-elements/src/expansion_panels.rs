pub use floem_core_theme::component_tokens::ExpansionPanelsTokens;
pub use floem_core_theme::props::expansion_panels::ExpansionPanelsProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ExpansionPanels` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_expansion_panels::CupertinoExpansionPanels as ExpansionPanels;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_expansion_panels::FluentExpansionPanels as ExpansionPanels;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_expansion_panels::MdExpansionPanels as ExpansionPanels;

/// Runtime-dispatch `ExpansionPanels` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::{AnyView, View};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::expansion_panels::ExpansionPanelsProps;

    use floem_widget_md_expansion_panels::MdExpansionPanels;
    use floem_widget_cupertino_expansion_panels::CupertinoExpansionPanels;
    use floem_widget_fluent_expansion_panels::FluentExpansionPanels;

    struct PanelItem {
        title: String,
        content: Box<dyn Fn() -> AnyView>,
    }

    pub struct ExpansionPanels {
        props: ExpansionPanelsProps,
        panels: Vec<PanelItem>,
    }

    impl ExpansionPanels {
        pub fn new() -> Self {
            Self {
                props: ExpansionPanelsProps::default(),
                panels: Vec::new(),
            }
        }

        pub fn panel(mut self, title: impl Into<String>, content: impl Fn() -> AnyView + 'static) -> Self {
            self.panels.push(PanelItem {
                title: title.into(),
                content: Box::new(content),
            });
            self
        }

        pub fn multiple(mut self, multiple: bool) -> Self {
            self.props.multiple = multiple;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize, bool) + 'static) -> Self {
            self.props.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            let panels = self.panels;
            match ds {
                DesignSystem::Material => {
                    let mut b = MdExpansionPanels::new();
                    for p in panels {
                        b = b.panel(p.title, p.content);
                    }
                    b = b.multiple(props.multiple);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let mut b = CupertinoExpansionPanels::new();
                    for p in panels {
                        b = b.panel(p.title, p.content);
                    }
                    b = b.multiple(props.multiple);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let mut b = FluentExpansionPanels::new();
                    for p in panels {
                        b = b.panel(p.title, p.content);
                    }
                    b = b.multiple(props.multiple);
                    if let Some(cb) = props.on_change {
                        b = b.on_change(cb);
                    }
                    b.view().into_any()
                }
            }
        }
    }

    impl Default for ExpansionPanels {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ExpansionPanels;
