//! Fluent defaults_provider, ported to the Floom 2.0 widget API.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_map, SharedMap};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::defaults_provider::DefaultsProviderProps;
use floom_core_theme::theme::MultiTheme;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

pub struct FluentDefaultsProvider {
    props: DefaultsProviderProps,
    value: SharedMap,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentDefaultsProvider {
    pub fn new() -> Self {
        let props = DefaultsProviderProps::default();
        Self {
            value: shared_map(props.defaults.clone()),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn defaults(mut self, defaults: HashMap<String, String>) -> Self {
        self.props.defaults = defaults.clone();
        *self.value.lock().unwrap() = defaults;
        self
    }

    pub fn defaults_signal(&self) -> SharedMap {
        self.value.clone()
    }

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

    pub fn view(self) -> Stack {
        let child = self.child.clone();
        Stack::new(floom_core::Axis::Vertical).push(move || match child.clone() {
            Some(build) => build(),
            None => Box::new(Text::new(String::new())),
        })
    }
}

impl Default for FluentDefaultsProvider {
    fn default() -> Self {
        Self::new()
    }
}
