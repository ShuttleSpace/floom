//! Fluent display_provider, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_flag, SharedBuffer, SharedFlag};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::display_provider::DisplayProviderProps;
use floom_core_theme::theme::MultiTheme;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

pub struct FluentDisplayProvider {
    props: DisplayProviderProps,
    value: SharedFlag,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentDisplayProvider {
    pub fn new() -> Self {
        let props = DisplayProviderProps::default();
        Self {
            value: shared_flag(props.dense),
            props,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn dense(mut self, bool: bool) -> Self {
        self.props.dense = bool;
        *self.value.lock().unwrap() = bool;
        self
    }

    pub fn density_signal(&self) -> SharedFlag {
        self.value.clone()
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &DisplayProviderProps {
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

impl Default for FluentDisplayProvider {
    fn default() -> Self {
        Self::new()
    }
}
