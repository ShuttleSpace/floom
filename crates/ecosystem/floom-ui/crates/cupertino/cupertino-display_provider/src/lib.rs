use floom::prelude::*;
use floom::reactive::RwSignal;
use floom::view::AnyView;
use floom_core_theme::props::display_provider::DisplayProviderProps;

/// Material [`DisplayProvider`]: owns a density [`RwSignal`] (compact/dense)
/// and shares it with its subtree through [`CupertinoDisplayProvider::density_signal`].
///
/// NOTE: subtree-scoped density overrides are not enforced by the type system
/// (Rust/Floom static dispatch); descendants read `density_signal()` and react.
pub struct CupertinoDisplayProvider {
    props: DisplayProviderProps,
    dense: RwSignal<bool>,
    child: Option<AnyView>,
}

impl CupertinoDisplayProvider {
    pub fn new() -> Self {
        let props = DisplayProviderProps::default();
        Self {
            dense: RwSignal::new(props.dense),
            props,
            child: None,
        }
    }

    /// When true, descendants should render in compact (dense) density.
    pub fn dense(mut self, dense: bool) -> Self {
        self.props.dense = dense;
        self.dense.set(dense);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the density flag, shared with descendants.
    pub fn density_signal(&self) -> RwSignal<bool> {
        self.dense
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &DisplayProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for CupertinoDisplayProvider {
    fn default() -> Self {
        Self::new()
    }
}
