pub use floem_core_theme::props::icons_provider::IconsProviderProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_icons_provider::CupertinoIconsProvider as IconsProvider;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_icons_provider::FluentIconsProvider as IconsProvider;
#[cfg(feature = "material")]
pub use floem_widget_md_icons_provider::MdIconsProvider as IconsProvider;
