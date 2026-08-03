pub use floem_core_theme::component_tokens::{ButtonTokens, ComponentId};
pub use floem_core_theme::props::button::{ButtonProps, ButtonSize, ButtonVariant};

#[cfg(feature = "material")]
pub use floem_widget_md_button::MdButton as Button;
#[cfg(feature = "material")]
pub use floem_widget_md_button::MdButtonStyle as ButtonStyle;

#[cfg(not(feature = "material"))]
compile_error!("At least one Design System feature must be enabled");
