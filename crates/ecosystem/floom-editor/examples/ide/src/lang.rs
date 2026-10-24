//! Language detection, and the line scanner that colours a file.
//!
//! The editor's [`Styling`] contract is per line: given one line's text, answer
//! the byte ranges that differ from the base style. That is enough for a
//! highlighter that does not carry state between lines — no multi-line strings,
//! no block comments that outlive their line — which is the honest limit of
//! this demo's scanner. A real editor would keep a scanner state per document;
//! the point here is the port's `Styling` seam, not a lexer library.

use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering};

use floom_editor::Styling;
use floom_types::{Color, FontFamily, FontWeight, TextSpan, TextStyle};

use crate::theme;

/// What a file is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Markdown,
    Toml,
    Text,
}

impl Lang {
    /// The language a path names, by extension.
    pub fn from_path(path: &Path) -> Self {
        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str()
        {
            "rs" => Self::Rust,
            "md" | "markdown" => Self::Markdown,
            "toml" => Self::Toml,
            _ => Self::Text,
        }
    }

    /// The name the status bar shows.
    pub fn label(self) -> &'static str {
        match self {
            Self::Rust => "Rust",
            Self::Markdown => "Markdown",
            Self::Toml => "TOML",
            Self::Text => "Plain Text",
        }
    }

    /// The colour of the file's dot in the explorer.
    pub fn color(self) -> Color {
        match self {
            Self::Text => theme::DIM,
            other => theme::language_color(other.label()),
        }
    }

    fn index(self) -> u8 {
        match self {
            Self::Rust => 0,
            Self::Markdown => 1,
            Self::Toml => 2,
            Self::Text => 3,
        }
    }

    fn from_index(index: u8) -> Self {
        match index {
            0 => Self::Rust,
            1 => Self::Markdown,
            2 => Self::Toml,
            _ => Self::Text,
        }
    }
}

/// The styling the editor holds, with the active file's language behind an
/// atomic because the editor asks for lines from inside its own paint.
///
/// An atomic rather than a mutex: the language changes when a tab is clicked,
/// and every line of the newly active file is about to read it. A lock taken
/// per line would serialise the paint against the tab strip for no reason.
pub struct Syntax {
    lang: AtomicU8,
}

impl Syntax {
    pub fn new(lang: Lang) -> Self {
        Self {
            lang: AtomicU8::new(lang.index()),
        }
    }

    pub fn set(&self, lang: Lang) {
        self.lang.store(lang.index(), Ordering::Relaxed);
    }

    pub fn get(&self) -> Lang {
        Lang::from_index(self.lang.load(Ordering::Relaxed))
    }
}

impl Styling for Syntax {
    fn id(&self) -> u64 {
        // The trait asks a styling that changes to change its id; the editor's
        // own cache is keyed on the document, but a cache that trusts this
        // contract must see the language move.
        0x1DE_0000 + u64::from(self.get().index())
    }

    fn font_size(&self) -> f32 {
        theme::FONT_SIZE
    }

    fn line_height(&self) -> f32 {
        theme::LINE_HEIGHT
    }

    fn font_family(&self) -> FontFamily {
        FontFamily::Monospace
    }

    fn weight(&self) -> FontWeight {
        FontWeight::Normal
    }

    fn tab_width(&self) -> usize {
        4
    }

    fn style_line(&self, _line: usize, text: &str, base: TextStyle) -> Vec<TextSpan> {
        let mut spans = Vec::new();
        match self.get() {
            Lang::Rust => rust(text, base, &mut spans),
            Lang::Markdown => markdown(text, base, &mut spans),
            Lang::Toml => toml(text, base, &mut spans),
            Lang::Text => {}
        }
        spans
    }
}

/// A span of `range` in `color`, at `weight`, on top of the base style.
fn span(
    out: &mut Vec<TextSpan>,
    base: TextStyle,
    range: std::ops::Range<usize>,
    color: Color,
    weight: FontWeight,
) {
    if range.start < range.end {
        out.push(TextSpan {
            range,
            style: TextStyle {
                color,
                weight,
                ..base
            },
        });
    }
}

fn char_len(text: &str, index: usize) -> usize {
    text[index..].chars().next().map(char::len_utf8).unwrap_or(1)
}

// --- Rust -------------------------------------------------------------------

const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
    "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
    "use", "where", "while",
];

fn rust(text: &str, base: TextStyle, out: &mut Vec<TextSpan>) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                span(out, base, i..len, theme::COMMENT, base.weight);
                return;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                let mut j = i + 2;
                while j + 1 < len && !(bytes[j] == b'*' && bytes[j + 1] == b'/') {
                    j += 1;
                }
                let end = (j + 2).min(len);
                span(out, base, i..end, theme::COMMENT, base.weight);
                i = end;
            }
            b'"' => {
                let mut j = i + 1;
                while j < len {
                    match bytes[j] {
                        b'\\' => j += 2,
                        b'"' => {
                            j += 1;
                            break;
                        }
                        _ => j += 1,
                    }
                }
                let end = j.min(len);
                span(out, base, i..end, theme::STRING, base.weight);
                i = end;
            }
            b'\'' => {
                if let Some(end) = char_literal_end(bytes, i) {
                    span(out, base, i..end, theme::STRING, base.weight);
                    i = end;
                } else {
                    let mut j = i + 1;
                    while j < len && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                        j += 1;
                    }
                    span(out, base, i..j, theme::TYPE, base.weight);
                    i = j;
                }
            }
            b'#' if bytes.get(i + 1) == Some(&b'[') => {
                let mut j = i + 2;
                let mut depth = 1usize;
                while j < len {
                    match bytes[j] {
                        b'[' => depth += 1,
                        b']' => {
                            depth -= 1;
                            if depth == 0 {
                                j += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let end = j.min(len);
                span(out, base, i..end, theme::ATTRIBUTE, base.weight);
                i = end;
            }
            b'0'..=b'9' => {
                let mut j = i;
                while j < len
                    && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_' || bytes[j] == b'.')
                {
                    j += 1;
                }
                span(out, base, i..j, theme::NUMBER, base.weight);
                i = j;
            }
            b if b.is_ascii_alphabetic() || b == b'_' => {
                let mut j = i;
                while j < len && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                    j += 1;
                }
                let word = &text[i..j];
                let mut next = j;
                while next < len && bytes[next] == b' ' {
                    next += 1;
                }
                let color = if RUST_KEYWORDS.contains(&word) {
                    Some(theme::KEYWORD)
                } else if bytes.get(next) == Some(&b'!') {
                    Some(theme::MACRO)
                } else if bytes.get(next) == Some(&b'(') {
                    Some(theme::FUNCTION)
                } else if word.starts_with(|c: char| c.is_ascii_uppercase()) {
                    Some(theme::TYPE)
                } else {
                    None
                };
                if let Some(color) = color {
                    span(out, base, i..j, color, base.weight);
                }
                i = j;
            }
            _ => i += char_len(text, i),
        }
    }
}

/// The end of a character literal at `i`, or `None` when the quote opens a
/// lifetime instead.
fn char_literal_end(bytes: &[u8], i: usize) -> Option<usize> {
    let mut j = i + 1;
    while j < bytes.len() && j - i <= 10 {
        match bytes[j] {
            b'\'' => return Some(j + 1),
            b'\\' => j += 2,
            _ => j += 1,
        }
    }
    None
}

// --- Markdown ---------------------------------------------------------------

fn markdown(text: &str, base: TextStyle, out: &mut Vec<TextSpan>) {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return;
    }
    let lead = text.len() - trimmed.len();

    if trimmed.starts_with('#') {
        let level = trimmed.chars().take_while(|c| *c == '#').count();
        if level <= 6 && trimmed.as_bytes().get(level) == Some(&b' ') {
            span(
                out,
                base,
                0..text.len(),
                theme::BRIGHT,
                FontWeight::Bold,
            );
            return;
        }
    }
    if trimmed.starts_with("```") {
        span(out, base, lead..text.len(), theme::DIM, base.weight);
        return;
    }
    if let Some(marker) = ["- ", "* ", "> "]
        .iter()
        .find(|marker| trimmed.starts_with(**marker))
    {
        span(
            out,
            base,
            lead..lead + marker.len(),
            theme::KEYWORD,
            FontWeight::Bold,
        );
    }
    if trimmed.starts_with('|') {
        for (index, byte) in text.bytes().enumerate() {
            if byte == b'|' {
                span(out, base, index..index + 1, theme::DIMMER, base.weight);
            }
        }
    }

    let bytes = text.as_bytes();
    let mut i = lead;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                if let Some(close) = text[i + 1..].find('`') {
                    let end = i + 1 + close + 1;
                    span(out, base, i..end, theme::STRING, base.weight);
                    i = end;
                } else {
                    i += 1;
                }
            }
            b'*' if bytes.get(i + 1) == Some(&b'*') => {
                if let Some(close) = text[i + 2..].find("**") {
                    let end = i + 2 + close + 2;
                    span(out, base, i..end, theme::BRIGHT, FontWeight::Bold);
                    i = end;
                } else {
                    i += 2;
                }
            }
            b'[' => {
                if let Some(close) = text[i..].find("](") {
                    let label_end = i + close;
                    if let Some(paren) = text[label_end..].find(')') {
                        let end = label_end + paren + 1;
                        span(out, base, i..label_end, theme::ACCENT, base.weight);
                        span(
                            out,
                            base,
                            label_end..end,
                            theme::DIMMER,
                            base.weight,
                        );
                        i = end;
                        continue;
                    }
                }
                i += 1;
            }
            _ => i += char_len(text, i),
        }
    }
}

// --- TOML -------------------------------------------------------------------

fn toml(text: &str, base: TextStyle, out: &mut Vec<TextSpan>) {
    let trimmed = text.trim_start();
    if trimmed.is_empty() {
        return;
    }
    let lead = text.len() - trimmed.len();

    if trimmed.starts_with('#') {
        span(out, base, lead..text.len(), theme::COMMENT, base.weight);
        return;
    }
    if trimmed.starts_with('[') {
        if let Some(close) = trimmed.find(']') {
            span(
                out,
                base,
                lead..lead + close + 1,
                theme::TYPE,
                FontWeight::Medium,
            );
        }
    }
    let Some(equals) = text.find('=') else {
        return;
    };
    let key = text[..equals].trim_end();
    let key_start = text.len() - text.trim_start().len();
    let _ = key_start;
    if !key.trim().is_empty() {
        span(
            out,
            base,
            0..key.len(),
            theme::FUNCTION,
            base.weight,
        );
    }
    span(
        out,
        base,
        equals..equals + 1,
        theme::DIMMER,
        base.weight,
    );

    let value = &text[equals + 1..];
    let value_lead = value.len() - value.trim_start().len();
    let value_start = equals + 1 + value_lead;
    let value = value.trim_start();
    let value_bytes = value.as_bytes();
    if value.starts_with('"') || value.starts_with('\'') {
        let quote = value_bytes[0];
        let mut j = 1;
        while j < value_bytes.len() {
            if value_bytes[j] == b'\\' && quote == b'"' {
                j += 2;
                continue;
            }
            if value_bytes[j] == quote {
                j += 1;
                break;
            }
            j += 1;
        }
        span(
            out,
            base,
            value_start..value_start + j,
            theme::STRING,
            base.weight,
        );
    } else if value.starts_with(|c: char| c.is_ascii_digit()) {
        let mut j = 0;
        while j < value_bytes.len()
            && (value_bytes[j].is_ascii_alphanumeric()
                || value_bytes[j] == b'_'
                || value_bytes[j] == b'.')
        {
            j += 1;
        }
        span(
            out,
            base,
            value_start..value_start + j,
            theme::NUMBER,
            base.weight,
        );
    } else if value.starts_with("true") || value.starts_with("false") {
        let len = if value.starts_with("true") { 4 } else { 5 };
        span(
            out,
            base,
            value_start..value_start + len,
            theme::KEYWORD,
            base.weight,
        );
    }
    if let Some(comment) = value.find('#') {
        span(
            out,
            base,
            value_start + comment..text.len(),
            theme::COMMENT,
            base.weight,
        );
    }
}
