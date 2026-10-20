//! Cupertino theme_provider, ported to the Floom 2.0 widget API.
//!
//! A provider owns a dark-mode flag and shares it with its subtree. v1 held it in an
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

use floom_core_theme::props::theme_provider::{ThemeProviderProps};
use floom_core_theme::theme::MultiTheme;

/// A builder for the subtree, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Cupertino's theme_provider.
pub struct CupertinoThemeProvider {
    props: ThemeProviderProps,
    /// The shared a dark-mode flag.
    value: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl CupertinoThemeProvider {
    pub fn new() -> Self {
        let props = ThemeProviderProps::default();
        Self {
            value: shared_flag(props.dark),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            child: None,
        }
    }

    /// The initial dark-mode flag.
    pub fn dark(mut self, bool: bool) -> Self {
        self.props.dark = bool;
        *self.value.lock().unwrap() = bool;
        self
    }

    /// The shared a dark-mode flag, for the descendants that read it.
    ///
    /// v1's `*_signal()`: the handle a descendant reads and reacts to.
    pub fn dark_signal(&self) -> SharedFlag {
        self.value.clone()
    }

    /// The subtree this provider wraps.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &ThemeProviderProps {
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

impl Default for CupertinoThemeProvider {
    fn default() -> Self {
        Self::new()
    }
}
