pub use floom_core_theme::component_tokens::{ComponentId, KbdTokens};
pub use floom_core_theme::props::kbd::KbdProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_kbd::CupertinoKbd as Kbd;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_kbd::FluentKbd as Kbd;
#[cfg(feature = "material")]
pub use floom_widget_md_kbd::MdKbd as Kbd;
