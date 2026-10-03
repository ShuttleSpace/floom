pub use floom_core_theme::props::theme_provider::ThemeProviderProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_theme_provider::CupertinoThemeProvider as ThemeProvider;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_theme_provider::FluentThemeProvider as ThemeProvider;
#[cfg(feature = "material")]
pub use floom_widget_md_theme_provider::MdThemeProvider as ThemeProvider;
