pub use floem_core_theme::component_tokens::{ComponentId, CodeTokens};
pub use floem_core_theme::props::code::CodeProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_code::CupertinoCode as Code;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_code::FluentCode as Code;
#[cfg(feature = "material")]
pub use floem_widget_md_code::MdCode as Code;
