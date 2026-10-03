pub use floom_core_theme::component_tokens::{ComponentId, TableTokens};
pub use floom_core_theme::props::table::TableProps;

#[cfg(all(feature = "cupertino", not(feature = "multi")))]
pub use floom_widget_cupertino_table::CupertinoTable as Table;
#[cfg(all(feature = "fluent", not(feature = "multi")))]
pub use floom_widget_fluent_table::FluentTable as Table;
#[cfg(all(feature = "material", not(feature = "multi")))]
pub use floom_widget_md_table::MdTable as Table;

/// Runtime-dispatch `Table` for the `multi` feature.
#[cfg(feature = "multi")]
mod dispatch {
    use floom::IntoView;
    use floom::reactive::SignalGet;
    use floom::view::View;
    use floom_core_theme::global_theme::{design_system_signal, DesignSystem};
    use floom_core_theme::props::table::TableProps;

    use floom_widget_md_table::MdTable;
    use floom_widget_cupertino_table::CupertinoTable;
    use floom_widget_fluent_table::FluentTable;

    pub struct Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        dense: bool,
        striped: bool,
    }

    impl Table {
        pub fn new() -> Self {
            let props = TableProps::default();
            Self {
                headers: props.headers,
                rows: props.rows,
                dense: props.dense,
                striped: props.striped,
            }
        }

        pub fn headers(mut self, headers: Vec<String>) -> Self {
            self.headers = headers;
            self
        }

        pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
            self.rows = rows;
            self
        }

        pub fn dense(mut self, dense: bool) -> Self {
            self.dense = dense;
            self
        }

        pub fn striped(mut self, striped: bool) -> Self {
            self.striped = striped;
            self
        }

        pub fn view(self) -> impl View {
            let ds = design_system_signal().get();
            match ds {
                DesignSystem::Material => MdTable::new()
                    .headers(self.headers)
                    .rows(self.rows)
                    .dense(self.dense)
                    .striped(self.striped)
                    .view()
                    .into_any(),
                DesignSystem::Cupertino => CupertinoTable::new()
                    .headers(self.headers)
                    .rows(self.rows)
                    .dense(self.dense)
                    .striped(self.striped)
                    .view()
                    .into_any(),
                DesignSystem::Fluent => FluentTable::new()
                    .headers(self.headers)
                    .rows(self.rows)
                    .dense(self.dense)
                    .striped(self.striped)
                    .view()
                    .into_any(),
            }
        }
    }

    impl Default for Table {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(feature = "multi")]
pub use dispatch::Table;
