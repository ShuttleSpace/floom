pub use floom_core_theme::props::icons_provider::IconsProviderProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_icons_provider::CupertinoIconsProvider as IconsProvider;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_icons_provider::FluentIconsProvider as IconsProvider;
#[cfg(feature = "material")]
pub use floom_widget_md_icons_provider::MdIconsProvider as IconsProvider;
