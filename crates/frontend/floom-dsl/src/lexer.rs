// SPDX-License-Identifier: MIT
//! Hand-written lexer: source text to spanned tokens.

use std::fmt;

use crate::ui_ir::Rgba;

use crate::DslError;

/// A position in the source, 1-based on both axes (what an editor shows).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Span {
    pub line: u32,
    pub column: u32,
}

impl Span {
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }

    /// The very beginning, for diagnostics about the document as a whole.
    pub const START: Self = Self::new(1, 1);
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A lexical token.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Ident(String),
    Text(String),
    Number(f64),
    Color(Rgba),
    Bool(bool),
    /// `@name`
    Ref(String),
    Eq,
    Colon,
    LBrace,
    RBrace,
}

impl Token {
    /// How a token reads in an error message.
    pub fn describe(&self) -> String {
        match self {
            Self::Ident(name) => format!("`{name}`"),
            Self::Text(text) => format!("the string \"{text}\""),
            Self::Number(n) => format!("the number {n}"),
            Self::Color(c) => format!("the colour {}", c.to_hex()),
            Self::Bool(b) => format!("`{b}`"),
            Self::Ref(name) => format!("`@{name}`"),
            Self::Eq => "`=`".into(),
            Self::Colon => "`:`".into(),
            Self::LBrace => "`{`".into(),
            Self::RBrace => "`}`".into(),
        }
    }
}

/// A token and where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub span: Span,
    pub token: Token,
}

struct Lexer<'a> {
    chars: Vec<char>,
    index: usize,
    line: u32,
    column: u32,
    source: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            index: 0,
            line: 1,
            column: 1,
            source,
        }
    }

    fn span(&self) -> Span {
        Span::new(self.line, self.column)
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.index).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.index + offset).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.index).copied()?;
        self.index += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Skip whitespace and comments, and note whether anything ran to the end.
    fn skip_trivia(&mut self) -> Result<(), DslError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.span();
                    self.bump();
                    self.bump();
                    loop {
                        match (self.peek(), self.peek_at(1)) {
                            (Some('*'), Some('/')) => {
                                self.bump();
                                self.bump();
                                break;
                            }
                            (Some(_), _) => {
                                self.bump();
                            }
                            (None, _) => {
                                return Err(DslError::Syntax {
                                    span: start,
                                    message: "unterminated block comment".into(),
                                });
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// A word: identifiers, keywords and multi-segment node keys.
    ///
    /// A `-` continues the word only when a letter follows, so
    /// `app.save-button` is one key while `gap=8-4` lexes the `-4` as a number
    /// rather than silently swallowing it.
    fn word(&mut self) -> String {
        let mut out = String::new();
        while let Some(c) = self.peek() {
            let continues = c.is_ascii_alphanumeric()
                || c == '_'
                || c == '.'
                || (c == '-'
                    && self
                        .peek_at(1)
                        .is_some_and(|n| n.is_ascii_alphabetic() || n == '_'));
            if !continues {
                break;
            }
            out.push(c);
            self.bump();
        }
        out
    }

    fn number(&mut self) -> Result<f64, DslError> {
        let start = self.span();
        let mut text = String::new();
        if self.peek() == Some('-') {
            text.push('-');
            self.bump();
        }
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                text.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if self.peek() == Some('.') && self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) {
            text.push('.');
            self.bump();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    text.push(c);
                    self.bump();
                } else {
                    break;
                }
            }
        }
        text.parse::<f64>().map_err(|_| DslError::Syntax {
            span: start,
            message: format!("`{text}` is not a number"),
        })
    }

    fn string(&mut self) -> Result<String, DslError> {
        let start = self.span();
        self.bump();
        let mut out = String::new();
        loop {
            // The span of the character this iteration is about to consume.
            // Captured *before* the bump: an unknown escape has to point at
            // its backslash, and after `bump()` returns `\\` the cursor is
            // already past it.
            let consumed = self.span();
            match self.bump() {
                None => {
                    return Err(DslError::Syntax {
                        span: start,
                        message: "unterminated string".into(),
                    });
                }
                Some('"') => return Ok(out),
                Some('\\') => match self.bump() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('r') => out.push('\r'),
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some(other) => {
                        return Err(DslError::Syntax {
                            span: consumed,
                            message: format!("unknown escape `\\{other}`"),
                        });
                    }
                    None => {
                        return Err(DslError::Syntax {
                            span: start,
                            message: "unterminated string".into(),
                        });
                    }
                },
                Some(c) => out.push(c),
            }
        }
    }

    fn color(&mut self) -> Result<Rgba, DslError> {
        let start = self.span();
        self.bump();
        let mut digits = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_hexdigit() {
                digits.push(c);
                self.bump();
            } else {
                break;
            }
        }
        Rgba::from_hex(&digits).ok_or_else(|| DslError::Syntax {
            span: start,
            message: format!("`#{digits}` is not a colour (#rgb, #rrggbb or #rrggbbaa)"),
        })
    }

    fn reference(&mut self) -> Result<String, DslError> {
        let start = self.span();
        self.bump();
        if !self
            .peek()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            return Err(DslError::Syntax {
                span: start,
                message: "`@` must be followed by a binding name".into(),
            });
        }
        Ok(self.word())
    }
}

/// Turn `source` into tokens, ending with the position just past the input.
pub fn lex(source: &str) -> Result<Vec<Spanned>, DslError> {
    let mut lexer = Lexer::new(source);
    let mut out = Vec::new();
    loop {
        lexer.skip_trivia()?;
        let span = lexer.span();
        let Some(c) = lexer.peek() else {
            return Ok(out);
        };
        let token = match c {
            '=' => {
                lexer.bump();
                Token::Eq
            }
            ':' => {
                lexer.bump();
                Token::Colon
            }
            '{' => {
                lexer.bump();
                Token::LBrace
            }
            '}' => {
                lexer.bump();
                Token::RBrace
            }
            '"' => Token::Text(lexer.string()?),
            '#' => Token::Color(lexer.color()?),
            '@' => Token::Ref(lexer.reference()?),
            c if c.is_ascii_digit()
                || (c == '-' && lexer.peek_at(1).is_some_and(|n| n.is_ascii_digit())) =>
            {
                Token::Number(lexer.number()?)
            }
            c if c.is_ascii_alphabetic() || c == '_' => match lexer.word().as_str() {
                "true" => Token::Bool(true),
                "false" => Token::Bool(false),
                word => Token::Ident(word.to_string()),
            },
            other => {
                return Err(DslError::Syntax {
                    span,
                    message: format!("unexpected character `{other}`"),
                });
            }
        };
        out.push(Spanned { span, token });
        let _ = lexer.source;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(source: &str) -> Vec<Token> {
        lex(source)
            .expect("lexes")
            .into_iter()
            .map(|s| s.token)
            .collect()
    }

    #[test]
    fn a_node_statement_lexes_into_its_parts() {
        assert_eq!(
            tokens(r#"a = text label="hi""#),
            vec![
                Token::Ident("a".into()),
                Token::Eq,
                Token::Ident("text".into()),
                Token::Ident("label".into()),
                Token::Eq,
                Token::Text("hi".into()),
            ]
        );
    }

    #[test]
    fn an_event_attribute_lexes_as_word_colon_word() {
        assert_eq!(
            tokens("on:click=save"),
            vec![
                Token::Ident("on".into()),
                Token::Colon,
                Token::Ident("click".into()),
                Token::Eq,
                Token::Ident("save".into()),
            ]
        );
    }

    #[test]
    fn colours_accept_the_three_css_lengths() {
        assert_eq!(
            tokens("#abc #112233 #11223344"),
            vec![
                Token::Color(Rgba::opaque(0xaa, 0xbb, 0xcc)),
                Token::Color(Rgba::opaque(0x11, 0x22, 0x33)),
                Token::Color(Rgba::new(0x11, 0x22, 0x33, 0x44)),
            ]
        );
    }

    #[test]
    fn a_key_may_be_dotted_and_hyphenated() {
        assert_eq!(
            tokens("app.save-button = text"),
            vec![
                Token::Ident("app.save-button".into()),
                Token::Eq,
                Token::Ident("text".into()),
            ]
        );
    }

    #[test]
    fn a_dash_before_a_digit_starts_a_negative_number() {
        // Otherwise `x-4` would lex as one word and the author would be told
        // about a bogus property instead of a missing operand.
        assert_eq!(
            tokens("a = text size=-4"),
            vec![
                Token::Ident("a".into()),
                Token::Eq,
                Token::Ident("text".into()),
                Token::Ident("size".into()),
                Token::Eq,
                Token::Number(-4.0),
            ]
        );
    }

    #[test]
    fn comments_are_skipped_and_positions_survive_them() {
        let source = "// leading\na = text /* inline */ size=1\n";
        let spanned = lex(source).expect("lexes");
        assert_eq!(spanned[0].token, Token::Ident("a".into()));
        assert_eq!(spanned[0].span, Span::new(2, 1));
        // The first token after each comment carries a real position: the
        // line comment puts `a` on line 2, and the block comment puts `size`
        // at column 23 — not wherever the comment happened to end.
        assert_eq!(spanned[3].token, Token::Ident("size".into()));
        assert_eq!(spanned[3].span, Span::new(2, 23));
        assert_eq!(spanned.last().unwrap().span, Span::new(2, 28));
    }

    #[test]
    fn escapes_inside_strings_are_decoded() {
        assert_eq!(
            tokens(r#""a\"b\\c\nd""#),
            vec![Token::Text("a\"b\\c\nd".into())]
        );
    }

    #[test]
    fn a_bad_escape_names_itself() {
        let err = lex(r#""a\q""#).expect_err("refused");
        let DslError::Syntax { span, message } = err else {
            panic!("expected a syntax error")
        };
        assert_eq!(span, Span::new(1, 3));
        assert!(message.contains("\\q"), "{message}");
    }

    #[test]
    fn an_unterminated_construct_points_at_where_it_started() {
        for (source, needle, column) in [
            ("a = text label=\"oops", "unterminated string", 16),
            ("/* forever", "unterminated block comment", 1),
            ("x = #12345", "is not a colour", 5),
            ("y = @", "binding name", 5),
        ] {
            let err = lex(source).expect_err("refused");
            let DslError::Syntax { span, message } = err else {
                panic!("expected a syntax error for {source:?}")
            };
            assert!(message.contains(needle), "{source:?} gave {message:?}");
            assert_eq!(span.column, column, "{source:?}");
        }
    }
}
