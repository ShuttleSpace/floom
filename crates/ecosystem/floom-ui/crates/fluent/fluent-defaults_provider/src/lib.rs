use std::collections::HashMap;

use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::props::defaults_provider::DefaultsProviderProps;

/// Material [`DefaultsProvider`]: owns a flat map of `Component.Prop -> value`
/// [`RwSignal`] and shares it with its subtree through
/// [`FluentDefaultsProvider::defaults_signal`].
///
/// NOTE: subtree-scoped default overrides are not enforced by the type system
/// (Rust/Floom static dispatch); descendants read `defaults_signal()` and react.
pub struct FluentDefaultsProvider {
    props: DefaultsProviderProps,
    defaults: RwSignal<HashMap<String, String>>,
    child: Option<AnyView>,
}

impl FluentDefaultsProvider {
    pub fn new() -> Self {
        let props = DefaultsProviderProps::default();
        Self {
            defaults: RwSignal::new(props.defaults.clone()),
            props,
            child: None,
        }
    }

    /// Seed the default-value map applied to descendants.
    pub fn defaults(mut self, defaults: HashMap<String, String>) -> Self {
        self.props.defaults = defaults.clone();
        self.defaults.set(defaults);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the defaults map, shared with descendants.
    pub fn defaults_signal(&self) -> RwSignal<HashMap<String, String>> {
        self.defaults
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &DefaultsProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for FluentDefaultsProvider {
    fn default() -> Self {
        Self::new()
    }
}
