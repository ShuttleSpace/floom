//! A declarative UI DSL: source text to UI IR to a Rust builder program.
//!
//! Three stages, each independently useful:
//!
//! - [`parse`] — source text to an [`ast::Ast`], with a line/column on every
//!   diagnostic. The lexer and parser are hand-written: the grammar is small
//!   enough that a parser generator would add a build step and a dependency
//!   without buying anything.
//! - [`lower`] — AST to a validated [`floem_ui_ir::UiDocument`]. This stage owns
//!   no vocabulary at all: it builds the IR and asks the IR's own tables to
//!   validate, so a property added to the IR is immediately usable in the DSL
//!   and a property the IR rejects is rejected here too. Errors are reported
//!   against the *position of the offending attribute*, which is possible
//!   because the lowering keeps a key/property → span map.
//! - [`emit`] — the document to a Rust builder program, with repeated identical
//!   subtrees emitted once. See that module for what it refuses and why.
//!
//! ```
//! let source = r#"
//!     root = stack axis=vertical gap=8 {
//!         title = text label="hello" size=16
//!     }
//! "#;
//! let compiled = floem_dsl::compile(source).expect("valid");
//! assert_eq!(compiled.document.nodes.len(), 2);
//! ```

#![forbid(unsafe_code)]

pub mod ast;
pub mod emit;
mod lexer;
mod lower;
mod parser;

use std::fmt;

pub use ast::{Ast, AstAttr, AstAttrName, AstNode, AstValue};
pub use emit::{EmitError, EmitOptions, Emitted, Unmapped, emit};
pub use lexer::{Span, Token, lex};
pub use lower::{Bound, lower};
pub use parser::parse;

use floem_ui_ir::{IrError, UiDocument};

/// Anything that can go wrong compiling DSL source.
#[derive(Debug, Clone, PartialEq)]
pub enum DslError {
    /// The source does not parse.
    Syntax { span: Span, message: String },
    /// The source parses but describes something the IR forbids.
    ///
    /// `span` is the position of the offending node or property when the IR's
    /// error names one; some rules (a duplicate binding) are document-wide and
    /// carry no position.
    Semantic { span: Option<Span>, error: IrError },
}

impl fmt::Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax { span, message } => write!(f, "{span}: {message}"),
            Self::Semantic {
                span: Some(s),
                error,
            } => write!(f, "{s}: {error}"),
            Self::Semantic { span: None, error } => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for DslError {}

/// A document plus the source position of every node key.
#[derive(Debug, Clone)]
pub struct Compiled {
    pub document: UiDocument,
    pub spans: lower::SpanMap,
}

/// Parse and lower a DSL document.
pub fn compile(source: &str) -> Result<Compiled, DslError> {
    let ast = parse(source)?;
    let bound = lower(&ast)?;
    Ok(Compiled {
        document: bound.document,
        spans: bound.spans,
    })
}
