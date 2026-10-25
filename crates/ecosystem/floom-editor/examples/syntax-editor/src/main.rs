//! The editor with syntax highlighting.
//!
//! The v1 example used `syntect` to highlight Rust. This one uses a small
//! hand-written scanner instead, because what the example is here to show is
//! the *port's* `Styling` trait: a styling that answers with the ranges of a
//! line that differ from the base style, and the editor turning those ranges
//! into ink.
//!
//! A real highlighter would implement the same trait — the ranges are the
//! whole contract.
//!
//! Run with `cargo run -p syntax-editor`.

use std::sync::Arc;

use floom::prelude::*;
use floom_editor::{EditorStyle, Styling, TextEditor, text_editor};
use floom_types::{Color, FontFamily, FontWeight, TextSpan, TextStyle};

/// The keywords the scanner highlights.
const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "if", "else", "match", "for", "while", "loop", "struct", "enum", "impl",
    "trait", "use", "mod", "pub", "return", "break", "continue", "const", "static", "type",
    "where", "in", "as", "ref", "self", "Some", "None", "Ok", "Err",
];

/// A styling that colours the keywords and the string literals.
struct RustStyling {
    keyword: Color,
    string: Color,
    number: Color,
}

impl Default for RustStyling {
    fn default() -> Self {
        Self {
            keyword: Color::rgb(0xCC, 0x78, 0xDD),
            string: Color::rgb(0x6A, 0xB2, 0x6A),
            number: Color::rgb(0xD1, 0x9A, 0x66),
        }
    }
}

impl Styling for RustStyling {
    fn id(&self) -> u64 {
        1
    }

    fn font_size(&self) -> f32 {
        15.0
    }

    fn line_height(&self) -> f32 {
        22.0
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
        let bytes = text.as_bytes();
        let mut index = 0usize;
        while index < bytes.len() {
            let c = bytes[index] as char;
            if c == '"' {
                // A string literal: to the closing quote, or to the end of the
                // line if the string is not closed.
                let start = index;
                index += 1;
                while index < bytes.len() && bytes[index] != b'"' {
                    if bytes[index] == b'\\' {
                        index += 1;
                    }
                    index += 1;
                }
                index = (index + 1).min(bytes.len());
                spans.push(TextSpan {
                    range: start..index,
                    style: TextStyle {
                        color: self.string,
                        ..base
                    },
                });
                continue;
            }
            if c.is_ascii_alphabetic() || c == '_' {
                let start = index;
                while index < bytes.len()
                    && ((bytes[index] as char).is_ascii_alphanumeric() || bytes[index] == b'_')
                {
                    index += 1;
                }
                let word = &text[start..index];
                if KEYWORDS.contains(&word) {
                    spans.push(TextSpan {
                        range: start..index,
                        style: TextStyle {
                            color: self.keyword,
                            ..base
                        },
                    });
                }
                continue;
            }
            if c.is_ascii_digit() {
                let start = index;
                while index < bytes.len() && (bytes[index] as char).is_ascii_digit() {
                    index += 1;
                }
                spans.push(TextSpan {
                    range: start..index,
                    style: TextStyle {
                        color: self.number,
                        ..base
                    },
                });
                continue;
            }
            index += 1;
        }
        spans
    }
}

fn main() {
    let text = "fn main() {\n    \
                let mut total = 0;\n    \
                for i in 0..10 {\n        \
                total += i * 2;\n    \
                }\n    \
                println!(\"total is {}\", total);\n\
                }\n";

    let editor: TextEditor = text_editor(text.to_string())
        .style(EditorStyle::dark())
        .styling(Arc::new(RustStyling::default()));

    let result = floom::run_desktop("floom — syntax editor", Size::new(720.0, 480.0), {
        let editor = editor.clone();
        move |host: &mut WidgetHost| {
            let root = Stack::new(Axis::Vertical).gap(8.0).child(Child::new(
                ChildKey::index(0),
                LayoutRequest::new().fill(),
                {
                    let editor = editor.clone();
                    move || Box::new(editor.clone())
                },
            ));
            host.mount_root(LayoutStyle::default(), root);
        }
    });

    if let Err(err) = result {
        eprintln!("desktop loop failed: {err}");
        std::process::exit(1);
    }
}
