use std::sync::{Arc, Mutex};

use floom_core::WidgetAny;
use floom_signal::{RwSignal, SignalGet};
use floom_widgets::text_input::{shared_flag, SharedBuffer, SharedFlag};
use floom_widgets::{Stack, Text};

use floom_core_theme::props::date_adapter_provider::DateAdapterProviderProps;
use floom_core_theme::theme::MultiTheme;
use floom_widget_runtime::theme_signal;

/// A builder for the subtree, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Material's date_adapter_provider.
pub struct MaterialDateAdapterProvider {
    props: DateAdapterProviderProps,
    /// The shared the name of a date adapter.
    value: SharedBuffer,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl MaterialDateAdapterProvider {
    pub fn new() -> Self {
        let props = DateAdapterProviderProps::default();
        Self {
            value: Arc::new(Mutex::new(props.adapter.clone())),
            props,
            theme: theme_signal(),
            child: None,
        }
    }

    /// The initial date adapter.
    pub fn adapter(mut self, adapter: impl Into<String>) -> Self {
        let adapter = adapter.into();
        self.props.adapter = adapter.clone();
        *self.value.lock().unwrap() = adapter;
        self
    }

    pub fn adapter_signal(&self) -> SharedBuffer {
        self.value.clone()
    }

    /// The subtree this provider wraps.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &DateAdapterProviderProps {
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

impl Default for MaterialDateAdapterProvider {
    fn default() -> Self {
        Self::new()
    }
}
