//! The editor's appearance.
//!
//! 2.0 does not have v1's `Style` / `prop!` / `prop_extractor!` machinery, and
//! the house answer — the one `floom-ui` settled on — is a plain struct that
//! the caller fills in. So the editor's style is a plain struct too, with a
//! `Default` that is the light theme and a `dark` constructor beside it.

use floom_types::{Color, FontFamily, FontWeight};

/// The editor's appearance.
///
/// Every colour the editor paints is named here, so a caller can restyle it
/// without reaching into the widget. The values v1 read out of a `Style` — the
/// text colour, the cursor, the selection, the current line, the gutter's two
/// inks — are all here; the ones v1 read out of a *style property* that the
/// framework owned (the font size, the line height, the tab width) are here
/// too, because in 2.0 there is no framework to own them.
#[derive(Debug, Clone, PartialEq)]
pub struct EditorStyle {
    /// The ink of the text.
    pub text_color: Color,
    /// The editor's background.
    pub background: Color,
    /// The caret's ink.
    pub cursor_color: Color,
    /// The selection's background.
    pub selection_color: Color,
    /// The current line's background.
    pub current_line_color: Color,
    /// The line numbers' ink, for every line but the current one.
    pub gutter_dim_color: Color,
    /// The current line's number ink.
    pub gutter_accent_color: Color,
    /// The gutter's background.
    pub gutter_background: Color,
    /// The size of the text, in logical pixels.
    pub font_size: f32,
    /// The height of one line, in logical pixels.
    pub line_height: f32,
    /// The width of the caret, in logical pixels.
    pub caret_width: f32,
    /// How wide a tab is, in columns.
    pub tab_width: usize,
    /// The typeface the text is set in.
    pub family: FontFamily,
    /// The weight the text is drawn at.
    pub weight: FontWeight,
    /// The width of the gutter, in logical pixels.
    pub gutter_width: f64,
    /// The gap between the gutter and the text, in logical pixels.
    pub gutter_padding: f64,
}

impl Default for EditorStyle {
    /// The light theme.
    ///
    /// The colours are v1's `default_light_theme`, unchanged: a port that
    /// restyled the editor would be a port that changed what the editor looks
    /// like, and the colours are not what is being ported.
    fn default() -> Self {
        Self {
            text_color: Color::rgb(0x38, 0x3A, 0x42),
            background: Color::rgb(0xFA, 0xFA, 0xFA),
            cursor_color: Color::rgb(0x52, 0x6F, 0xFF),
            selection_color: Color::rgb(0xE5, 0xE5, 0xE6),
            current_line_color: Color::rgb(0xF2, 0xF2, 0xF2),
            gutter_dim_color: Color::rgb(0xA0, 0xA1, 0xA7),
            gutter_accent_color: Color::rgb(0x38, 0x3A, 0x42),
            gutter_background: Color::rgb(0xFA, 0xFA, 0xFA),
            font_size: 16.0,
            line_height: 24.0,
            caret_width: 2.0,
            tab_width: 4,
            family: FontFamily::Monospace,
            weight: FontWeight::Normal,
            gutter_width: 56.0,
            gutter_padding: 12.0,
        }
    }
}

impl EditorStyle {
    /// The dark theme: v1's `default_dark_color`.
    pub fn dark() -> Self {
        Self {
            text_color: Color::rgb(0xAB, 0xB2, 0xBF),
            background: Color::rgb(0x28, 0x2C, 0x34),
            cursor_color: Color::rgb(0x52, 0x8B, 0xFF),
            selection_color: Color::rgb(0x3E, 0x44, 0x51),
            current_line_color: Color::rgb(0x2C, 0x31, 0x3C),
            gutter_dim_color: Color::rgb(0x5C, 0x63, 0x70),
            gutter_accent_color: Color::rgb(0xAB, 0xB2, 0xBF),
            gutter_background: Color::rgb(0x28, 0x2C, 0x34),
            ..Self::default()
        }
    }
}
