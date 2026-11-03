//! Fluent slide transition, ported to the Floom 2.0 widget API.
use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::PresenceTransition;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Conditional, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, SlideTransitionTokens};
use floom_core_theme::props::slide_transition::{SlideDirection, SlideTransitionProps};
use floom_core_theme::theme::{MultiTheme, Theme};

const DESIGN_SYSTEM: &str = "fluent";

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSlideTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
    pub offset: f32,
    pub direction: SlideDirection,
}

pub struct FluentSlideTransition {
    props: SlideTransitionProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    show: SharedFlag,
}

impl FluentSlideTransition {
    pub fn new() -> Self {
        Self {
            props: SlideTransitionProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
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

    pub fn resolved_style(&self) -> FluentSlideTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::SlideTransition(tokens) =
            theme.component(ComponentId::SlideTransition)
        else {
            unreachable!("ComponentId::SlideTransition must resolve to SlideTransitionTokens")
        };
        FluentSlideTransitionStyle {
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

    fn transition_for(&self, style: &FluentSlideTransitionStyle) -> Option<PresenceTransition> {
        Some(match style.direction {
            SlideDirection::X => {
                PresenceTransition::slide_x(f64::from(style.duration_ms) / 1000.0, style.offset)
            }
            SlideDirection::Y => {
                PresenceTransition::slide(f64::from(style.duration_ms) / 1000.0, style.offset)
            }
        })
    }

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

impl Default for FluentSlideTransition {
    fn default() -> Self {
        Self::new()
    }
}
