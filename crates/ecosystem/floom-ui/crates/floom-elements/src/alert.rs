pub use floom_core_theme::component_tokens::AlertTokens;
pub use floom_core_theme::props::alert::{AlertProps, AlertVariant};

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_alert::CupertinoAlert as Alert;
#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_alert::CupertinoAlertStyle as AlertStyle;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_alert::FluentAlert as Alert;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_alert::FluentAlertStyle as AlertStyle;
#[cfg(feature = "material")]
pub use floom_widget_md_alert::MdAlert as Alert;
#[cfg(feature = "material")]
pub use floom_widget_md_alert::MdAlertStyle as AlertStyle;
