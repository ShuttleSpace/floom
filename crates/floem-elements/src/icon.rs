pub use floem_core_theme::component_tokens::IconTokens;
pub use floem_core_theme::props::icon::IconProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_icon::CupertinoIcon as Icon;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_icon::CupertinoIconStyle as IconStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_icon::FluentIcon as Icon;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_icon::FluentIconStyle as IconStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_icon::MdIcon as Icon;
#[cfg(feature = "material")]
pub use floem_widget_md_icon::MdIconStyle as IconStyle;
