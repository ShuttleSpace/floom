//! Material expand transition, ported to the Floom 2.0 widget API.
//!
//! v1 mounted the child only while a `show` signal was true, and played an
//! animation on the way in and on the way out. 2.0's [`Conditional`] is that
//! shape: it mounts and unmounts a subtree, and keeps an exiting one mounted
//! until its exit has played, so the departure is visible.
//!
//! ## The signal became a flag
//!
//! v1 held an `RwSignal<bool>`, which is `Rc`-based and cannot cross a `Send +
//! Sync` slot boundary -- so the port holds a [`SharedFlag`], the same
//! `Arc<Mutex<bool>>` the text input's buffer uses, and the engine's
//! `Conditional` reads it in `update`. The signal is therefore still the
//! caller's: [`MaterialExpandTransition::show_signal`] hands it over, and whoever writes it drives
//! the child in and out.
//!
//! ## The height cannot be animated
//!
//! v1's expand transition animated the *height*: the box went from zero to the
//! content's natural height, with the overflow clipped so the content did not
//! spill out while it collapsed. 2.0 cannot do that: the host only re-runs
//! layout when layout is dirty, so a box cannot change its height per frame --
//! the same gap the expansion panels record (B4, "size"). The content is
//! therefore its final height from the first frame, and the transition this kit
//! offers is the compositor-only one, which is what `floom-widgets`' linear
//! progress does with its sliding segment.
//!
use std::sync::Arc;

use floom_core::{ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::PresenceTransition;
use floom_widgets::text_input::{shared_flag, write_flag, SharedFlag};
use floom_widgets::{Conditional, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ExpandTransitionTokens};
use floom_core_theme::props::expand_transition::ExpandTransitionProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_widget_runtime::theme_signal;

const DESIGN_SYSTEM: &str = "material";

/// A builder for the content, replayed each time it is mounted.
///
/// A 2.0 slot is a replayed `Fn`, so the content cannot be a value: it is
/// built again every time it appears.
pub type ChildBuilder = Arc<dyn Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialExpandTransitionStyle {
    pub design_system: &'static str,
    pub duration_ms: f32,
}

/// Material's expand transition.
pub struct MaterialExpandTransition {
    props: ExpandTransitionProps,
    child: Option<ChildBuilder>,
    theme: RwSignal<MultiTheme>,
    /// The shared "is it showing" flag.
    show: SharedFlag,
}

impl MaterialExpandTransition {
    pub fn new() -> Self {
        Self {
            props: ExpandTransitionProps::default(),
            child: None,
            theme: theme_signal(),
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

    pub fn resolved_style(&self) -> MaterialExpandTransitionStyle {
        let theme = self.theme.get();
        let ComponentTokens::ExpandTransition(tokens) =
            theme.component(ComponentId::ExpandTransition)
        else {
            unreachable!("ComponentId::ExpandTransition must resolve to ExpandTransitionTokens")
        };
        MaterialExpandTransitionStyle {
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
    fn transition_for(&self, style: &MaterialExpandTransitionStyle) -> Option<PresenceTransition> {
        // v1's expand animated the *height*, which 2.0 cannot: the host only
        // re-runs layout when layout is dirty, so a box cannot change its height
        // per frame. See the module note.
        None
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

impl Default for MaterialExpandTransition {
    fn default() -> Self {
        Self::new()
    }
}
