use floem::prelude::*;
use floem_core_theme::props::spacer::SpacerProps;
use floem_core_theme::theme::MultiTheme;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentSpacerStyle {
    pub grow: f32,
}

pub struct FluentSpacer {
    props: SpacerProps,
    theme: RwSignal<MultiTheme>,
}

impl FluentSpacer {
    pub fn new() -> Self {
        Self {
            props: SpacerProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Fluent)),
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

    pub fn resolved_style(&self) -> FluentSpacerStyle {
        FluentSpacerStyle {
            grow: self.props.grow,
        }
    }

    /// Build an empty, flex-growing view that pushes siblings apart.
    pub fn view(self) -> impl View {
        let grow = self.props.grow;
        Container::new(Empty::new()).style(move |s| s.flex_grow(grow))
    }
}

impl Default for FluentSpacer {
    fn default() -> Self {
        Self::new()
    }
}
