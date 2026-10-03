pub use floom_core_theme::component_tokens::{ComponentId, OtpInputTokens};
pub use floom_core_theme::props::otp_input::OtpInputProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_otp_input::CupertinoOtpInput as OtpInput;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_otp_input::FluentOtpInput as OtpInput;
#[cfg(feature = "material")]
pub use floom_widget_md_otp_input::MdOtpInput as OtpInput;
