//! How the text is set: the size, the face, and the ranges that differ.
//!
//! v1's `Styling` trait answered per *line*, because a document could set one
//! line in a different face from the next. This one does too: the trait's
//! `style_line` is handed the line's number and the line's own text, and
//! answers the ranges that differ from the base style.
//!
//! What is *not* per line is the line's height. Every row is the same height,
//! because the scroll position counts rows and a row whose height depended on
//! its content would be a row the scroll could not name. The height is the
//! trait's `line_height`, and the editor tells the engine what it is so the
//! engine's rows and the editor's rows are the same rows.

use floom_types::{FontFamily, FontWeight, TextSpan, TextStyle};

/// How the editor's text is set.
///
/// The default implementation is a plain 16pt sans-serif document with no
/// highlighted ranges, which is a text editor with the highlighting turned
/// off — the state v1's `SimpleStyling` was in before anything asked for
/// more.
pub trait Styling: Send + Sync {
    /// An id for the styling.
    ///
    /// The id is what a cache keys on, so a styling that changes has to
    /// change it. `SimpleStyling` bumps it on every setter for exactly this
    /// reason.
    fn id(&self) -> u64;

    /// The size of the text, in logical pixels.
    fn font_size(&self) -> f32 {
        16.0
    }

    /// The height of one line, in logical pixels.
    fn line_height(&self) -> f32 {
        (1.5 * self.font_size()).round().max(self.font_size())
    }

    /// The typeface the text is set in.
    fn font_family(&self) -> FontFamily {
        FontFamily::SansSerif
    }

    /// The weight the text is drawn at.
    fn weight(&self) -> FontWeight {
        FontWeight::Normal
    }

    /// How wide a tab is, in columns.
    fn tab_width(&self) -> usize {
        4
    }

    /// The ranges of `text` that differ from `base`.
    ///
    /// The ranges are byte ranges into `text`, which is the line's own text
    /// without its line ending. Returning nothing means the line is drawn in
    /// the base style throughout.
    fn style_line(&self, _line: usize, _text: &str, _base: TextStyle) -> Vec<TextSpan> {
        Vec::new()
    }
}

/// The plain styling: one size, one face, no highlighted ranges.
///
/// This is v1's `SimpleStyling` with the setters kept.
#[derive(Debug, Clone, PartialEq)]
pub struct SimpleStyling {
    id: u64,
    font_size: f32,
    line_height: Option<f32>,
    font_family: FontFamily,
    weight: FontWeight,
    tab_width: usize,
}

impl Default for SimpleStyling {
    fn default() -> Self {
        Self {
            id: 0,
            font_size: 16.0,
            line_height: None,
            font_family: FontFamily::SansSerif,
            weight: FontWeight::Normal,
            tab_width: 4,
        }
    }
}

impl SimpleStyling {
    /// A styling with the default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// The styling's cache id.
    ///
    /// Bumped by every setter, so a cache keyed on it notices a change.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Sets the size of the text.
    pub fn set_font_size(&mut self, font_size: f32) -> &mut Self {
        self.font_size = font_size;
        self.id += 1;
        self
    }

    /// Sets the height of one line.
    ///
    /// `None` means "1.5 times the font size", which is what v1's
    /// `SimpleStyling` did when its line height was left alone.
    pub fn set_line_height(&mut self, line_height: Option<f32>) -> &mut Self {
        self.line_height = line_height;
        self.id += 1;
        self
    }

    /// Sets the typeface.
    pub fn set_font_family(&mut self, family: FontFamily) -> &mut Self {
        self.font_family = family;
        self.id += 1;
        self
    }

    /// Sets the weight.
    pub fn set_weight(&mut self, weight: FontWeight) -> &mut Self {
        self.weight = weight;
        self.id += 1;
        self
    }

    /// Sets how wide a tab is, in columns.
    pub fn set_tab_width(&mut self, tab_width: usize) -> &mut Self {
        self.tab_width = tab_width;
        self.id += 1;
        self
    }
}

impl Styling for SimpleStyling {
    fn id(&self) -> u64 {
        self.id
    }

    fn font_size(&self) -> f32 {
        self.font_size
    }

    fn line_height(&self) -> f32 {
        match self.line_height {
            Some(height) => height.max(self.font_size),
            None => (1.5 * self.font_size).round().max(self.font_size),
        }
    }

    fn font_family(&self) -> FontFamily {
        self.font_family.clone()
    }

    fn weight(&self) -> FontWeight {
        self.weight
    }

    fn tab_width(&self) -> usize {
        self.tab_width
    }
}
