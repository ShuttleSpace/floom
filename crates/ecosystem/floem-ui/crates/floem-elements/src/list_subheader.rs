pub use floem_core_theme::component_tokens::ListSubheaderTokens;
pub use floem_core_theme::props::list_subheader::ListSubheaderProps;

// Single-Design-System builds keep the direct re-export. Disabled in `multi`
// mode, where `ListSubheader` is the runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_list_subheader::CupertinoListSubheader as ListSubheader;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_list_subheader::FluentListSubheader as ListSubheader;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_list_subheader::MdListSubheader as ListSubheader;

/// Runtime-dispatch `ListSubheader` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::View;
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::list_subheader::ListSubheaderProps;

    use floem_widget_md_list_subheader::MdListSubheader;
    use floem_widget_cupertino_list_subheader::CupertinoListSubheader;
    use floem_widget_fluent_list_subheader::FluentListSubheader;

    pub struct ListSubheader {
        props: ListSubheaderProps,
    }

    impl ListSubheader {
        pub fn new(text_content: impl Into<String>) -> Self {
            Self {
                props: ListSubheaderProps {
                    text: text_content.into(),
                    inset: false,
                },
            }
        }

        pub fn inset(mut self, inset: bool) -> Self {
            self.props.inset = inset;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let props = self.props;
            match ds {
                DesignSystem::Material => {
                    MdListSubheader::new(props.text).inset(props.inset).view().into_any()
                }
                DesignSystem::Cupertino => {
                    CupertinoListSubheader::new(props.text)
                        .inset(props.inset)
                        .view()
                        .into_any()
                }
                DesignSystem::Fluent => {
                    FluentListSubheader::new(props.text)
                        .inset(props.inset)
                        .view()
                        .into_any()
                }
            }
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::ListSubheader;
