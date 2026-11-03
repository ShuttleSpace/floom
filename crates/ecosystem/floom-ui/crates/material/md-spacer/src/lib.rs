
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
