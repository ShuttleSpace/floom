pub use floem_core_theme::props::defaults_provider::DefaultsProviderProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_defaults_provider::CupertinoDefaultsProvider as DefaultsProvider;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_defaults_provider::FluentDefaultsProvider as DefaultsProvider;
#[cfg(feature = "material")]
pub use floem_widget_md_defaults_provider::MdDefaultsProvider as DefaultsProvider;
