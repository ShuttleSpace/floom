//! Cupertino application root, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::Surface;

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// A builder for the application's content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Cupertino's application root.
pub struct CupertinoApp {
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl CupertinoApp {
    pub fn new() -> Self {
        Self {
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            child: None,
        }
    }

    /// The shell's bars and the main region, which fill the viewport.
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

    /// The application root, ready to mount.
    pub fn view(self) -> Surface {
        // Initialise the process-wide layout signal for the whole app, as v1 did:
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

impl Default for CupertinoApp {
    fn default() -> Self {
        Self::new()
    }
}
