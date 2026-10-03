pub use floem_core_theme::props::date_adapter_provider::DateAdapterProviderProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_date_adapter_provider::CupertinoDateAdapterProvider as DateAdapterProvider;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_date_adapter_provider::FluentDateAdapterProvider as DateAdapterProvider;
#[cfg(feature = "material")]
pub use floem_widget_md_date_adapter_provider::MdDateAdapterProvider as DateAdapterProvider;
