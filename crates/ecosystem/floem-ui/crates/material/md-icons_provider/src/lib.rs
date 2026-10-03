use std::collections::HashMap;

use floem::prelude::*;
use floem::reactive::RwSignal;
use floem::view::AnyView;
use floem_core_theme::props::icons_provider::IconsProviderProps;

/// Material [`IconsProvider`]: owns a semantic-alias -> glyph map [`RwSignal`]
/// and shares it with its subtree through [`MdIconsProvider::icons_signal`].
///
/// NOTE: subtree-scoped icon overrides are not enforced by the type system
/// (Rust/Floem static dispatch); descendants read `icons_signal()` and react.
pub struct MdIconsProvider {
    props: IconsProviderProps,
    icons: RwSignal<HashMap<String, String>>,
    child: Option<AnyView>,
}

impl MdIconsProvider {
    pub fn new() -> Self {
        let props = IconsProviderProps::default();
        Self {
            icons: RwSignal::new(props.icons.clone()),
            props,
            child: None,
        }
    }

    /// Seed the alias -> glyph map provided to descendants.
    pub fn icons(mut self, icons: HashMap<String, String>) -> Self {
        self.props.icons = icons.clone();
        self.icons.set(icons);
        self
    }

    /// The subtree wrapped by this provider.
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
        self
    }

    /// Read/write handle for the icons map, shared with descendants.
    pub fn icons_signal(&self) -> RwSignal<HashMap<String, String>> {
        self.icons
    }

    /// Immutable access to the resolved provider props.
    pub fn props(&self) -> &IconsProviderProps {
        &self.props
    }

    pub fn view(self) -> impl View {
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        Container::new(child)
    }
}

impl Default for MdIconsProvider {
    fn default() -> Self {
        Self::new()
    }
}
