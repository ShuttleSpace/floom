use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::props::date_adapter_provider::DateAdapterProviderProps;

/// Material [`DateAdapterProvider`]: owns a date-adapter identifier [`RwSignal`]
/// and shares it with its subtree through [`MdDateAdapterProvider::adapter_signal`].
///
/// NOTE: subtree-scoped adapter overrides are not enforced by the type system
/// (Rust/Floom static dispatch); descendants read `adapter_signal()` and react.
pub struct MdDateAdapterProvider {
    props: DateAdapterProviderProps,
    adapter: RwSignal<String>,
    child: Option<AnyView>,
}

impl MdDateAdapterProvider {
    pub fn new() -> Self {
        let props = DateAdapterProviderProps::default();
        Self {
            adapter: RwSignal::new(props.adapter.clone()),
            props,
            child: None,
        }
    }

    /// Initial date-adapter identifier (e.g. `"native"`, `"luxon"`).
    pub fn adapter(mut self, adapter: impl Into<String>) -> Self {
        let a = adapter.into();
        self.props.adapter = a.clone();
        self.adapter.set(a);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the adapter identifier, shared with descendants.
    pub fn adapter_signal(&self) -> RwSignal<String> {
        self.adapter
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &DateAdapterProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for MdDateAdapterProvider {
    fn default() -> Self {
        Self::new()
    }
}
