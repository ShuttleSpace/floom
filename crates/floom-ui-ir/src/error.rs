//! Everything the IR can refuse, with the rule each refusal enforces.

use std::fmt;

use crate::node::{EventName, NodeKind};
use crate::value::ValueType;
use crate::version::IrVersion;

/// A document that could not be loaded, or a rule it broke.
///
/// The variants are deliberately fine-grained: a tool that reports "invalid UI
/// document" is useless, while one that reports *which node, which property and
/// which type* was expected lets an author fix it without guessing.
#[derive(Debug, Clone, PartialEq)]
pub enum IrError {
    // --- version gate ------------------------------------------------------
    /// The document was written by a newer minor of this major.
    VersionTooNew {
        found: IrVersion,
        supported: IrVersion,
    },
    /// The structural vocabulary differs; no honest reading exists.
    MajorMismatch { found: u32, supported: u32 },
    /// An older minor with no registered step to bring it forward.
    NoMigrationPath { minor: u32 },

    /// The document is not even the right shape (wrong JSON, unknown enum tag).
    Json(String),

    // --- structure ---------------------------------------------------------
    /// A node key that is not a legal key.
    BadKey { key: String },
    /// Two nodes claim the same key.
    DuplicateKey { key: String },
    /// The document's `root` names no node.
    MissingRoot { root: String },
    /// A slot names a child that does not exist.
    UnknownChild { parent: String, child: String },
    /// A node is claimed by two different parents.
    TwoParents {
        child: String,
        first: String,
        second: String,
    },
    /// Following slots from a node returns to it.
    Cycle { at: String },
    /// A node no walk from the root can reach.
    Unreachable { key: String },
    /// One node declares the same slot twice.
    DuplicateSlot { key: String, slot: String },

    // --- properties --------------------------------------------------------
    /// The same property is set twice on one node.
    DuplicateProp { key: String, prop: String },
    /// No kind declares this property.
    UnknownProp { key: String, prop: String },
    /// The property exists, but not for this kind.
    PropNotAllowed {
        key: String,
        kind: NodeKind,
        prop: String,
    },
    /// The value's tag does not match the property's declared type.
    WrongValueType {
        key: String,
        prop: String,
        expected: ValueType,
        found: &'static str,
    },
    /// An identifier value outside the domain the property allows.
    ValueOutsideDomain {
        key: String,
        prop: String,
        value: String,
    },

    // --- bindings and events ----------------------------------------------
    /// Two bindings share a name.
    DuplicateBinding { name: String },
    /// A reference names a binding that is not declared.
    UnresolvedBinding { key: String, name: String },
    /// A value property bound to a binding of the wrong type.
    BindingTypeMismatch {
        name: String,
        expected: ValueType,
        found: ValueType,
    },
    /// An event handler that is a value, not a handler.
    HandlerUsedAsValue { name: String },
    /// A value binding used as an event handler.
    ValueUsedAsHandler { name: String },
    /// The kind does not emit this event.
    EventNotAllowed {
        key: String,
        kind: NodeKind,
        event: EventName,
    },
    /// The same event is bound twice on one node.
    DuplicateEvent { key: String, event: EventName },
}

impl fmt::Display for IrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VersionTooNew { found, supported } => write!(
                f,
                "document is {found:?} but this reader speaks at most {supported:?}"
            ),
            Self::MajorMismatch { found, supported } => write!(
                f,
                "document major {found} is not readable by major {supported}"
            ),
            Self::NoMigrationPath { minor } => {
                write!(f, "no migration step from minor {minor}")
            }
            Self::Json(msg) => write!(f, "not a UI IR document: {msg}"),
            Self::BadKey { key } => write!(f, "`{key}` is not a legal node key"),
            Self::DuplicateKey { key } => write!(f, "node key `{key}` is used twice"),
            Self::MissingRoot { root } => write!(f, "root `{root}` is not a node"),
            Self::UnknownChild { parent, child } => {
                write!(f, "`{parent}` claims child `{child}`, which does not exist")
            }
            Self::TwoParents {
                child,
                first,
                second,
            } => write!(f, "`{child}` is a child of both `{first}` and `{second}`"),
            Self::Cycle { at } => write!(f, "the slots of `{at}` lead back to it"),
            Self::Unreachable { key } => {
                write!(f, "`{key}` is not reachable from the root")
            }
            Self::DuplicateSlot { key, slot } => {
                write!(f, "`{key}` declares slot `{slot}` twice")
            }
            Self::DuplicateProp { key, prop } => {
                write!(f, "`{key}` sets `{prop}` twice")
            }
            Self::UnknownProp { key, prop } => {
                write!(f, "`{prop}` on `{key}` is not a known property")
            }
            Self::PropNotAllowed { key, kind, prop } => {
                write!(f, "`{prop}` is not a property of {kind} (`{key}`)")?;
                let owners: Vec<String> = crate::schema::kinds_declaring(prop)
                    .iter()
                    .map(|k| k.to_string())
                    .collect();
                if owners.is_empty() {
                    Ok(())
                } else {
                    write!(f, "; it belongs to {}", owners.join(", "))
                }
            }
            Self::WrongValueType {
                key,
                prop,
                expected,
                found,
            } => write!(
                f,
                "`{prop}` on `{key}` wants {expected}, got a {found} value"
            ),
            Self::ValueOutsideDomain { key, prop, value } => {
                write!(f, "`{prop}` on `{key}` does not accept `{value}`")
            }
            Self::DuplicateBinding { name } => write!(f, "binding `{name}` is declared twice"),
            Self::UnresolvedBinding { key, name } => {
                write!(f, "`{key}` refers to undeclared binding `{name}`")
            }
            Self::BindingTypeMismatch {
                name,
                expected,
                found,
            } => write!(
                f,
                "binding `{name}` is a {found} but the property wants {expected}"
            ),
            Self::HandlerUsedAsValue { name } => {
                write!(f, "`{name}` is an event handler, not a value")
            }
            Self::ValueUsedAsHandler { name } => {
                write!(f, "`{name}` is a value, not an event handler")
            }
            Self::EventNotAllowed { key, kind, event } => {
                write!(f, "{kind} (`{key}`) does not emit {event}")?;
                let emitted: Vec<String> = crate::schema::events_for(*kind)
                    .map(|e| e.to_string())
                    .collect();
                if emitted.is_empty() {
                    write!(f, "; it emits no events at all")
                } else {
                    write!(f, "; it emits {}", emitted.join(", "))
                }
            }
            Self::DuplicateEvent { key, event } => {
                write!(f, "`{key}` binds {event} twice")
            }
        }
    }
}

impl std::error::Error for IrError {}
