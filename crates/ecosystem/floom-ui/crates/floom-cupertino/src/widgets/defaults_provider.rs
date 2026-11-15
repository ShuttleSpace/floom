//! Cupertino defaults_provider, ported to the Floom 2.0 widget API.
//!

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_map, SharedMap};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::defaults_provider::DefaultsProviderProps;
use floom_core_theme::theme::MultiTheme;

/// A builder for the subtree, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Cupertino's defaults_provider.
pub struct CupertinoDefaultsProvider {
    props: DefaultsProviderProps,
    /// The shared a flat map of component-property overrides.
    value: SharedMap,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl CupertinoDefaultsProvider {
    pub fn new() -> Self {
        let props = DefaultsProviderProps::default();
        Self {
            value: shared_map(props.defaults.clone()),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            child: None,
        }
    }

    /// The initial a flat map of component-property overrides.
    pub fn defaults(mut self, defaults: HashMap<String, String>) -> Self {
        self.props.defaults = defaults.clone();
        *self.value.lock().unwrap() = defaults;
        self
    }

    /// The shared a flat map of component-property overrides, for the descendants that read it.
    ///
    /// v1's `*_signal()`: the handle a descendant reads and reacts to.
    pub fn defaults_signal(&self) -> SharedMap {
        self.value.clone()
    }

    /// The subtree this provider wraps.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &DefaultsProviderProps {
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

impl Default for CupertinoDefaultsProvider {
    fn default() -> Self {
        Self::new()
    }
}
