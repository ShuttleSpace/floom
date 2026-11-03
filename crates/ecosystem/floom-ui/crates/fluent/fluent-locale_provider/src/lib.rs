//! Fluent locale_provider, ported to the Floom 2.0 widget API.

use std::sync::{Arc, Mutex};

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_flag, SharedBuffer, SharedFlag};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::locale_provider::LocaleProviderProps;
use floom_core_theme::theme::MultiTheme;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

pub struct FluentLocaleProvider {
    props: LocaleProviderProps,
    value: SharedBuffer,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentLocaleProvider {
    pub fn new() -> Self {
        let props = LocaleProviderProps::default();
        Self {
            value: Arc::new(Mutex::new(props.locale.clone())),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        let locale = locale.into();
        self.props.locale = locale.clone();
        *self.value.lock().unwrap() = locale;
        self
    }

    pub fn locale_signal(&self) -> SharedBuffer {
        self.value.clone()
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &LocaleProviderProps {
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

impl Default for FluentLocaleProvider {
    fn default() -> Self {
        Self::new()
    }
}
