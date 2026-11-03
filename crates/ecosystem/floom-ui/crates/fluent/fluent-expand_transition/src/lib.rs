//! Fluent expand transition, ported to the Floom 2.0 widget API.
use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::PresenceTransition;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Conditional, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ExpandTransitionTokens};
use floom_core_theme::props::expand_transition::ExpandTransitionProps;
use floom_core_theme::theme::{MultiTheme, Theme};

const DESIGN_SYSTEM: &str = "fluent";

pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentExpandTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
}

pub struct FluentExpandTransition {
    props: ExpandTransitionProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    show: SharedFlag,
}

impl FluentExpandTransition {
    pub fn new() -> Self {
        Self {
            props: ExpandTransitionProps::default(),
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

    pub fn props(&self) -> &ExpandTransitionProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentExpandTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::ExpandTransition(tokens) =
            theme.component(ComponentId::ExpandTransition)
        else {
            unreachable!("ComponentId::ExpandTransition must resolve to ExpandTransitionTokens")
        };
        FluentExpandTransitionStyle {
            design_system: DESIGN_SYSTEM,
            duration_ms: tokens.duration_ms,
        }
    }

    pub fn transition(&self) -> Option<PresenceTransition> {
        let style = self.resolved_style();
        self.transition_for(&style)
    }

    fn transition_for(&self, style: &FluentExpandTransitionStyle) -> Option<PresenceTransition> {
        // per frame. See the module note.
        None
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

impl Default for FluentExpandTransition {
    fn default() -> Self {
        Self::new()
    }
}
