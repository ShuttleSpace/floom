//! Fluent display_provider, ported to the Floom 2.0 widget API.
//!
//! A provider owns a density flag and shares it with its subtree. v1 held it in an
//! `RwSignal`; 2.0 holds it in a shared flag, because an `RwSignal` is
//! `Rc`-based and cannot cross a `Send + Sync` slot boundary -- the same reason
//! the dialogs' `open_signal()` became a shared flag.
//!
//! ## The subtree is not re-themed
//!
//! v1's own note said it: because Rust uses static dispatch, a provider cannot
//! re-theme a subtree the way a dynamically-typed framework could. It hands its
//! descendants a typed handle and they read it. 2.0 cannot do it either, and
//! the port does not pretend to: the handle is shared, and whoever writes it is
//! the one who decides what the subtree shows.

use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::{Stack, Text};
use floom_widgets::text_input::{SharedBuffer, SharedFlag, shared_flag};

use floom_core_theme::props::display_provider::{DisplayProviderProps};
use floom_core_theme::theme::MultiTheme;

/// A builder for the subtree, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Fluent's display_provider.
pub struct FluentDisplayProvider {
    props: DisplayProviderProps,
    /// The shared a density flag.
    value: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentDisplayProvider {
    pub fn new() -> Self {
        let props = DisplayProviderProps::default();
        Self {
            value: shared_flag(props.dense),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    /// The initial density flag.
    pub fn dense(mut self, bool: bool) -> Self {
        self.props.dense = bool;
        *self.value.lock().unwrap() = bool;
        self
    }

    /// The shared a density flag, for the descendants that read it.
    ///
    /// v1's `*_signal()`: the handle a descendant reads and reacts to.
    pub fn density_signal(&self) -> SharedFlag {
        self.value.clone()
    }

    /// The subtree this provider wraps.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &DisplayProviderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The subtree, with the provider's handle beside it.
    ///
    /// v1 wrapped the child in a `Container` -- a box that added nothing but a
    /// node -- and 2.0's `Container` is a leaf that cannot hold anything, so
    /// the passthrough is a `Stack`.
    pub fn view(self) -> Stack {
        let child = self.child.clone();
        Stack::new(floom_core::Axis::Vertical).push(move || match child.clone() {
            Some(build) => build(),
            None => Box::new(Text::new(String::new())),
        })
    }
}

impl Default for FluentDisplayProvider {
    fn default() -> Self {
        Self::new()
    }
}
