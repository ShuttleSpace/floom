pub use floom_core_theme::component_tokens::PaginationTokens;
pub use floom_core_theme::props::pagination::PaginationProps;

// Single-Design-System builds (and the default `material` build) keep the
// direct re-export. These are disabled in `multi` mode, where `Pagination` is the
// runtime-dispatch wrapper defined below.
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_pagination::CupertinoPagination as Pagination;
#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_pagination::CupertinoPaginationStyle as PaginationStyle;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_pagination::FluentPagination as Pagination;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_pagination::FluentPaginationStyle as PaginationStyle;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_pagination::MdPagination as Pagination;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_pagination::MdPaginationStyle as PaginationStyle;

/// Runtime-dispatch `Pagination` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::pagination::PaginationProps;

    use floom_widget_md_pagination::MdPagination;
    use floom_widget_cupertino_pagination::CupertinoPagination;
    use floom_widget_fluent_pagination::FluentPagination;

    pub struct Pagination {
        props: PaginationProps,
        on_change: Option<Box<dyn Fn(usize)>>,
    }

    impl Pagination {
        pub fn new() -> Self {
            Self {
                props: PaginationProps::default(),
                on_change: None,
            }
        }

        pub fn total(mut self, total: usize) -> Self {
            self.props.total = total;
            self
        }

        pub fn current(mut self, current: usize) -> Self {
            self.props.current = current;
            self
        }

        pub fn on_change(mut self, callback: impl Fn(usize) + 'static) -> Self {
            self.on_change = Some(Box::new(callback));
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            let on_change = self.on_change.unwrap_or_else(|| Box::new(|_| {}));
            match ds {
                DesignSystem::Material => MdPagination::new()
                    .total(self.props.total)
                    .current(self.props.current)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoPagination::new()
                    .total(self.props.total)
                    .current(self.props.current)
                    .on_change(on_change)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentPagination::new()
                    .total(self.props.total)
                    .current(self.props.current)
                    .on_change(on_change)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Pagination {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Pagination;
#[cfg(feature = "multi")]
pub use floom_widget_md_pagination::MdPaginationStyle as PaginationStyle;
