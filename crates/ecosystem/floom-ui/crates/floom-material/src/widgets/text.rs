use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};

use floom_widget_runtime::theme_signal;
use floom_widgets::Text;

use floom_core_theme::props::text::TextProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::typography::{FontWeight as ThemeFontWeight, TextStyle};

const DESIGN_SYSTEM: &str = "material";

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialTextStyle {
    pub design_system: &'static str,
    pub color: Color,
    pub font_size: f32,
    pub line_height: f32,
    pub typography: TextStyle,
}

/// Material's text.
pub struct MaterialText {
    props: TextProps,
    theme: RwSignal<MultiTheme>,
}

impl MaterialText {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            props: TextProps {
                content: content.into(),
                ..Default::default()
            },
            theme: theme_signal(),
        }
    }

    /// Truncate to this many lines. Carried on the props, as v1 carried it;
    /// see the module note.
    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.props.max_lines = Some(max_lines);
        self
    }

    /// Make the text selectable. Carried on the props, as v1 carried it; see
    /// the module note.
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

    pub fn resolved_style(&self) -> MaterialTextStyle {
        let theme = self.theme.get();
        let typography = theme.typography().body.medium.clone();
        MaterialTextStyle {
            design_system: DESIGN_SYSTEM,
            color: theme.colors().on_surface,
            font_size: typography.font_size,
            line_height: typography.line_height,
            typography,
        }
    }

    /// The text, ready to mount.
    pub fn view(self) -> Text {
        let style = self.resolved_style();
        let content = self.props.content.clone();
        Text::new(content)
            .size(style.font_size)
            .weight(to_engine_weight(style.typography.font_weight))
            .color(to_paint(style.color))
    }
}

impl Default for MaterialText {
    fn default() -> Self {
        Self::new(String::new())
    }
}

pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
