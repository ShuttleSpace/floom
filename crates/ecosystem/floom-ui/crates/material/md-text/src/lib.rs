//! Material text, ported to the Floom 2.0 widget API.
//!
//! v1 drew a `Label` and styled it with a closure that re-read the theme on
//! every use. 2.0's [`Text`] is a widget the caller holds and restyles, so the
//! theme is resolved once and handed over as explicit values.
//!
//! ## What the port names
//!
//! * **`max_lines` and `selectable` are carried but not applied.** v1 set both
//!   on the props and neither reached the view -- it drew a plain `Label` -- and
//!   2.0's `Text` has no knob for either. So the props keep reporting them and
//!   no kit claims a truncation or a selection it does not perform.
//! * **the label's type is the theme's**: `typography().body.medium`'s size and
//!   weight, and `on_surface` for the colour.

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
            // The app-wide signal, as v1's was, so the text re-themes with the
            // app. Cupertino and Fluent held a snapshot; that difference is
            // v1's and is kept.
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

/// The theme's weight enum -> the engine's.
///
/// v1 called `to_floom()`, which returns a `parley::style::FontWeight` -- a third
/// type with the same name. 2.0's `Text::weight` takes the engine's own
/// `FontWeight`, so the nine variants are named here.
pub fn to_engine_weight(w: ThemeFontWeight) -> FontWeight {
    match w {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        // v1's name for 400 is `NORMAL`; the theme's is `Regular`.
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        // v1's `to_floom()` sent `Black` to 800; parley's table has no 900.
        ThemeFontWeight::Black => FontWeight::Black,
    }
}
