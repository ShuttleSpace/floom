pub use floem_core_theme::component_tokens::{ComponentId, FloatingActionButtonTokens};
pub use floem_core_theme::props::floating_action_button::FloatingActionButtonProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_floating_action_button::CupertinoFloatingActionButton as FloatingActionButton;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_floating_action_button::FluentFloatingActionButton as FloatingActionButton;
#[cfg(feature = "material")]
pub use floem_widget_md_floating_action_button::MdFloatingActionButton as FloatingActionButton;
