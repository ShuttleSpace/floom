pub use floom_core_theme::component_tokens::{ComponentId, SparklineTokens};
pub use floom_core_theme::props::sparkline::SparklineProps;

#[cfg(feature = "cupertino")]
pub use floom_widget_cupertino_sparkline::CupertinoSparkline as Sparkline;
#[cfg(feature = "fluent")]
pub use floom_widget_fluent_sparkline::FluentSparkline as Sparkline;
#[cfg(feature = "material")]
pub use floom_widget_md_sparkline::MdSparkline as Sparkline;
