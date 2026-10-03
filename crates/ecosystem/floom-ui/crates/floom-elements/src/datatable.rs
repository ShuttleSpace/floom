pub use floom_core_theme::component_tokens::DataTableTokens;
pub use floom_core_theme::props::datatable::{ColumnAlign, DataTableColumn, DataTableProps};

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_datatable::CupertinoDataTable as DataTable;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_datatable::CupertinoDataTableStyle as DataTableStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_datatable::FluentDataTable as DataTable;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_datatable::FluentDataTableStyle as DataTableStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_datatable::MdDataTable as DataTable;
#[cfg(feature = "material")]
pub use floom_widget_md_datatable::MdDataTableStyle as DataTableStyle;
