//! `WidgetAny` for the composites in this crate.
//!
//! `floom-widgets` keeps its own list, and every widget it ships appears in it,
//! because a host that mounts a widget needs to recover it by type. That list
//! cannot simply be extended from here — it is a private macro over
//! `floom-widgets`' own paths — so each crate spells its own list out.
//!
//! The cost of that is a rule with no enforcement: adding a widget here and
//! forgetting this file is a compile error at the mount site rather than
//! anything subtler, which is the direction one wants a mistake to fail in.

use std::any::Any;

use floom_core::{SemanticsHint, WidgetAny};

/// Implements [`WidgetAny`] for widgets that carry no accessibility description
/// of their own.
///
/// A widget that publishes nothing is not an error — a decorative composite has
/// nothing to say, and its inner `Pressable` or `Text` does the publishing. What
/// matters is that it can still be recovered by type after a mount.
macro_rules! impl_widget_any {
    ($($widget:ty),* $(,)?) => {
        $(
            impl WidgetAny for $widget {
                fn as_any(&self) -> &dyn Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn Any {
                    self
                }

                fn semantics_hint(&self) -> Option<SemanticsHint> {
                    None
                }
            }
        )*
    };
}

impl_widget_any!(crate::nav::NavHost);
