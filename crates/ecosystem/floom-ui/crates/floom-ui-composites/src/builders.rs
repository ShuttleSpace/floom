//! `WidgetBuilder` -- a slot that builds a widget.
//!
//! 2.0's slots are replayed `Fn`s and a widget is mounted by its parent, so
//! a caller who wants to hand over a piece of content hands over a builder
//! rather than a built widget. The kits call it an `IconBuilder` or a
//! `RowBuilder`; it is one thing.

use std::sync::Arc;

use floom_core::WidgetAny;

/// A slot that builds a widget when it is mounted.
pub type WidgetBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;
