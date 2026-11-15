//! Cupertino fade transition, ported to the Floom 2.0 widget API.
//!
use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::PresenceTransition;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Conditional, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, FadeTransitionTokens};
use floom_core_theme::props::fade_transition::FadeTransitionProps;
use floom_core_theme::theme::{MultiTheme, Theme};

const DESIGN_SYSTEM: &str = "cupertino";

/// A builder for the content, replayed each time it is mounted.
///
/// A 2.0 slot is a replayed `Fn`, so the content cannot be a value: it is
/// built again every time it appears.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoFadeTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
}

/// Cupertino's fade transition.
pub struct CupertinoFadeTransition {
    props: FadeTransitionProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    /// The shared "is it showing" flag.
    show: SharedFlag,
}

impl CupertinoFadeTransition {
    pub fn new() -> Self {
        Self {
            props: FadeTransitionProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            show: shared_flag(false),
        }
    }

    /// Set the content that appears and disappears.
    ///
    /// v1 took a factory closure for the same reason this does: the child is
    /// mounted and unmounted on every enter and leave, so it must be
    /// rebuildable.
    pub fn child(mut self, build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static) -> Self {
        self.child = Some(Arc::new(build));
        self
    }

    /// The shared flag that shows and hides the content.
    ///
    /// v1's `show_signal()`: an external trigger -- a button, a route -- writes
    /// it and the content follows.
    pub fn show_signal(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Shows or hides the content, through the shared flag.
    pub fn show(mut self, show: bool) -> Self {
        write_flag(&self.show, show);
        self
    }

    pub fn props(&self) -> &FadeTransitionProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoFadeTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::FadeTransition(tokens) = theme.component(ComponentId::FadeTransition)
        else {
            unreachable!("ComponentId::FadeTransition must resolve to FadeTransitionTokens")
        };
        CupertinoFadeTransitionStyle {
            design_system: DESIGN_SYSTEM,
            duration_ms: tokens.duration_ms,
        }
    }

    /// The transition this kit plays, if it plays one.
    ///
    /// The same transition on the way in and on the way out, which is what v1
    /// did: one animation, run forwards and backwards.
    pub fn transition(&self) -> Option<PresenceTransition> {
        let style = self.resolved_style();
        self.transition_for(&style)
    }

    /// The transition for a resolved style. Split out so `transition` and
    /// `view` cannot disagree about which one they mean.
    fn transition_for(&self, style: &CupertinoFadeTransitionStyle) -> Option<PresenceTransition> {
        Some(PresenceTransition::fade(
            f64::from(style.duration_ms) / 1000.0,
        ))
    }

    /// The content, mounted only while the flag says so.
    pub fn view(self) -> Conditional {
        let style = self.resolved_style();
        // v1's fade: opacity 0 -> 1 over the token's duration, played on
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

impl Default for CupertinoFadeTransition {
    fn default() -> Self {
        Self::new()
    }
}
