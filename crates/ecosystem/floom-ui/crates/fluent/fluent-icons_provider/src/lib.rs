//! Fluent icons_provider, ported to the Floom 2.0 widget API.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_map, SharedMap};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::icons_provider::IconsProviderProps;
use floom_core_theme::theme::MultiTheme;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

pub struct FluentIconsProvider {
    props: IconsProviderProps,
    value: SharedMap,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentIconsProvider {
    pub fn new() -> Self {
        let props = IconsProviderProps::default();
        Self {
            value: shared_map(props.icons.clone()),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn icons(mut self, icons: HashMap<String, String>) -> Self {
        self.props.icons = icons.clone();
        *self.value.lock().unwrap() = icons;
        self
    }

    pub fn icons_signal(&self) -> SharedMap {
        self.value.clone()
    }

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

impl Default for FluentIconsProvider {
    fn default() -> Self {
        Self::new()
    }
}
