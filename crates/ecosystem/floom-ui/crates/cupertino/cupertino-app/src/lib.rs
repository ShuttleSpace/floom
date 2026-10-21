//! Cupertino application root, ported to the Floom 2.0 widget API.
//!
//! The root of an application: a full-viewport box in the app's background
//! colour, with the shell's bars and the main region inside it.
//!
//! ## The layout signal is still initialised
//!
//! v1 called `layout_signal()` while building the view, and the shell pieces --
//! the app bar, the footer, the navigation drawer -- publish their sizes into
//! it for `main` to inset itself by. Dropping that would leave the content
//! under the bars, so the port keeps it.
//!
//! ## The box
//!
//! v1's `Container` was both a decoration and a holder. 2.0's `Container` is a
//! leaf -- a decorated box with nothing inside it -- so the root is a `Surface`,
//! which holds the content and paints the background.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::Surface;

use floom_core_theme::layout::layout_signal;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

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
        // the shell pieces publish their sizes into it.
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

