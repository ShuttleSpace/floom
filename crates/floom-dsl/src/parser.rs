//! Recursive-descent parser over the token stream.
//!
//! Grammar, in full:
//!
//! ```text
//! document   := bindings? node
//! bindings   := "bindings" "{" ( IDENT ":" IDENT )* "}"
//! node       := IDENT "=" IDENT attribute* block?
//! block      := "{" node* "}"
//! attribute  := IDENT "=" literal
//!             | "on" ":" IDENT "=" IDENT
//! literal    := STRING | NUMBER | BOOL | COLOR | REF | IDENT
//! ```
//!
//! A node is always named. That is a deliberate cost: the IR matches nodes
//! across a reload by key, so an unnamed node would have to be keyed by
//! position, and inserting a sibling above it would move every following node's
//! identity — exactly the silent state loss the key exists to prevent.

use crate::DslError;
use crate::ast::{Ast, AstAttr, AstAttrName, AstNode, AstValue};
use crate::lexer::{Span, Spanned, Token, lex};

struct Parser {
    tokens: Vec<Spanned>,
    index: usize,
}

type PResult<T> = Result<T, DslError>;

impl Parser {
    fn peek(&self) -> Option<&Spanned> {
        self.tokens.get(self.index)
    }

    fn peek_at(&self, offset: usize) -> Option<&Spanned> {
        self.tokens.get(self.index + offset)
    }

    fn bump(&mut self) -> Option<Spanned> {
        let token = self.tokens.get(self.index).cloned()?;
        self.index += 1;
        Some(token)
    }

    /// The position to blame when the input ends early.
    fn end_span(&self) -> Span {
        self.tokens
            .last()
            .map(|s| s.span)
            .unwrap_or(Span::new(1, 1))
    }

    fn error<T>(&self, span: Span, message: impl Into<String>) -> PResult<T> {
        Err(DslError::Syntax {
            span,
            message: message.into(),
        })
    }

    /// Describe what was found, for "expected X, found Y".
    fn found(&self) -> String {
        match self.peek() {
            Some(s) => s.token.describe(),
            None => "the end of the document".into(),
        }
    }

    fn expect(&mut self, want: Token, expected: &str) -> PResult<Span> {
        let span = self
            .peek()
            .map(|s| s.span)
            .unwrap_or_else(|| self.end_span());
        match self.peek() {
            Some(s) if s.token == want => {
                self.bump();
                Ok(span)
            }
            _ => self.error(span, format!("expected {expected}, found {}", self.found())),
        }
    }

    fn ident(&mut self, what: &str) -> PResult<(String, Span)> {
        let span = self
            .peek()
            .map(|s| s.span)
            .unwrap_or_else(|| self.end_span());
        match self.peek() {
            Some(Spanned {
                token: Token::Ident(name),
                ..
            }) => {
                let name = name.clone();
                self.bump();
                Ok((name, span))
            }
            _ => self.error(span, format!("expected {what}, found {}", self.found())),
        }
    }

    /// `bindings { name: type … }`, if present.
    fn bindings(&mut self) -> PResult<Vec<crate::ast::AstBinding>> {
        let is_block = matches!(
            (self.peek(), self.peek_at(1)),
            (
                Some(Spanned {
                    token: Token::Ident(word),
                    ..
                }),
                Some(Spanned {
                    token: Token::LBrace,
                    ..
                })
            ) if word == "bindings"
        );
        if !is_block {
            return Ok(Vec::new());
        }
        self.bump();
        self.bump();
        let mut out = Vec::new();
        while !matches!(
            self.peek(),
            Some(Spanned {
                token: Token::RBrace,
                ..
            })
        ) {
            let (name, span) = self.ident("a binding name")?;
            self.expect(Token::Colon, "`:`")?;
            let (ty, ty_span) = self.ident("a binding type")?;
            out.push(crate::ast::AstBinding {
                span,
                name,
                ty,
                ty_span,
            });
        }
        self.bump();
        Ok(out)
    }

    fn node(&mut self) -> PResult<AstNode> {
        let (key, key_span) = self.ident("a node name")?;
        self.expect(Token::Eq, "`=` after the node name")?;
        let (kind, kind_span) = self.ident("a node kind")?;

        let mut attrs = Vec::new();
        while let Some(Spanned {
            token: Token::Ident(word),
            span,
        }) = self.peek()
        {
            let word = word.clone();
            let span = *span;
            // `on:click=handler` is the one attribute form that is not
            // `name = value`; the lookahead is what tells the two apart.
            let is_event = word == "on"
                && matches!(
                    self.peek_at(1),
                    Some(Spanned {
                        token: Token::Colon,
                        ..
                    })
                );
            // `name = value` after a node's kind is ambiguous between an
            // attribute and a sibling node (`b = text size=1` vs
            // `b = text c = text`): both are `Ident Eq`. The property
            // vocabulary is the tiebreak — an unknown name starts a sibling.
            // A node key that collides with a property name therefore cannot
            // carry attributes of its own; that is a documented grammar rule,
            // not an accident.
            if !is_event && !floom_ui_ir::prop_is_declared(&word) {
                break;
            }
            if is_event {
                self.bump();
                self.bump();
                let (event, _) = self.ident("an event name")?;
                self.expect(Token::Eq, "`=` after the event name")?;
                let (handler, handler_span) = self.ident("a handler name")?;
                attrs.push(AstAttr {
                    span,
                    name: AstAttrName::Event(event),
                    value: AstValue::Ident(handler),
                    value_span: handler_span,
                });
            } else {
                self.bump();
                self.expect(Token::Eq, "`=` after the property name")?;
                let (value, value_span) = self.literal()?;
                attrs.push(AstAttr {
                    span,
                    name: AstAttrName::Prop(word),
                    value,
                    value_span,
                });
            }
        }

        let mut children = Vec::new();
        if matches!(
            self.peek(),
            Some(Spanned {
                token: Token::LBrace,
                ..
            })
        ) {
            self.bump();
            while !matches!(
                self.peek(),
                Some(Spanned {
                    token: Token::RBrace,
                    ..
                })
            ) {
                children.push(self.node()?);
            }
            self.bump();
        }

        Ok(AstNode {
            key,
            key_span,
            kind,
            kind_span,
            attrs,
            children,
        })
    }

    fn literal(&mut self) -> PResult<(AstValue, Span)> {
        let span = self
            .peek()
            .map(|s| s.span)
            .unwrap_or_else(|| self.end_span());
        let Some(spanned) = self.bump() else {
            return self.error(span, "expected a value, found the end of the document");
        };
        let value = match spanned.token {
            Token::Text(text) => AstValue::Text(text),
            Token::Number(n) => AstValue::Number(n),
            Token::Bool(b) => AstValue::Bool(b),
            Token::Color(c) => AstValue::Color(c),
            Token::Ref(name) => AstValue::Ref(name),
            Token::Ident(name) => AstValue::Ident(name),
            other => {
                return self.error(
                    spanned.span,
                    format!("expected a value, found {}", other.describe()),
                );
            }
        };
        Ok((value, spanned.span))
    }
}

/// Parse DSL source into an AST.
pub fn parse(source: &str) -> Result<Ast, DslError> {
    let tokens = lex(source)?;
    if tokens.is_empty() {
        return Err(DslError::Syntax {
            span: Span::new(1, 1),
            message: "the document is empty".into(),
        });
    }
    let mut parser = Parser { tokens, index: 0 };
    let bindings = parser.bindings()?;
    let root = parser.node()?;
    if let Some(extra) = parser.peek() {
        return Err(DslError::Syntax {
            span: extra.span,
            message: format!(
                "expected the end of the document after the root node, found {}",
                extra.token.describe()
            ),
        });
    }
    Ok(Ast { bindings, root })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bare_node_parses() {
        let ast = parse("a = text").expect("parses");
        assert_eq!(ast.root.key, "a");
        assert_eq!(ast.root.kind, "text");
        assert!(ast.root.attrs.is_empty());
        assert!(ast.root.children.is_empty());
        assert!(ast.bindings.is_empty());
    }

    #[test]
    fn nesting_builds_a_tree_in_order() {
        let ast = parse("a = stack { b = text c = text }").expect("parses");
        let keys: Vec<&str> = ast.root.children.iter().map(|c| c.key.as_str()).collect();
        assert_eq!(keys, ["b", "c"]);
        assert_eq!(ast.root.len(), 3);
    }

    #[test]
    fn a_bindings_block_is_read_and_is_optional() {
        let ast = parse("bindings { n: number h: handler }\nr = text").expect("parses");
        assert_eq!(ast.bindings.len(), 2);
        assert_eq!(ast.bindings[0].name, "n");
        assert_eq!(ast.bindings[1].ty, "handler");
        assert!(parse("r = text").expect("parses").bindings.is_empty());
    }

    #[test]
    fn an_event_attribute_names_its_handler() {
        let ast = parse("a = button on:click=save").expect("parses");
        assert_eq!(ast.root.attrs[0].name, AstAttrName::Event("click".into()));
        assert_eq!(ast.root.attrs[0].value, AstValue::Ident("save".into()));
    }

    #[test]
    fn literal_kinds_are_preserved() {
        // The attribute names are real vocabulary — the parser needs them to
        // tell an attribute from a sibling node — and every value kind must
        // survive the parse unchanged.
        let ast = parse(
            "a = stack { t = text label=\"hi\" background=#102030 \
             b = button variant=filled s = stack axis=vertical width=1.5 }",
        )
        .expect("parses");
        let t = &ast.root.children[0];
        assert_eq!(t.attrs[0].value, AstValue::Text("hi".into()));
        assert_eq!(
            t.attrs[1].value,
            AstValue::Color(floom_ui_ir::Rgba::opaque(0x10, 0x20, 0x30))
        );
        let b = &ast.root.children[1];
        assert_eq!(b.attrs[0].value, AstValue::Ident("filled".into()));
        let s = &ast.root.children[2];
        assert_eq!(s.attrs[1].value, AstValue::Number(1.5));
    }

    #[test]
    fn a_ref_is_distinct_from_an_ident() {
        // `@x` is a binding; `x` is a literal word. Confusing them would turn a
        // typo into a silently new value.
        let ast = parse("a = text label=@x axis=y").expect("parses");
        assert_eq!(ast.root.attrs[0].value, AstValue::Ref("x".into()));
        assert_eq!(ast.root.attrs[1].value, AstValue::Ident("y".into()));
    }

    #[test]
    fn the_position_of_a_failure_is_reported() {
        // Exact positions, not just "somewhere": a diagnostic that points at
        // the wrong token is worse than none, because the author trusts it.
        // The convention is "blame the last token consumed": a missing token
        // is reported at the token that was missing its continuation, which
        // is where the author was typing.
        for (source, needle, line, column) in [
            ("a = ", "expected a node kind", 1, 3),
            ("a text", "expected `=`", 1, 3),
            ("a = text size=", "expected a value", 1, 14),
            ("a = text { b = text", "expected a node name", 1, 16),
            ("a = text }", "expected the end of the document", 1, 10),
            ("bindings { n number } r = text", "expected `:`", 1, 14),
        ] {
            let err = parse(source).expect_err("refused");
            let DslError::Syntax { span, message } = err else {
                panic!("expected a syntax error for {source:?}");
            };
            assert!(message.contains(needle), "{source:?} gave {message:?}");
            assert_eq!(
                (span.line, span.column),
                (line, column),
                "{source:?} blamed the wrong token: {message:?}"
            );
        }
    }

    #[test]
    fn an_unknown_word_after_a_kind_starts_a_sibling_not_an_attribute() {
        // `size` is a property, so it binds to `b`; `c` is in no property
        // table, so it starts a sibling. Without the vocabulary tiebreak both
        // spellings are `Ident Eq` and the second child is silently eaten as
        // an attribute of the first.
        let ast = parse("a = stack { b = text size=12 c = text }").expect("parses");
        let b = &ast.root.children[0];
        assert_eq!(b.attrs.len(), 1, "size belongs to b");
        assert_eq!(b.attrs[0].name, AstAttrName::Prop("size".into()));
        assert_eq!(b.children.len(), 0);
        assert_eq!(ast.root.children[1].key, "c");

        // And the whole-sibling form stays whole.
        let ast = parse("a = stack { b = text c = text }").expect("parses");
        assert_eq!(ast.root.children.len(), 2);
        assert!(ast.root.children[0].attrs.is_empty());
    }

    #[test]
    fn an_empty_document_is_a_clear_error() {
        let err = parse("   // just a comment\n").expect_err("refused");
        assert!(
            matches!(&err, DslError::Syntax { message, .. } if message.contains("empty")),
            "{err:?}"
        );
    }

    #[test]
    fn an_empty_block_is_allowed() {
        let ast = parse("a = stack {}").expect("parses");
        assert!(ast.root.children.is_empty());
    }
}
