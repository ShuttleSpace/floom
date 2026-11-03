//! Typed, versioned, serializable schema for native-component codegen (Phase 18.1).
//!
//! This is the tooling-facing IDL that the `floom-codegen` backends consume.
//! It deliberately mirrors the runtime native-composition vocabulary in
//! `floom-types` (`NativeViewKind`, `NativeCompositionMode`, `InputMode`) but
//! is its own serializable copy, so the compiler crate stays decoupled from
//! the runtime and the schema can be serialized for tooling only (Step 3).
//! `From` bridges convert between the two vocabularies.

use floom_types::{
    InputMode as RuntimeInputMode, NativeCompositionMode as RuntimeComposition,
    NativeViewKind as RuntimeViewKind,
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A validated identifier (Rust/foreign-name safe).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ident(pub String);

impl Ident {
    /// Validate ASCII identifier syntax: starts with a letter or `_`, then
    /// letters/digits/`_` only.
    pub fn new(s: &str) -> Result<Self, SchemaError> {
        if s.is_empty() {
            return Err(SchemaError::EmptyIdent);
        }
        let mut chars = s.chars();
        let first = chars.next().unwrap();
        if !(first.is_ascii_alphabetic() || first == '_') {
            return Err(SchemaError::BadIdentStart(s.to_string()));
        }
        for c in s.chars() {
            if !(c.is_ascii_alphanumeric() || c == '_') {
                return Err(SchemaError::BadIdentChar(c, s.to_string()));
            }
        }
        Ok(Ident(s.to_string()))
    }
}

/// ABI version. Major increments break compatibility; minor increments are
/// backward-compatible additions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiVersion {
    pub major: u32,
    pub minor: u32,
}

impl AbiVersion {
    /// A producer `self` satisfies a consumer `required` when majors match and
    /// the producer's minor is at least the required minor.
    pub fn compatible(&self, required: AbiVersion) -> bool {
        self.major == required.major && self.minor >= required.minor
    }
}

/// A reference to a primitive or user-defined type in the generated glue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeRef {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    String,
    /// `Vec<u8>` / raw bytes (e.g. a frozen-pixel buffer).
    Bytes,
    /// A `ResourceId`-typed handle owned by the renderer (texture, image).
    Resource(Ident),
    /// An opaque native handle the OS side owns.
    Handle(Ident),
    Option(Box<TypeRef>),
    Vec(Box<TypeRef>),
    /// A user-defined type the backend must already know about.
    Custom(Ident),
}

/// What kind of native view a participant is — mirror of `floom_types::NativeViewKind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewKind {
    Map,
    Video,
    Web,
    /// A native text field; the framework must yield IME ownership.
    TextInput,
    Custom(String),
}

/// How a participant is composited — mirror of `floom_types::NativeCompositionMode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Composition {
    Overlay,
    Texture,
    Surface,
    Offscreen,
}

/// Whether a participant takes pointer input itself — mirror of `floom_types::InputMode`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputMode {
    Passthrough,
    Capture,
}

impl From<ViewKind> for RuntimeViewKind {
    fn from(v: ViewKind) -> Self {
        match v {
            ViewKind::Map => RuntimeViewKind::Map,
            ViewKind::Video => RuntimeViewKind::Video,
            ViewKind::Web => RuntimeViewKind::Web,
            ViewKind::TextInput => RuntimeViewKind::TextInput,
            ViewKind::Custom(s) => RuntimeViewKind::Custom(Box::leak(s.into_boxed_str())),
        }
    }
}

impl From<RuntimeViewKind> for ViewKind {
    fn from(v: RuntimeViewKind) -> Self {
        match v {
            RuntimeViewKind::Map => ViewKind::Map,
            RuntimeViewKind::Video => ViewKind::Video,
            RuntimeViewKind::Web => ViewKind::Web,
            RuntimeViewKind::TextInput => ViewKind::TextInput,
            RuntimeViewKind::Custom(s) => ViewKind::Custom(s.to_string()),
        }
    }
}

impl From<Composition> for RuntimeComposition {
    fn from(c: Composition) -> Self {
        match c {
            Composition::Overlay => RuntimeComposition::Overlay,
            Composition::Texture => RuntimeComposition::Texture,
            Composition::Surface => RuntimeComposition::Surface,
            Composition::Offscreen => RuntimeComposition::Offscreen,
        }
    }
}

impl From<RuntimeComposition> for Composition {
    fn from(c: RuntimeComposition) -> Self {
        match c {
            RuntimeComposition::Overlay => Composition::Overlay,
            RuntimeComposition::Texture => Composition::Texture,
            RuntimeComposition::Surface => Composition::Surface,
            RuntimeComposition::Offscreen => Composition::Offscreen,
        }
    }
}

impl From<InputMode> for RuntimeInputMode {
    fn from(i: InputMode) -> Self {
        match i {
            InputMode::Passthrough => RuntimeInputMode::Passthrough,
            InputMode::Capture => RuntimeInputMode::Capture,
        }
    }
}

impl From<RuntimeInputMode> for InputMode {
    fn from(i: RuntimeInputMode) -> Self {
        match i {
            RuntimeInputMode::Passthrough => InputMode::Passthrough,
            RuntimeInputMode::Capture => InputMode::Capture,
        }
    }
}

/// Read/write access for a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Access {
    Read,
    Write,
    ReadWrite,
}

/// A compile-time literal, used for property defaults.
///
/// Note: only `PartialEq` (not `Eq`) because `Float(f64)` is not `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}

/// A property exposed by the native component.
///
/// Only `PartialEq` (not `Eq`) because it carries an optional [`Literal`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertySchema {
    pub name: Ident,
    pub ty: TypeRef,
    pub access: Access,
    pub default: Option<Literal>,
}

/// A named parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Param {
    pub name: Ident,
    pub ty: TypeRef,
}

/// A command: a call the framework makes into native code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSchema {
    pub name: Ident,
    pub params: Vec<Param>,
    pub ret: Option<TypeRef>,
}

/// An event: a callback the native side raises into the framework.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventSchema {
    pub name: Ident,
    pub params: Vec<Param>,
}

/// The full definition of one native component — the unit of codegen.
///
/// Only `PartialEq` (not `Eq`) because it carries [`PropertySchema`], which in
/// turn carries an optional [`Literal`] that is not `Eq`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentSchema {
    pub name: Ident,
    pub abi: AbiVersion,
    pub kind: ViewKind,
    pub composition: Composition,
    pub input: InputMode,
    pub properties: Vec<PropertySchema>,
    pub commands: Vec<CommandSchema>,
    pub events: Vec<EventSchema>,
}

impl ComponentSchema {
    /// Validate identifier syntax and reject duplicate property/command/event
    /// names. Returns the first [`SchemaError`] found.
    pub fn validate(&self) -> Result<(), SchemaError> {
        if self.name.0.is_empty() {
            return Err(SchemaError::EmptyIdent);
        }
        Self::check_unique(
            "property",
            &self.properties.iter().map(|p| &p.name).collect::<Vec<_>>(),
        )?;
        Self::check_unique(
            "command",
            &self.commands.iter().map(|c| &c.name).collect::<Vec<_>>(),
        )?;
        Self::check_unique(
            "event",
            &self.events.iter().map(|e| &e.name).collect::<Vec<_>>(),
        )?;
        Ok(())
    }

    fn check_unique(kind: &str, names: &[&Ident]) -> Result<(), SchemaError> {
        let mut seen = std::collections::HashSet::new();
        for n in names {
            if !seen.insert(n.0.clone()) {
                return Err(SchemaError::DuplicateName(kind.to_string(), n.0.clone()));
            }
        }
        Ok(())
    }
}

/// Schema construction/validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaError {
    EmptyIdent,
    BadIdentStart(String),
    BadIdentChar(char, String),
    DuplicateName(String, String),
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchemaError::EmptyIdent => write!(f, "identifier must not be empty"),
            SchemaError::BadIdentStart(s) => {
                write!(f, "identifier `{s}` must start with a letter or `_`")
            }
            SchemaError::BadIdentChar(c, s) => {
                write!(f, "identifier `{s}` contains invalid character `{c}`")
            }
            SchemaError::DuplicateName(kind, name) => {
                write!(f, "duplicate {kind} name `{name}`")
            }
        }
    }
}

impl std::error::Error for SchemaError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ComponentSchema {
        ComponentSchema {
            name: Ident::new("VideoPlayer").unwrap(),
            abi: AbiVersion { major: 1, minor: 0 },
            kind: ViewKind::Video,
            composition: Composition::Texture,
            input: InputMode::Passthrough,
            properties: vec![
                PropertySchema {
                    name: Ident::new("src").unwrap(),
                    ty: TypeRef::String,
                    access: Access::ReadWrite,
                    default: Some(Literal::String("".into())),
                },
                PropertySchema {
                    name: Ident::new("muted").unwrap(),
                    ty: TypeRef::Bool,
                    access: Access::ReadWrite,
                    default: Some(Literal::Bool(false)),
                },
            ],
            commands: vec![
                CommandSchema {
                    name: Ident::new("play").unwrap(),
                    params: vec![],
                    ret: None,
                },
                CommandSchema {
                    name: Ident::new("seek").unwrap(),
                    params: vec![Param {
                        name: Ident::new("ms").unwrap(),
                        ty: TypeRef::U64,
                    }],
                    ret: None,
                },
            ],
            events: vec![
                EventSchema {
                    name: Ident::new("ended").unwrap(),
                    params: vec![],
                },
                EventSchema {
                    name: Ident::new("error").unwrap(),
                    params: vec![Param {
                        name: Ident::new("message").unwrap(),
                        ty: TypeRef::String,
                    }],
                },
            ],
        }
    }

    #[test]
    fn ident_validation() {
        assert!(Ident::new("ok_name1").is_ok());
        assert!(Ident::new("_ok").is_ok());
        assert!(Ident::new("").is_err());
        assert!(Ident::new("1bad").is_err());
        assert!(Ident::new("bad-x").is_err());
    }

    #[test]
    fn abi_compatibility() {
        // Same major, producer minor >= required minor: compatible.
        assert!(AbiVersion { major: 2, minor: 3 }.compatible(AbiVersion { major: 2, minor: 1 }));
        // Same major, producer minor < required minor: not compatible.
        assert!(!AbiVersion { major: 2, minor: 0 }.compatible(AbiVersion { major: 2, minor: 1 }));
        // Different major: never compatible.
        assert!(!AbiVersion { major: 1, minor: 9 }.compatible(AbiVersion { major: 2, minor: 0 }));
    }

    #[test]
    fn schema_json_roundtrip() {
        let c = sample();
        let json = serde_json::to_string_pretty(&c).expect("serialize");
        let back: ComponentSchema = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(c, back);
    }

    #[test]
    fn validate_rejects_duplicates() {
        let mut c = sample();
        c.commands.push(c.commands[0].clone());
        assert!(matches!(
            c.validate(),
            Err(SchemaError::DuplicateName(_, _))
        ));
        // Removing the dup passes.
        c.commands.pop();
        assert!(c.validate().is_ok());
    }

    #[test]
    fn vocab_bridges_roundtrip() {
        for k in [
            ViewKind::Map,
            ViewKind::Video,
            ViewKind::Web,
            ViewKind::TextInput,
        ] {
            assert_eq!(k, ViewKind::from(RuntimeViewKind::from(k.clone())));
        }
        assert_eq!(
            Composition::Surface,
            Composition::from(RuntimeComposition::from(Composition::Surface))
        );
        assert_eq!(
            InputMode::Capture,
            InputMode::from(RuntimeInputMode::from(InputMode::Capture))
        );
    }
}
