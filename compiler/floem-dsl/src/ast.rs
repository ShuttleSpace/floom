//! The parse tree: what the parser produces and the lowering consumes.
//!
//! Deliberately shallow. Kinds are still strings and property names are still
//! strings here; resolving them is the *lowering* step's job, because the
//! vocabulary lives in the IR and the parser should not know it.

use floem_ui_ir::Rgba;

use crate::lexer::Span;

/// A parsed document.
#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    /// The `bindings { … }` block, or empty when the document declares none.
    pub bindings: Vec<AstBinding>,
    pub root: AstNode,
}

/// A `name: type` declaration inside `bindings { … }`.
#[derive(Debug, Clone, PartialEq)]
pub struct AstBinding {
    pub span: Span,
    pub name: String,
    pub ty: String,
    pub ty_span: Span,
}

/// One `key = kind attribute* block?` statement.
#[derive(Debug, Clone, PartialEq)]
pub struct AstNode {
    pub key: String,
    pub key_span: Span,
    /// As written; the lowering resolves it against the IR's kind vocabulary.
    pub kind: String,
    pub kind_span: Span,
    pub attrs: Vec<AstAttr>,
    pub children: Vec<AstNode>,
}

impl AstNode {
    /// Preorder walk, including `self`.
    pub fn walk(&self, visit: &mut impl FnMut(&AstNode)) {
        visit(self);
        for child in &self.children {
            child.walk(visit);
        }
    }

    /// Number of nodes in this subtree.
    pub fn len(&self) -> usize {
        let mut count = 0;
        self.walk(&mut |_| count += 1);
        count
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

/// One attribute: either a property assignment or an event binding.
#[derive(Debug, Clone, PartialEq)]
pub struct AstAttr {
    pub span: Span,
    pub name: AstAttrName,
    pub value: AstValue,
    pub value_span: Span,
}

/// What an attribute is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AstAttrName {
    /// `name = value`
    Prop(String),
    /// `on:name = handler`
    Event(String),
}

/// A literal written in the source.
#[derive(Debug, Clone, PartialEq)]
pub enum AstValue {
    Text(String),
    Number(f64),
    Bool(bool),
    Color(Rgba),
    /// A bare word: an enum member such as `vertical`.
    Ident(String),
    /// `@name`
    Ref(String),
}

impl AstValue {
    /// A short description for diagnostics.
    pub fn describe(&self) -> String {
        match self {
            Self::Text(text) => format!("\"{text}\""),
            Self::Number(n) => format!("{n}"),
            Self::Bool(b) => format!("{b}"),
            Self::Color(c) => c.to_hex(),
            Self::Ident(name) => name.clone(),
            Self::Ref(name) => format!("@{name}"),
        }
    }
}
