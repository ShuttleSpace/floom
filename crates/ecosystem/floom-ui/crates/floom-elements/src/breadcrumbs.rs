pub use floom_core_theme::component_tokens::BreadcrumbsTokens;
pub use floom_core_theme::props::breadcrumbs::BreadcrumbsProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_breadcrumbs::CupertinoBreadcrumbs as Breadcrumbs;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_breadcrumbs::CupertinoBreadcrumbsStyle as BreadcrumbsStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_breadcrumbs::FluentBreadcrumbs as Breadcrumbs;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_breadcrumbs::FluentBreadcrumbsStyle as BreadcrumbsStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_breadcrumbs::MdBreadcrumbs as Breadcrumbs;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_breadcrumbs::MdBreadcrumbsStyle as BreadcrumbsStyle;

/// Runtime-dispatch `Breadcrumbs` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};

    use floom_widget_md_breadcrumbs::MdBreadcrumbs;
    use floom_widget_cupertino_breadcrumbs::CupertinoBreadcrumbs;
    use floom_widget_fluent_breadcrumbs::FluentBreadcrumbs;

    pub struct Breadcrumbs {
        items: Vec<String>,
        on_click: Option<Box<dyn Fn(usize)>>,
    }

    impl Breadcrumbs {
        pub fn new() -> Self {
            Self {
                items: Vec::new(),
                on_click: None,
            }
        }

        pub fn items(mut self, items: Vec<String>) -> Self {
            self.items = items;
            self
        }

        pub fn on_click(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_click = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_click = self
                .on_click
                .unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdBreadcrumbs::new()
                    .items(self.items)
                    .on_click(on_click)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoBreadcrumbs::new()
                    .items(self.items)
                    .on_click(on_click)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentBreadcrumbs::new()
                    .items(self.items)
                    .on_click(on_click)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Breadcrumbs {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Breadcrumbs;
#[cfg(feature = "multi")]
pub use floom_widget_md_breadcrumbs::MdBreadcrumbsStyle as BreadcrumbsStyle;
