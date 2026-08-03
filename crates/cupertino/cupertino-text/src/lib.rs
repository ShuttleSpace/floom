use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::props::text::TextProps;
use floem_core_theme::theme::{CupertinoTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::TextStyle;

const DESIGN_SYSTEM: &str = "cupertino";

#[derive(Debug, Clone, PartialEq)]
pub struct CupertinoTextStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub typography: TextStyle,
}

pub struct CupertinoText {
    props: TextProps,
    theme: RwSignal<CupertinoTheme>,
}

impl CupertinoText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
            theme: RwSignal::new(CupertinoTheme::new()),
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

    pub fn theme(&self) -> RwSignal<CupertinoTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: CupertinoTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> CupertinoTextStyle {
        let theme = self.theme.get();
        let typography = theme.typography().body.medium.clone();
        CupertinoTextStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().on_surface,
            font_size: typography.font_size,
            line_height: typography.line_height,
            typography,
        }
    }
}
