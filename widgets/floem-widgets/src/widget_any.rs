//! Opt-in downcasting for widgets stored behind a trait object.
//!
//! A retained host owns its widgets as `Box<dyn Widget>`, which erases the
//! concrete type — but an application still needs to read a `Slider`'s value or
//! push a new string into a `Text`. Rust cannot downcast without `Any`, and
//! `Widget` itself stays free of that machinery so third-party implementations
//! do not have to opt in.
//!
//! Widgets that are meant to live in a type-erased tree implement
//! [`WidgetAny`] once (via the macro below); the host stores that object
//! instead, and — because `Widget` is a supertrait — can still call every
//! `Widget` method on it.

use std::any::Any;

use floem_core::Widget;

use crate::primitives::SemanticsHint;

/// A widget that can be recovered from a trait object.
pub trait WidgetAny: Widget {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// The accessibility description of this widget.
    ///
    /// This exists because [`SemanticsContext`] carries only a label: a host
    /// holding `dyn Widget` has no other way to learn a widget's role or state
    /// without `floem-core` growing a dependency on `floem-accessibility`
    /// (which would invert the framework's layering). Returning the hint here
    /// is how roles reach the retained semantics tree today.
    ///
    /// [`SemanticsContext`]: floem_core::SemanticsContext
    fn semantics_hint(&self) -> Option<SemanticsHint>;
}

macro_rules! impl_widget_any {
    ($($ty:ty),* $(,)?) => {
        $(
            impl WidgetAny for $ty {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<SemanticsHint> {
                    Some(<$ty>::semantics_hint(self))
                }
            }
        )*
    };
}

impl_widget_any!(
    crate::primitives::Text,
    crate::primitives::Icon,
    crate::primitives::Image,
    crate::primitives::Container,
    crate::primitives::Spacer,
    crate::primitives::Divider,
    crate::controls::Button,
    crate::controls::TextField,
    crate::controls::Checkbox,
    crate::controls::Radio,
    crate::controls::Switch,
    crate::controls::Slider,
    crate::controls::Progress,
    crate::controls::Select,
);
