//! The demo's palette and the editor style it hands to the widget.
//!
//! One place, so the chrome and the text agree. The editor's own defaults are
//! a light theme copied from v1; a demo that is meant to look like an IDE sets
//! every colour the editor paints rather than inheriting half of them.

use floom_editor::EditorStyle;
use floom_types::{Color, FontFamily, FontWeight};

// --- surfaces ---------------------------------------------------------------

/// The editor's background, and the active tab's.
pub const EDITOR_BG: Color = Color::rgb(0x1E, 0x21, 0x27);
/// The explorer, the tab strip and the status bar.
pub const CHROME_BG: Color = Color::rgb(0x17, 0x19, 0x1E);
/// A slightly lighter chrome, for hovered rows.
pub const CHROME_HOVER: Color = Color::rgb(0x22, 0x25, 0x2C);
/// The selected row, and an inactive tab.
pub const CHROME_ACTIVE: Color = Color::rgb(0x2A, 0x2E, 0x37);
/// A one-pixel rule between regions.
///
/// Bright enough to read against `CHROME_BG` — a rule that is nearly the
/// colour of the background on both sides of it is a rule nobody can see,
/// which is the same as not having drawn one.
pub const BORDER: Color = Color::rgb(0x39, 0x3F, 0x4B);
/// The current line's band inside the editor.
pub const CURRENT_LINE: Color = Color::rgb(0x23, 0x26, 0x2D);

// --- ink --------------------------------------------------------------------

/// Ordinary text.
pub const TEXT: Color = Color::rgb(0xAB, 0xB2, 0xBF);
/// The brightest text: the active tab, a heading.
pub const BRIGHT: Color = Color::rgb(0xD7, 0xDA, 0xE0);
/// Line numbers, hints, inactive tabs.
pub const DIM: Color = Color::rgb(0x5C, 0x63, 0x70);
/// A slightly darker dim, for separators and the status bar's quiet half.
pub const DIMMER: Color = Color::rgb(0x4B, 0x52, 0x63);

/// The accent: the caret, the active row's bar, links.
pub const ACCENT: Color = Color::rgb(0x61, 0xAF, 0xEF);

// --- syntax -----------------------------------------------------------------

pub const KEYWORD: Color = Color::rgb(0xC6, 0x78, 0xDD);
pub const STRING: Color = Color::rgb(0x98, 0xC3, 0x79);
pub const NUMBER: Color = Color::rgb(0xD1, 0x9A, 0x66);
pub const FUNCTION: Color = Color::rgb(0x61, 0xAF, 0xEF);
pub const TYPE: Color = Color::rgb(0xE5, 0xC0, 0x7B);
pub const COMMENT: Color = Color::rgb(0x5C, 0x63, 0x70);
pub const MACRO: Color = Color::rgb(0x56, 0xB6, 0xC2);
pub const ATTRIBUTE: Color = Color::rgb(0xE5, 0xC0, 0x7B);

// --- metrics ----------------------------------------------------------------

/// The editor's text size, in logical pixels.
pub const FONT_SIZE: f32 = 14.5;
/// One row, in logical pixels.
pub const LINE_HEIGHT: f32 = 22.0;
/// Chrome text.
pub const UI_SIZE: f32 = 12.5;
/// The explorer's section label and the status bar's quiet half.
pub const SMALL: f32 = 10.5;

/// The editor's appearance.
///
/// Soft wrap and indent guides on, whitespace markers off: the two the demo
/// wants to show, and the one that would clutter source that already indents.
pub fn editor_style() -> EditorStyle {
    EditorStyle {
        text_color: TEXT,
        background: EDITOR_BG,
        cursor_color: ACCENT,
        selection_color: Color::rgb(0x3E, 0x44, 0x51),
        current_line_color: CURRENT_LINE,
        gutter_dim_color: DIMMER,
        gutter_accent_color: TEXT,
        gutter_background: EDITOR_BG,
        font_size: FONT_SIZE,
        line_height: LINE_HEIGHT,
        caret_width: 2.0,
        tab_width: 4,
        family: FontFamily::Monospace,
        weight: FontWeight::Normal,
        gutter_width: 52.0,
        gutter_padding: 12.0,
        soft_wrap: true,
        show_whitespace: false,
        whitespace_color: Color::rgb(0x3A, 0x3F, 0x4B),
        show_indent_guides: true,
        indent_guide_color: Color::rgb(0x2A, 0x2E, 0x36),
    }
}

/// Each language's colour, worn by its file's dot in the explorer.
pub fn language_color(label: &str) -> Color {
    match label {
        "Rust" => NUMBER,
        "Markdown" => FUNCTION,
        "TOML" => TYPE,
        _ => DIM,
    }
}
