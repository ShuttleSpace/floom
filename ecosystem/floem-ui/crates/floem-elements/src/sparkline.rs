pub use floem_core_theme::component_tokens::{ComponentId, SparklineTokens};
pub use floem_core_theme::props::sparkline::SparklineProps;

#[cfg(feature = "cupertino")]
pub use floem_widget_cupertino_sparkline::CupertinoSparkline as Sparkline;
#[cfg(feature = "fluent")]
pub use floem_widget_fluent_sparkline::FluentSparkline as Sparkline;
#[cfg(feature = "material")]
pub use floem_widget_md_sparkline::MdSparkline as Sparkline;
