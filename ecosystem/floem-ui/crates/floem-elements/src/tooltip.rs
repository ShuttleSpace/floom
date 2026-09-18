pub use floem_core_theme::component_tokens::TooltipTokens;
pub use floem_core_theme::props::tooltip::TooltipProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Tooltip` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_tooltip::CupertinoTooltip as Tooltip;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floem_widget_cupertino_tooltip::CupertinoTooltipStyle as TooltipStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_tooltip::FluentTooltip as Tooltip;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floem_widget_fluent_tooltip::FluentTooltipStyle as TooltipStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_tooltip::MdTooltip as Tooltip;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floem_widget_md_tooltip::MdTooltipStyle as TooltipStyle;

/// Runtime-dispatch `Tooltip` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floem::IntoView;
    use floem::reactive::SignalGet;
    use floem::view::AnyView;
    use floem::view::View;
    use floem::views::{empty, Empty};
    use floem_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floem_core_theme::props::tooltip::TooltipProps;
    use floem_core_theme::theme::MultiTheme;

    use floem_widget_md_tooltip::MdTooltip;
    use floem_widget_cupertino_tooltip::CupertinoTooltip;
    use floem_widget_fluent_tooltip::FluentTooltip;

    pub struct Tooltip {
        props: TooltipProps,
        child: Option<AnyView>,
        theme: Option<MultiTheme>,
    }

    impl Tooltip {
        pub fn new() -> Self {
            Self {
                props: TooltipProps::default(),
                child: None,
                theme: None,
            }
        }

        pub fn child(mut self, child: impl IntoView + 'static) -> Self {
            self.child = Some(child.into_any());
            self
        }

        pub fn text(mut self, text: impl Into<String>) -> Self {
            self.props.text = text.into();
            self
        }

        pub fn with_theme(mut self, theme: MultiTheme) -> Self {
            self.theme = Some(theme);
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let child = self.child.unwrap_or_else(|| Empty::new().into_any());
            match ds {
                DesignSystem::Material => {
                    let b = MdTooltip::new().child(child).text(self.props.text.clone());
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Cupertino => {
                    let b = CupertinoTooltip::new()
                        .child(child)
                        .text(self.props.text.clone());
                    let b = if let Some(t) = self.theme {
                        b.with_theme(t)
                    } else {
                        b
                    };
                    b.view().into_any()
                }
                DesignSystem::Fluent => {
                    let b = FluentTooltip::new().child(child).text(self.props.text.clone());
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

    impl Default for Tooltip {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Tooltip;
#[cfg(feature = "multi")]
pub use floem_widget_md_tooltip::MdTooltipStyle as TooltipStyle;
