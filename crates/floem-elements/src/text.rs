pub use floem_core_theme::component_tokens::TextTokens;
pub use floem_core_theme::props::text::TextProps;

#[cfg(feature = "material")]
pub use floem_widget_md_text::MdText as Text;
#[cfg(feature = "material")]
pub use floem_widget_md_text::MdTextStyle as TextStyle;
