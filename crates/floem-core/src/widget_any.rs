//! Opt-in downcasting for widgets stored behind a trait object.
//!
//! A retained host owns its widgets as `Box<dyn WidgetAny>`, which erases the
//! concrete type — but an application still needs to read a `Slider`'s value or
//! push a new string into a `Text`. Rust cannot downcast a bare `dyn Widget`
//! without `Any`, and `Widget` itself stays free of that machinery so third-party
//! implementations do not have to opt in. Widgets that live in a type-erased
//! tree implement this once; the host stores that object and — because `Widget`
//! is a supertrait — can still call every `Widget` method on it.

use std::any::Any;

/// A widget that can be recovered from a trait object and that declares its
/// accessibility description.
pub trait WidgetAny: crate::Widget + Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// The accessibility description of this widget, in engine-neutral form.
    ///
    /// This exists because [`crate::SemanticsHint`] is the bridge between a widget
    /// and the framework's accessibility layer: `floem-core` owns the neutral
    /// vocabulary and the host lowers it into `floem-accessibility` when it
    /// builds the semantics tree. Returning the hint here is how roles reach
    /// that tree without `floem-core` depending on the accessibility framework.
    fn semantics_hint(&self) -> Option<crate::SemanticsHint>;
}
