use floem::prelude::*;
use floem::reactive::{RwSignal, SignalGet};
use floem_core_theme::props::text::TextProps;
use floem_core_theme::theme::{MultiTheme, Theme};
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
    theme: RwSignal<MultiTheme>,
}

impl CupertinoText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
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

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
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

    /// Build a renderable Cupertino text view.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let content = self.props.content.clone();
        text(content).style(move |s| {
            let t = theme.get();
            let ts = t.typography().body.medium.clone();
            s.color(t.colors().on_surface.to_peniko())
                .font_size(ts.font_size)
                .font_weight(ts.font_weight.to_floem())
        })
    }
}
