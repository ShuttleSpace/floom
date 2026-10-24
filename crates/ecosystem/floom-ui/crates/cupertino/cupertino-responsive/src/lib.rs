//! Cupertino responsive box, ported to the Floom 2.0 widget API.
//!
//! A box that keeps a width-to-height ratio as it is resized -- a media
//! placeholder, a 16:9 video frame.
//!
//! ## The aspect ratio is now a layout capability
//!
//! v1 wrote `.aspect_ratio(aspect)` on the style closure, and 2.0's
//! `LayoutRequest` had no such field: a box could state its width and its
//! height, but not "these two in this ratio", which is the one thing a
//! responsive box is for -- a fixed height would be wrong at every width but
//! one.
//!
//! So `LayoutRequest` grew `aspect_ratio`, the layout adapter carries it, and
//! the host lowers it into the layout engine's own notion of a ratio. This is
//! the same shape of addition as `Spacer::growing`: a capability the design
//! system asked for and the engine could not say.
//!
//! ## The ratio and the parent's stretch
//!
//! The box states the ratio; whether it keeps that height is the parent's
//! business. A parent that stretches its children gives them the parent's
//! cross-axis extent, and a stretched child is given a height, so the ratio is
//! moot. The kit states what it wants and says so here rather than pretending
//! the ratio always wins.

use std::sync::Arc;

use floom_core::{LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::{Align, Surface, Text};

use floom_core_theme::props::responsive::ResponsiveProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// A builder for the box's content, replayed each time it is mounted.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

/// Cupertino's responsive box.
pub struct CupertinoResponsive {
    props: ResponsiveProps,
    theme: RwSignal<MultiTheme>,
    child: Option<ChildBuilder>,
}

impl CupertinoResponsive {
    /// A box with the given width-to-height ratio: `16.0 / 9.0` for a
    /// widescreen frame.
    pub fn new(aspect_ratio: f32) -> Self {
        Self {
            props: ResponsiveProps { aspect_ratio },
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

impl Default for CupertinoResponsive {
    fn default() -> Self {
        Self::new(1.0)
    }
}
