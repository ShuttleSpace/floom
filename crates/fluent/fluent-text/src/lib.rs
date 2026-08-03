use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::props::text::TextProps;
use floem_core_theme::theme::{FluentTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::TextStyle;

const DESIGN_SYSTEM: &str = "fluent";

#[derive(Debug, Clone, PartialEq)]
pub struct FluentTextStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub typography: TextStyle,
}

pub struct FluentText {
    props: TextProps,
    theme: RwSignal<FluentTheme>,
}

impl FluentText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
            theme: RwSignal::new(FluentTheme::new()),
        }
    }
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.props.max_lines = Some(max_lines);
        self
    }
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.props.is_selectable = selectable;
        self
    }
    pub fn props(&self) -> &TextProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<FluentTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: FluentTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentTextStyle {
        let theme = self.theme.get();
        let typography = theme.typography().body.medium.clone();
        FluentTextStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().on_surface,
            font_size: typography.font_size,
            line_height: typography.line_height,
            typography,
        }
    }
}
