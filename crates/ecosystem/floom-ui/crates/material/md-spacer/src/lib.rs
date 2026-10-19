//! Material spacer, ported to the Floom 2.0 widget API.
//!
//! A spacer is an empty, flex-growing view that pushes its siblings apart. v1
//! spelled it as an empty `Container` with a `flex_grow` on it; 2.0 spells it
//! [`Spacer`], and the engine's `Spacer` grew the one thing v1's kit needed and
//! the engine did not have: a *factor*.
//!
//! ## The factor
//!
//! The engine's [`Spacer::flexible`] absorbs all the free space -- grow 1.0 --
//! and a fixed-extent spacer takes no space at all. v1's kit took a grow
//! *factor*, and the default (1.0) is the only one the engine could say. So
//! [`Spacer::growing`] was added: a spacer that absorbs free space by a factor,
//! of which `flexible()` is the 1.0 case.
//!
//! ## What the port names
//!
//! * **the theme is not read.** A spacer draws nothing, so the kit's theme
//!   cannot change what it looks like -- it is carried so the kit has the same
//!   shape as its siblings, and so a caller can read it back.

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_widgets::Spacer;

use floom_core_theme::props::spacer::SpacerProps;
use floom_core_theme::theme::MultiTheme;
use floom_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSpacerStyle {
    pub grow: f32,
}

/// Material's spacer.
pub struct MdSpacer {
    props: SpacerProps,
    theme: RwSignal<MultiTheme>,
}

impl MdSpacer {
    pub fn new() -> Self {
        Self {
            props: SpacerProps::default(),
            theme: theme_signal(),
        }
    }

    /// Flex grow factor applied to the empty spacer view.
    pub fn grow(mut self, grow: f32) -> Self {
        self.props.grow = grow;
        self
    }

    pub fn props(&self) -> &SpacerProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdSpacerStyle {
        MdSpacerStyle {
            grow: self.props.grow,
        }
    }

    /// The empty, flex-growing view that pushes its siblings apart.
    pub fn view(self) -> Spacer {
        Spacer::growing(self.props.grow)
    }
}

impl Default for MdSpacer {
    fn default() -> Self {
        Self::new()
    }
}
