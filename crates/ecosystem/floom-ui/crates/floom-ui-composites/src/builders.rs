use std::sync::Arc;

use floom_core::WidgetAny;

/// A slot that builds a widget when it is mounted.
pub type WidgetBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;
