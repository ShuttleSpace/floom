pub use floom_core_theme::props::defaults_provider::DefaultsProviderProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_defaults_provider::CupertinoDefaultsProvider as DefaultsProvider;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_defaults_provider::FluentDefaultsProvider as DefaultsProvider;
#[cfg(feature = "material")]
pub use floom_widget_md_defaults_provider::MdDefaultsProvider as DefaultsProvider;
