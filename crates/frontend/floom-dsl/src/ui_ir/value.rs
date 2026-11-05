// SPDX-License-Identifier: MIT
//! Property values, bindings, and the serializable colour.

use serde::{Deserialize, Serialize};

use floom_types::Color;

/// An 8-bit RGBA colour, so a document reads the same in any tool.
///
/// `floom_types::Color` is the runtime type and deliberately does not implement
/// `serde` (it is L0 vocabulary that everything links); the IR carries its own
/// copy and bridges with `From`, exactly as `floom-codegen` mirrors the native
/// composition vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }

    /// `#rrggbb`, or `#rrggbbaa` when the alpha is not opaque — so the common
    /// case reads like the CSS everybody already knows.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (with or without the `#`).
    pub fn from_hex(s: &str) -> Option<Self> {
        let body = s.strip_prefix('#').unwrap_or(s);
        let byte = |i: usize| u8::from_str_radix(&body[i..i + 2], 16).ok();
        match body.len() {
            3 => {
                let nibble =
                    |i: usize| u8::from_str_radix(&body[i..i + 1], 16).ok().map(|v| v * 17);
                Some(Self::new(nibble(0)?, nibble(1)?, nibble(2)?, 255))
            }
            6 => Some(Self::new(byte(0)?, byte(2)?, byte(4)?, 255)),
            8 => Some(Self::new(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }
}

impl From<Rgba> for Color {
    fn from(v: Rgba) -> Self {
        Color::rgba(v.r, v.g, v.b, v.a)
    }
}

impl From<Color> for Rgba {
    fn from(c: Color) -> Self {
        Self::new(c.r, c.g, c.b, c.a)
    }
}

/// The tag of a value, used for type checking independent of the payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ValueType {
    Bool,
    Int,
    Number,
    Text,
    Color,
    Ident,
    List,
}

impl std::fmt::Display for ValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Bool => "Bool",
            Self::Int => "Int",
            Self::Number => "Number",
            Self::Text => "Text",
            Self::Color => "Color",
            Self::Ident => "Ident",
            Self::List => "List",
        };
        f.write_str(name)
    }
}

/// A literal property value, or a reference to a binding.
///
/// Every variant is explicitly tagged on the wire. Untagged numbers would let
/// `3` be either an `Int` or a `Number` depending on which type happened to be
/// tried first, and a property's declared type is precisely what the IR checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PropValue {
    Bool(bool),
    Int(i64),
    Number(f64),
    Text(String),
    Color(Rgba),
    /// A member of a small closed set (`axis = vertical`).
    Ident(String),
    List(Vec<PropValue>),
    /// `@name`: resolved against the document's [`Binding`] table.
    Ref(String),
    /// A computed expression lowered from inline TSX (`a ? b : c`, `x * 2`, …),
    /// carried as the Rust source that produces the value. Unlike [`PropValue::Ref`]
    /// it owns no binding: the emitter inlines it at every read site, so a
    /// per-frame closure recomputes it from the signals it names — the same
    /// pull-based reactivity the framework already uses for signal reads.
    Computed(String),
}

impl PropValue {
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "Bool",
            Self::Int(_) => "Int",
            Self::Number(_) => "Number",
            Self::Text(_) => "Text",
            Self::Color(_) => "Color",
            Self::Ident(_) => "Ident",
            Self::List(_) => "List",
            Self::Ref(_) => "Ref",
            Self::Computed(_) => "Computed",
        }
    }

    /// The type tag of this value. A reference or a computed expression has no
    /// type of its own; the caller resolves the former against the binding
    /// table and the latter is only ever inlined, never type-checked here.
    pub fn value_type(&self) -> Option<ValueType> {
        Some(match self {
            Self::Bool(_) => ValueType::Bool,
            Self::Int(_) => ValueType::Int,
            Self::Number(_) => ValueType::Number,
            Self::Text(_) => ValueType::Text,
            Self::Color(_) => ValueType::Color,
            Self::Ident(_) => ValueType::Ident,
            Self::List(_) => ValueType::List,
            Self::Ref(_) => return None,
            Self::Computed(_) => return None,
        })
    }

    pub fn as_ref_name(&self) -> Option<&str> {
        match self {
            Self::Ref(name) => Some(name),
            _ => None,
        }
    }

    pub fn as_ident(&self) -> Option<&str> {
        match self {
            Self::Ident(name) => Some(name),
            _ => None,
        }
    }
}

/// What a binding holds.
///
/// The value/handler split is load-bearing: a property reference and an event
/// handler are different namespaces, and a document that uses one where the
/// other belongs is a genuine authoring bug rather than a style question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingKind {
    Value(ValueType),
    Handler,
}

/// A named slot in the document's binding table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Binding {
    pub name: String,
    pub kind: BindingKind,
    /// The literal a value binding starts at, when the source declaration
    /// carried one (`createSignal("hi")`). `None` means "the type's default",
    /// which is what a DSL `bindings { title: Text }` row says. Keeping this
    /// next to the binding rather than on the usage site is what lets the
    /// emitter seed the signal with the author's value: a reactive read that
    /// starts at `""` paints an empty label until something writes it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub init: Option<PropValue>,
}

impl Binding {
    pub fn value(name: impl Into<String>, ty: ValueType) -> Self {
        Self {
            name: name.into(),
            kind: BindingKind::Value(ty),
            init: None,
        }
    }

    /// A value binding that starts at a known literal.
    pub fn value_with_init(
        name: impl Into<String>,
        ty: ValueType,
        init: impl Into<Option<PropValue>>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: BindingKind::Value(ty),
            init: init.into(),
        }
    }

    pub fn handler(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BindingKind::Handler,
            init: None,
        }
    }
}
