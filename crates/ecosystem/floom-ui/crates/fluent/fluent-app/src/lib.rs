//! Fluent application root, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::Surface;

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

pub struct FluentApp {
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl FluentApp {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
        }
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn view(self) -> Surface {
        let _ = layout_signal();

        let theme = self.theme;
        let child = self.child.clone();

        Surface::new()
            .request(LayoutRequest::new().fill())
            .background(to_paint(theme.get().colors().background))
            .push(move || match child.clone() {
                Some(build) => build(),
                None => Box::new(floom_widgets::Text::new(String::new())),
            })
    }
}

impl Default for FluentApp {
    fn default() -> Self {
        Self::new()
    }
}
