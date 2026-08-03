pub use floem_core_theme::component_tokens::SwitchTokens;
pub use floem_core_theme::props::switch::SwitchProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_switch::CupertinoSwitch as Switch;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_switch::CupertinoSwitchStyle as SwitchStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_switch::FluentSwitch as Switch;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_switch::FluentSwitchStyle as SwitchStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_switch::MdSwitch as Switch;
#[cfg(feature = "material")]
pub use floem_widget_md_switch::MdSwitchStyle as SwitchStyle;
