pub use floem_core_theme::component_tokens::ParallaxTokens;
pub use floem_core_theme::props::parallax::ParallaxProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_parallax::CupertinoParallax as Parallax;
#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_parallax::CupertinoParallaxStyle as ParallaxStyle;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_parallax::FluentParallax as Parallax;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_parallax::FluentParallaxStyle as ParallaxStyle;
#[cfg(feature = "material")]
pub use floem_widget_md_parallax::MdParallax as Parallax;
#[cfg(feature = "material")]
pub use floem_widget_md_parallax::MdParallaxStyle as ParallaxStyle;
