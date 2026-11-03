//! Span-based diagnostics for the TSX front end.
//!
//! A `TsxError` carries a human-readable message plus an optional source
//! position. Positions are 1-based line/column so they can be printed next to
//! the offending line, matching the diagnostic style `floom-dsl` uses for its
//! own `DslError`.

use std::fmt;

/// A compile error raised while turning TSX into a `UiDocument`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TsxError {
    /// What went wrong, in terms an author can act on.
    pub message: String,
    /// 1-based source line, when the error can be pinned to one.
    pub line: Option<usize>,
    /// 1-based column within `line`, when known.
    pub col: Option<usize>,
}

impl TsxError {
    /// An error with no source location (e.g. a post-parse invariant failure).
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            col: None,
        }
    }

    /// Attach a 1-based line/column to an existing error.
    pub fn at(mut self, line: usize, col: usize) -> Self {
        self.line = Some(line);
        self.col = Some(col);
        self
    }
}

impl fmt::Display for TsxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.col) {
            (Some(line), Some(col)) => write!(f, "[{line}:{col}] {}", self.message),
            (Some(line), None) => write!(f, "[{line}] {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for TsxError {}

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, TsxError>;
