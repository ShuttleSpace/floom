use floem::prelude::*;
use floem_core_theme::props::spacer::SpacerProps;
use floem_core_theme::theme::MultiTheme;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MdSpacerStyle {
    pub grow: f32,
}

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

    pub fn resolved_style(&self) -> MdSpacerStyle {
        MdSpacerStyle {
            grow: self.props.grow,
        }
    }

    /// Build an empty, flex-growing view that pushes siblings apart.
    pub fn view(self) -> impl View {
        let grow = self.props.grow;
        Container::new(Empty::new()).style(move |s| s.flex_grow(grow))
    }
}

impl Default for MdSpacer {
    fn default() -> Self {
        Self::new()
    }
}
