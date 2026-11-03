//! Cupertino spacer, ported to the Floom 2.0 widget API.
//!

use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_widgets::Spacer;

use floom_core_theme::props::spacer::SpacerProps;
use floom_core_theme::theme::MultiTheme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoSpacerStyle {
    pub grow: f32,
}

/// Cupertino's spacer.
pub struct CupertinoSpacer {
    props: SpacerProps,
    theme: RwSignal<MultiTheme>,
}

impl CupertinoSpacer {
    pub fn new() -> Self {
        Self {
            props: SpacerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
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

    pub fn resolved_style(&self) -> CupertinoSpacerStyle {
        CupertinoSpacerStyle {
            grow: self.props.grow,
        }
    }

    /// The empty, flex-growing view that pushes its siblings apart.
    pub fn view(self) -> Spacer {
        Spacer::growing(self.props.grow)
    }
}

impl Default for CupertinoSpacer {
    fn default() -> Self {
        Self::new()
    }
}
