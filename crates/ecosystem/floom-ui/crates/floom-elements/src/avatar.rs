pub use floom_core_theme::component_tokens::AvatarTokens;
pub use floom_core_theme::props::avatar::AvatarProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_avatar::CupertinoAvatar as Avatar;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_avatar::CupertinoAvatarStyle as AvatarStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_avatar::FluentAvatar as Avatar;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_avatar::FluentAvatarStyle as AvatarStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_avatar::MdAvatar as Avatar;
#[cfg(feature = "material")]
pub use floom_widget_md_avatar::MdAvatarStyle as AvatarStyle;
