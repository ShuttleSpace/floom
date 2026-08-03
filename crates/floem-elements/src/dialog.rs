pub use floem_core_theme::component_tokens::DialogTokens;
pub use floem_core_theme::props::dialog::DialogProps;

#[cfg(feature = "material")]
pub use floem_widget_md_dialog::MdDialog as Dialog;
#[cfg(feature = "material")]
pub use floem_widget_md_dialog::MdDialogStyle as DialogStyle;
