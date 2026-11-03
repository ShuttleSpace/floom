
use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::props::responsive::ResponsiveProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

/// A builder for the box's content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Material's responsive box.
pub struct MaterialResponsive {
    props: ResponsiveProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl MaterialResponsive {
    /// A box with the given width-to-height ratio: `16.0 / 9.0` for a
    /// widescreen frame.
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            props: ResponsiveProps { aspect_ratio },
            theme: theme_signal(),
            child: None,
        }
    }

    /// The box's content.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn props(&self) -> &ResponsiveProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The box, ready to mount.
    pub fn view(self) -> Surface {
        let theme = self.theme;
        let ratio = self.props.aspect_ratio;
        let child = self.child.clone();

        Surface::new()
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .aspect_ratio(ratio)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(theme.get().colors().surface_variant))
            .push(move || match child.clone() {
                Some(build) => build(),
                None => Box::new(Text::new(String::new())),
            })
    }
}

impl Default for MaterialResponsive {
    fn default() -> Self {
        Self::new(1.0)
    }
}
