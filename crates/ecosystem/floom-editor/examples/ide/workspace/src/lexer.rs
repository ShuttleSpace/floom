//! The tokenizer.

use std::fmt;

/// One lexeme, with the span it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Number(i64),
    Ident(String),
    Keyword(Keyword),
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    Less,
    Open,
    Close,
    Semi,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Keyword {
    Let,
    True,
    False,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Kind::Number(value) => write!(f, "{value}"),
            Kind::Ident(name) => write!(f, "{name}"),
            Kind::Keyword(keyword) => write!(f, "{keyword:?}"),
            other => write!(f, "{other:?}"),
        }
    }
}

pub struct Lexer<'source> {
    source: &'source str,
    offset: usize,
}

impl<'source> Lexer<'source> {
    pub fn new(source: &'source str) -> Self {
        Self { source, offset: 0 }
    }

    /// Every token, ending with an `Eof` sentinel.
    pub fn tokens(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace();
            let start = self.offset;
            let Some(c) = self.bump() else { break };

            let kind = match c {
                '0'..='9' => self.number(c)?,
                'a'..='z' | 'A'..='Z' | '_' => self.word(c),
                '+' => Kind::Plus,
                '-' => Kind::Minus,
                '*' => Kind::Star,
                '/' => Kind::Slash,
                '=' => Kind::Equal,
                '<' => Kind::Less,
                '(' => Kind::Open,
                ')' => Kind::Close,
                ';' => Kind::Semi,
                other => return Err(format!("unexpected character {other:?}")),
            };

            tokens.push(Token {
                kind,
                start,
                end: self.offset,
            });
        }

        tokens.push(Token {
            kind: Kind::Eof,
            start: self.offset,
            end: self.offset,
        });
        Ok(tokens)
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.source[self.offset..].chars().next()?;
        self.offset += c.len_utf8();
        Some(c)
    }

    fn number(&mut self, first: char) -> Result<Kind, String> {
        let mut value = first.to_digit(10).unwrap() as i64;

        while let Some(c) = self.peek() {
            let Some(digit) = c.to_digit(10) else { break };
            value = value * 10 + i64::from(digit);
            self.bump();
        }

        Ok(Kind::Number(value))
    }

    fn word(&mut self, first: char) -> Kind {
        let start = self.offset - first.len_utf8();

        while let Some(c) = self.peek() {
            if !(c.is_alphanumeric() || c == '_') {
                break;
            }
            self.bump();
        }

        match &self.source[start..self.offset] {
            "let" => Kind::Keyword(Keyword::Let),
            "true" => Kind::Keyword(Keyword::True),
            "false" => Kind::Keyword(Keyword::False),
            name => Kind::Ident(name.to_string()),
        }
    }

    fn peek(&self) -> Option<char> {
        self.source[self.offset..].chars().next()
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }
}
