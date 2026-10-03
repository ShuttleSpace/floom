pub use floom_core_theme::component_tokens::BadgeTokens;
pub use floom_core_theme::props::badge::BadgeProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_badge::CupertinoBadge as Badge;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_badge::CupertinoBadgeStyle as BadgeStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_badge::FluentBadge as Badge;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_badge::FluentBadgeStyle as BadgeStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_badge::MdBadge as Badge;
#[cfg(feature = "material")]
pub use floom_widget_md_badge::MdBadgeStyle as BadgeStyle;
