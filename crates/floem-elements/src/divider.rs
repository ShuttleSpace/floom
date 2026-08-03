pub use floem_core_theme::component_tokens::DividerTokens;
pub use floem_core_theme::props::divider::{DividerOrientation, DividerProps};

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_divider::CupertinoDivider as Divider;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_divider::CupertinoDividerStyle as DividerStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_divider::FluentDivider as Divider;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_divider::FluentDividerStyle as DividerStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_divider::MdDivider as Divider;
#[cfg(feature = "material")]
pub use floem_widget_md_divider::MdDividerStyle as DividerStyle;
