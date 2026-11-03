//! AST to UI IR.
//!
//! The one rule this module follows: **it owns no vocabulary.** Kinds, events,
//! properties and domains are resolved against the IR's tables, and anything
//! the IR forbids is reported by the IR's own validator. The reason is drift —
//! a second property table here would silently accept what the IR rejects (or
//! the reverse), and the disagreement would show up as a document that loads
//! from a file but not from source.
//!
//! What the lowering adds on top is *positions*. The IR reasons about keys and
//! property names; an editor needs line and column, so every node, property and
//! binding declaration is recorded with a span and used to place the IR's
//! errors.

use std::collections::BTreeMap;

use floom_ui_ir::{
    Binding, BindingKind, DEFAULT_SLOT, EventBinding, EventName, IrError, IrVersion, NodeKind,
    Prop, PropValue, Slot, UiDocument, UiNode, ValueType, prop_spec,
};

use crate::DslError;
use crate::ast::{Ast, AstAttrName, AstNode, AstValue};
use crate::lexer::Span;

/// Where each named thing was written.
#[derive(Debug, Clone, Default)]
pub struct SpanMap {
    nodes: BTreeMap<String, Span>,
    props: BTreeMap<(String, String), Span>,
    bindings: BTreeMap<String, Span>,
}

impl SpanMap {
    pub fn node(&self, key: &str) -> Option<Span> {
        self.nodes.get(key).copied()
    }

    pub fn prop(&self, key: &str, prop: &str) -> Option<Span> {
        self.props
            .get(&(key.to_string(), prop.to_string()))
            .copied()
    }

    pub fn binding(&self, name: &str) -> Option<Span> {
        self.bindings.get(name).copied()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// The best position to blame for an IR error.
    ///
    /// Most errors name a node and, when the rule is about one property, that
    /// property as well; pointing at the attribute rather than the node is the
    /// difference between "something in this subtree" and "this value".
    pub fn locate(&self, error: &IrError) -> Option<Span> {
        match error {
            IrError::UnknownProp { key, prop }
            | IrError::PropNotAllowed { key, prop, .. }
            | IrError::WrongValueType { key, prop, .. }
            | IrError::ValueOutsideDomain { key, prop, .. }
            | IrError::DuplicateProp { key, prop } => self
                .prop(key, prop)
                .or_else(|| self.nodes.get(key).copied()),
            IrError::BadKey { key }
            | IrError::DuplicateKey { key }
            | IrError::Unreachable { key }
            | IrError::DuplicateSlot { key, .. }
            | IrError::DuplicateEvent { key, .. }
            | IrError::EventNotAllowed { key, .. }
            | IrError::UnresolvedBinding { key, .. } => self.nodes.get(key).copied(),
            IrError::UnknownChild { parent, .. } => self.nodes.get(parent).copied(),
            IrError::TwoParents { child, .. } | IrError::Cycle { at: child } => {
                self.nodes.get(child).copied()
            }
            // These are about the binding declaration, which is where the fix
            // goes: a value binding used as a handler is declared wrong.
            IrError::DuplicateBinding { name }
            | IrError::BindingTypeMismatch { name, .. }
            | IrError::HandlerUsedAsValue { name }
            | IrError::ValueUsedAsHandler { name } => self.bindings.get(name).copied(),
            _ => None,
        }
    }
}

/// A lowered document and the positions that produced it.
#[derive(Debug, Clone)]
pub struct Bound {
    pub document: UiDocument,
    pub spans: SpanMap,
}

/// Resolve a binding declaration's type word.
fn binding_type(word: &str, span: Span) -> Result<BindingKind, DslError> {
    Ok(match word {
        "handler" => BindingKind::Handler,
        "text" => BindingKind::Value(ValueType::Text),
        "number" => BindingKind::Value(ValueType::Number),
        "int" => BindingKind::Value(ValueType::Int),
        "bool" => BindingKind::Value(ValueType::Bool),
        "color" | "colour" => BindingKind::Value(ValueType::Color),
        "ident" => BindingKind::Value(ValueType::Ident),
        other => {
            return Err(DslError::Syntax {
                span,
                message: format!(
                    "`{other}` is not a binding type; expected handler, text, number, int, bool, \
                     color or ident"
                ),
            });
        }
    })
}

/// Convert a source literal into an IR value.
///
/// Numbers are the interesting case: the source has one numeric spelling, so
/// `columns=3` and `gap=3` must become an `Int` and a `Number` respectively.
/// The property's declared type decides, which keeps the source readable while
/// the document stays precisely typed. A fractional value in an `Int` slot is
/// refused here rather than truncated, because truncating would silently move
/// every following item.
fn literal(
    kind: NodeKind,
    name: &str,
    value: &AstValue,
    span: Span,
) -> Result<PropValue, DslError> {
    Ok(match value {
        AstValue::Text(text) => PropValue::Text(text.clone()),
        AstValue::Bool(b) => PropValue::Bool(*b),
        AstValue::Color(c) => PropValue::Color(*c),
        AstValue::Ident(word) => PropValue::Ident(word.clone()),
        AstValue::Ref(binding) => PropValue::Ref(binding.clone()),
        AstValue::Number(n) => {
            if prop_spec(kind, name).map(|spec| spec.ty) == Some(ValueType::Int) {
                if n.fract() != 0.0 {
                    return Err(DslError::Syntax {
                        span,
                        message: format!("`{name}` wants a whole number, got {n}"),
                    });
                }
                PropValue::Int(*n as i64)
            } else {
                PropValue::Number(*n)
            }
        }
    })
}

/// A list of the words a message should suggest, for a typo.
fn suggestions<'a>(words: impl IntoIterator<Item = &'a str>) -> String {
    let mut list: Vec<&str> = words.into_iter().collect();
    list.sort_unstable();
    list.join(", ")
}

/// Lower an AST into a validated UI IR document.
pub fn lower(ast: &Ast) -> Result<Bound, DslError> {
    let mut spans = SpanMap::default();

    let mut bindings = Vec::with_capacity(ast.bindings.len());
    for declaration in &ast.bindings {
        // Every declaration is kept, duplicates included: the IR's validator
        // reports the duplicate, so the rule lives in one place. Deduplicating
        // here would make the mistake invisible.
        spans
            .bindings
            .insert(declaration.name.clone(), declaration.span);
        bindings.push(Binding {
            name: declaration.name.clone(),
            kind: binding_type(&declaration.ty, declaration.ty_span)?,
        });
    }

    // Flatten depth-first, in source order, so the node list reads like the
    // document and a diff of two documents is a diff of the source.
    let mut nodes: Vec<UiNode> = Vec::new();
    flatten(&ast.root, &mut nodes, &mut spans)?;

    let document = UiDocument {
        version: IrVersion::CURRENT,
        root: ast.root.key.clone().into(),
        bindings,
        nodes,
    };

    // The IR is the authority. Anything it rejects is a semantic error, aimed
    // at the position that produced it.
    document.validate().map_err(|error| DslError::Semantic {
        span: spans.locate(&error),
        error,
    })?;

    Ok(Bound { document, spans })
}

fn flatten(node: &AstNode, nodes: &mut Vec<UiNode>, spans: &mut SpanMap) -> Result<(), DslError> {
    spans.nodes.insert(node.key.clone(), node.key_span);

    let Some(kind) = NodeKind::parse(&node.kind) else {
        let known: Vec<&str> = NodeKind::ALL.iter().map(|k| k.name()).collect();
        return Err(DslError::Syntax {
            span: node.kind_span,
            message: format!(
                "`{}` is not a node kind; expected one of {}",
                node.kind,
                suggestions(known)
            ),
        });
    };

    let mut props: Vec<Prop> = Vec::new();
    let mut events: Vec<EventBinding> = Vec::new();
    for attr in &node.attrs {
        match &attr.name {
            AstAttrName::Prop(name) => {
                spans
                    .props
                    .insert((node.key.clone(), name.clone()), attr.value_span);
                let value = literal(kind, name, &attr.value, attr.value_span)?;
                props.push(Prop::new(name.clone(), value));
            }
            AstAttrName::Event(name) => {
                let Some(event) = EventName::parse(name) else {
                    let known: Vec<&str> = EventName::all().iter().map(|e| e.name()).collect();
                    return Err(DslError::Syntax {
                        span: attr.span,
                        message: format!(
                            "`{name}` is not an event; expected one of {}",
                            suggestions(known)
                        ),
                    });
                };
                let handler = match &attr.value {
                    AstValue::Ident(handler) => handler.clone(),
                    other => {
                        return Err(DslError::Syntax {
                            span: attr.value_span,
                            message: format!(
                                "an event handler must be a name, found {}",
                                other.describe()
                            ),
                        });
                    }
                };
                events.push(EventBinding::new(event, handler));
            }
        }
    }

    let children: Vec<floom_ui_ir::NodeKey> = node
        .children
        .iter()
        .map(|child| child.key.clone().into())
        .collect();

    nodes.push(UiNode {
        key: node.key.clone().into(),
        kind,
        props,
        slots: if children.is_empty() {
            Vec::new()
        } else {
            vec![Slot::new(DEFAULT_SLOT, children)]
        },
        events,
    });

    for child in &node.children {
        flatten(child, nodes, spans)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn lowering_produces_a_document_in_source_order() {
        // Depth-first and in source order, so a diff of two documents reads
        // like a diff of the source that produced them.
        let ast = parse("a = stack { b = text c = stack { d = text } }").expect("parses");
        let bound = lower(&ast).expect("lowers");
        let keys: Vec<&str> = bound
            .document
            .nodes
            .iter()
            .map(|n| n.key.as_str())
            .collect();
        assert_eq!(keys, ["a", "b", "c", "d"]);
        // The container's slot points at its children by key.
        let slots = &bound.document.nodes[0].slots;
        assert_eq!(slots.len(), 1);
        assert_eq!(
            slots[0].children,
            vec![
                floom_ui_ir::NodeKey::from("b"),
                floom_ui_ir::NodeKey::from("c")
            ]
        );
        assert!(
            bound.document.nodes[1].slots.is_empty(),
            "a leaf has no slots"
        );
    }

    #[test]
    fn an_unknown_kind_names_the_vocabulary() {
        let ast = parse("a = txet").expect("parses");
        let DslError::Syntax { span, message } = lower(&ast).expect_err("refused") else {
            panic!("expected a syntax error");
        };
        assert!(message.contains("not a node kind"), "{message}");
        assert!(
            message.contains("text"),
            "the suggestion list names text: {message}"
        );
        assert_eq!(span, Span::new(1, 5), "blamed at the kind, not the key");
    }

    #[test]
    fn numbers_take_their_type_from_the_property() {
        // One numeric spelling at the source; the property's declared type
        // decides Int versus Number.
        let ast = parse("a = stack { g = grid columns=3 s = stack gap=3 }").expect("parses");
        let bound = lower(&ast).expect("lowers");
        let columns = &bound.document.nodes[1].props[0].value;
        assert_eq!(columns, &PropValue::Int(3));
        let gap = &bound.document.nodes[2].props[0].value;
        assert_eq!(gap, &PropValue::Number(3.0));
    }

    #[test]
    fn a_fractional_int_is_refused_rather_than_truncated() {
        // Truncating 2.5 to 2 would silently move every following grid item;
        // the author has to say which they meant.
        let ast = parse("g = grid columns=2.5").expect("parses");
        let DslError::Syntax { span, message } = lower(&ast).expect_err("refused") else {
            panic!("expected a syntax error");
        };
        assert!(message.contains("whole number"), "{message}");
        assert_eq!(span, Span::new(1, 18), "blamed at the value");
    }

    #[test]
    fn an_unknown_event_is_reported_at_the_attribute() {
        let ast = parse("b = button on:clik=save").expect("parses");
        let DslError::Syntax { span, message } = lower(&ast).expect_err("refused") else {
            panic!("expected a syntax error");
        };
        assert!(message.contains("not an event"), "{message}");
        assert!(message.contains("click"), "click is suggested: {message}");
        assert_eq!(span, Span::new(1, 12), "blamed at the `on` attribute");
    }

    #[test]
    fn an_ir_rejection_is_a_semantic_error_located_at_its_property() {
        // The IR is the vocabulary authority; the lowering's job is to place
        // the rejection at the line and column that produced it.
        let ast = parse("s = stack axis=diagonal").expect("parses");
        let DslError::Semantic { span, error } = lower(&ast).expect_err("refused") else {
            panic!("expected a semantic error, got {ast:?}");
        };
        assert!(
            error.to_string().contains("diagonal"),
            "the bad value is named: {error}"
        );
        assert_eq!(span, Some(Span::new(1, 16)), "blamed at the value");
    }

    #[test]
    fn a_handler_used_as_a_value_blames_the_binding_declaration() {
        // The rule lives with the declaration: `n` was declared as a handler,
        // and that is where the fix goes.
        let ast = parse("bindings { n: handler }\nt = text label=@n").expect("parses");
        let DslError::Semantic { span, error } = lower(&ast).expect_err("refused") else {
            panic!("expected a semantic error");
        };
        assert!(
            matches!(error, IrError::HandlerUsedAsValue { .. }),
            "{error:?}"
        );
        assert_eq!(span, Some(Span::new(1, 12)), "blamed at the declaration");
    }

    #[test]
    fn duplicate_binding_declarations_reach_the_validator() {
        // The lowering deliberately keeps both: deduplicating here would make
        // the mistake invisible, and the IR already has the rule.
        // Handler names are plain identifiers (the `@` prefix is for value
        // references); they resolve against the declared bindings by name.
        let ast =
            parse("bindings { n: handler n: handler }\nt = button on:click=n").expect("parses");
        let DslError::Semantic { error, .. } = lower(&ast).expect_err("refused") else {
            panic!("expected a semantic error");
        };
        assert!(
            matches!(error, IrError::DuplicateBinding { .. }),
            "{error:?}"
        );
    }
}
