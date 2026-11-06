use std::collections::HashMap;
use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_map, SharedMap};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::icons_provider::IconsProviderProps;
use floom_core_theme::theme::MultiTheme;
use floom_widget_runtime::theme_signal;

/// A builder for the subtree, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Material's icons_provider.
pub struct MaterialIconsProvider {
    props: IconsProviderProps,
    /// The shared a map of icon-name overrides.
    value: SharedMap,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl MaterialIconsProvider {
    pub fn new() -> Self {
        let props = IconsProviderProps::default();
        Self {
            value: shared_map(props.icons.clone()),
            props,
            theme: theme_signal(),
            child: None,
        }
    }

    /// The initial a map of icon-name overrides.
    pub fn icons(mut self, icons: HashMap<String, String>) -> Self {
        self.props.icons = icons.clone();
        *self.value.lock().unwrap() = icons;
        self
    }

    pub fn icons_signal(&self) -> SharedMap {
        self.value.clone()
    }

    /// The subtree this provider wraps.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &IconsProviderProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn view(self) -> Stack {
        let child = self.child.clone();
        Stack::new(floom_core::Axis::Vertical).push(move || match child.clone() {
            Some(build) => build(),
            None => Box::new(Text::new(String::new())),
        })
    }
}

impl Default for MaterialIconsProvider {
    fn default() -> Self {
        Self::new()
    }
}
