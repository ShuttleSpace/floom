use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::PresenceTransition;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Conditional, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, SlideTransitionTokens};
use floom_core_theme::props::slide_transition::{SlideDirection, SlideTransitionProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_runtime::theme_signal;

const DESIGN_SYSTEM: &str = "material";

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSlideTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
    /// How far the content travels, in pixels.
    pub offset: f32,
    pub direction: SlideDirection,
}

/// Material's slide transition.
pub struct MaterialSlideTransition {
    props: SlideTransitionProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    /// The shared "is it showing" flag.
    show: SharedFlag,
}

impl MaterialSlideTransition {
    pub fn new() -> Self {
        Self {
            props: SlideTransitionProps::default(),
            child: None,
            theme: theme_signal(),
            show: shared_flag(false),
        }
    }

    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    pub fn show_signal(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Shows or hides the content, through the shared flag.
    pub fn show(mut self, show: bool) -> Self {
        write_flag(&self.show, show);
        self
    }

    pub fn props(&self) -> &SlideTransitionProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialSlideTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideTransition(tokens) =
            theme.component(ComponentId::SlideTransition)
        else {
            unreachable!("ComponentId::SlideTransition must resolve to SlideTransitionTokens")
        };
        MaterialSlideTransitionStyle {
            design_system: DESIGN_SYSTEM,
            duration_ms: tokens.duration_ms,
            offset: tokens.offset,
            direction: self.props.direction,
        }
    }

    pub fn transition(&self) -> Option<PresenceTransition> {
        let style = self.resolved_style();
        self.transition_for(&style)
    }

    /// The transition for a resolved style. Split out so `transition` and
    /// `view` cannot disagree about which one they mean.
    fn transition_for(&self, style: &MaterialSlideTransitionStyle) -> Option<PresenceTransition> {
        Some(match style.direction {
            SlideDirection::X => {
                PresenceTransition::slide_x(f64::from(style.duration_ms) / 1000.0, style.offset)
            }
            SlideDirection::Y => {
                PresenceTransition::slide(f64::from(style.duration_ms) / 1000.0, style.offset)
            }
        })
    }

    /// The content, mounted only while the flag says so.
    pub fn view(self) -> Conditional {
        let style = self.resolved_style();
        let transition = self.transition_for(&style);
        let child = self.child.clone();
        let show = self.show.clone();

        let mut conditional = Conditional::new(false).when_flag(show).show_with(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || match child.clone() {
                Some(build) => build(),
                None => Box::new(Text::new(String::new())),
            },
        );
        if let Some(transition) = transition {
            conditional = conditional.enter(transition).exit(transition);
        }
        conditional
    }
}

impl Default for MaterialSlideTransition {
    fn default() -> Self {
        Self::new()
    }
}
