pub use floom_core_theme::component_tokens::FadeTransitionTokens;
pub use floom_core_theme::props::fade_transition::FadeTransitionProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_fade_transition::CupertinoFadeTransition as FadeTransition;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_fade_transition::CupertinoFadeTransitionStyle as FadeTransitionStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_fade_transition::FluentFadeTransition as FadeTransition;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_fade_transition::FluentFadeTransitionStyle as FadeTransitionStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_fade_transition::MdFadeTransition as FadeTransition;
#[cfg(feature = "material")]
pub use floom_widget_md_fade_transition::MdFadeTransitionStyle as FadeTransitionStyle;
