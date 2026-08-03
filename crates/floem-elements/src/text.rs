pub use floem_core_theme::component_tokens::TextTokens;
pub use floem_core_theme::props::text::TextProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_text::CupertinoText as Text;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_text::CupertinoTextStyle as TextStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_text::FluentText as Text;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_text::FluentTextStyle as TextStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_text::MdText as Text;
#[cfg(feature = "material")]
pub use floem_widget_md_text::MdTextStyle as TextStyle;
