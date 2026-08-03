use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::props::text::TextProps;
use floem_core_theme::theme::Theme;
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::TextStyle;
use floem_widget_runtime::theme_signal;

#[derive(Debug, Clone, PartialEq)]
pub struct MdTextStyle {
    pub color: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub typography: TextStyle,
}

pub struct MdText {
    props: TextProps,
    theme: RwSignal<floem_core_theme::theme::MaterialTheme>,
}

impl MdText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
            theme: theme_signal(),
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

    pub fn theme(&self) -> RwSignal<floem_core_theme::theme::MaterialTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: floem_core_theme::theme::MaterialTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MdTextStyle {
        let theme = self.theme.get();
        let typography = theme.typography().body.medium.clone();
        MdTextStyle {
            color: theme.colors().on_surface,
            font_size: typography.font_size,
            line_height: typography.line_height,
            typography,
        }
    }
}
