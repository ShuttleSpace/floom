pub use floom_core_theme::props::locale_provider::LocaleProviderProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_locale_provider::CupertinoLocaleProvider as LocaleProvider;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_locale_provider::FluentLocaleProvider as LocaleProvider;
#[cfg(feature = "material")]
pub use floom_widget_md_locale_provider::MdLocaleProvider as LocaleProvider;
