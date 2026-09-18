pub use floem_core_theme::props::display_provider::DisplayProviderProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_display_provider::CupertinoDisplayProvider as DisplayProvider;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_display_provider::FluentDisplayProvider as DisplayProvider;
#[cfg(feature = "material")]
pub use floem_widget_md_display_provider::MdDisplayProvider as DisplayProvider;
