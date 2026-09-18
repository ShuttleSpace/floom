//! The AI-friendly component schema (Task 25.2).
//!
//! Emitted, never hand-written. Every row comes out of the same
//! `floem_ui_ir` tables the validator enforces, so an agent that reads this
//! schema authors documents the validator accepts — the two can never
//! disagree, because there is only one table. Examples and token
//! relationships are derived the same way: a token exists because the
//! resolved properties say the widget is styled by it, with the values the
//! domain allows.

use floem_inspector::{ProtocolVersion, STYLE_STATES};
use floem_ui_ir::{NodeKind, PropClass, all_prop_names, events_for, prop_spec};

use crate::protocol::PROTOCOL_VERSION;

/// The whole widget vocabulary, as an agent should see it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ComponentSchema {
    pub version: ProtocolVersion,
    pub widgets: Vec<WidgetInfo>,
}

/// One widget kind: what it accepts, what it emits, what it nests, and one
/// worked example.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WidgetInfo {
    pub name: String,
    /// Whether the kind can hold children.
    pub container: bool,
    /// The slots it nests content into — `"children"` for every container.
    pub slots: Vec<String>,
    pub properties: Vec<PropertyInfo>,
    pub events: Vec<String>,
    pub tokens: Vec<TokenRel>,
    /// A deterministic example built from the property table itself.
    pub example: String,
}

/// One property: its name, which layer stores it, what it accepts, and the
/// closed domain of values when there is one.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PropertyInfo {
    pub name: String,
    /// `"style"`, `"layout"` or `"widget"` — where the value is stored, which
    /// is also which side of the validator it lives on.
    pub class: String,
    pub value_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub domain: Vec<String>,
}

/// A design token a widget is styled by, with the values it allows.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TokenRel {
    pub token: String,
    pub values: Vec<String>,
}

/// Build the schema from the IR tables.
///
/// The property walk is O(kinds × names) by design: `prop_spec` is the one
/// authority on whether a `(kind, name)` pair exists, and consulting it for
/// every pair is cheaper than maintaining a second per-kind index that could
/// drift from the tables.
pub fn component_schema() -> ComponentSchema {
    let names = all_prop_names();
    let widgets = NodeKind::ALL
        .iter()
        .map(|kind| {
            let properties: Vec<PropertyInfo> = names
                .iter()
                .filter_map(|name| {
                    prop_spec(*kind, name).map(|spec| PropertyInfo {
                        name: (*name).to_string(),
                        class: class_name(spec.class).to_string(),
                        value_type: spec.ty.to_string(),
                        domain: spec.domain.iter().map(|d| d.to_string()).collect(),
                    })
                })
                .collect();
            let tokens = token_rels(&properties);
            let example = example_for(*kind, &properties);
            WidgetInfo {
                name: kind.to_string(),
                container: kind.is_container(),
                slots: kind
                    .is_container()
                    .then(|| vec!["children".to_string()])
                    .unwrap_or_default(),
                properties,
                events: events_for(*kind).map(|e| e.to_string()).collect(),
                tokens,
                example,
            }
        })
        .collect();
    ComponentSchema {
        version: PROTOCOL_VERSION,
        widgets,
    }
}

fn class_name(class: PropClass) -> &'static str {
    match class {
        PropClass::Style => "style",
        PropClass::Layout => "layout",
        PropClass::Widget => "widget",
    }
}

/// The design tokens `properties` say this widget is styled by.
///
/// Derived, not asserted: `variant` is a token of a widget exactly when the
/// widget declares a `variant` property, and its allowed values are the
/// property's own domain.
fn token_rels(properties: &[PropertyInfo]) -> Vec<TokenRel> {
    let mut rels = Vec::new();
    let styled = properties.iter().any(|p| p.class == "style");
    if styled {
        rels.push(TokenRel {
            token: "interaction_state".to_string(),
            values: STYLE_STATES.iter().map(|s| s.to_string()).collect(),
        });
    }
    let domain_of = |name: &str| {
        properties
            .iter()
            .find(|p| p.name == name)
            .map(|p| p.domain.clone())
            .unwrap_or_default()
    };
    if properties.iter().any(|p| p.name == "variant") {
        rels.push(TokenRel {
            token: "variant".to_string(),
            values: domain_of("variant"),
        });
    }
    if properties
        .iter()
        .any(|p| p.name == "size" && p.value_type == "Ident")
    {
        rels.push(TokenRel {
            token: "control_size".to_string(),
            values: domain_of("size"),
        });
    }
    rels
}

/// One worked example, built from the widget-class properties alone.
///
/// Identifiers quote their first allowed value; free text is a placeholder;
/// numbers and bools get neutral literals. Hand-writing these would be a
/// second vocabulary that drifts the moment a domain changes.
fn example_for(kind: NodeKind, properties: &[PropertyInfo]) -> String {
    let mut parts = Vec::new();
    for p in properties.iter().filter(|p| p.class == "widget") {
        let value = if !p.domain.is_empty() {
            format!("\"{}\"", p.domain[0])
        } else {
            match p.value_type.as_str() {
                "Text" => "\"text\"".to_string(),
                "Bool" => "false".to_string(),
                "Int" => "0".to_string(),
                _ => "0.0".to_string(),
            }
        };
        parts.push(format!("{}: {}", p.name, value));
    }
    if parts.is_empty() {
        kind.to_string()
    } else {
        format!("{} {{ {} }}", kind, parts.join(", "))
    }
}
