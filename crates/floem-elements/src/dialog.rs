pub use floem_core_theme::component_tokens::DialogTokens;
pub use floem_core_theme::props::dialog::DialogProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_dialog::CupertinoDialog as Dialog;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_dialog::CupertinoDialogStyle as DialogStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_dialog::FluentDialog as Dialog;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_dialog::FluentDialogStyle as DialogStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_dialog::MdDialog as Dialog;
#[cfg(feature = "material")]
pub use floem_widget_md_dialog::MdDialogStyle as DialogStyle;
