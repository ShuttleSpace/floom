pub use floem_core_theme::component_tokens::SwitchTokens;
pub use floem_core_theme::props::switch::SwitchProps;

#[cfg(feature = "material")]
pub use floem_widget_md_switch::MdSwitch as Switch;
#[cfg(feature = "material")]
pub use floem_widget_md_switch::MdSwitchStyle as SwitchStyle;
