//! The design-importer boundary (Phase 26.2).
//!
//! Accepts *normalized* design data — the vocabulary an upstream
//! Figma-specific converter produces after it has handled tokens, files and
//! plugin quirks. This module knows nothing about Figma's REST shapes; it
//! knows the mapping table, and that is all it should know:
//!
//! | design                        | IR                              |
//! |-------------------------------|---------------------------------|
//! | auto-layout (axis, gap)       | `Stack` + `axis`/`gap`          |
//! | auto-layout padding           | `padding`                       |
//! | corner radius                 | `radius`                        |
//! | fill / stroke                 | `background` / `border_*`       |
//! | component variants            | `variant` (semantic, in-domain) |
//! | text run                      | `Text` + `label`/`size`         |
//! | text styles                   | contract typography tokens      |
//!
//! Everything emitted must survive [`UiDocument::validate`] — the importer is
//! not allowed to produce a document the IR would refuse, because a document
//! an agent cannot load is worse than no document at all.

use serde::{Deserialize, Serialize};

use crate::design_contract::{
    Accessibility, Animation, ColorRole, ContractVersion, DesignContract, Dimensions, Radius,
    Spacing, StateStyle, TypeStyle,
};
use crate::document::UiDocument;
use crate::node::{NodeKey, NodeKind, Prop, Slot, UiNode};
use crate::value::{PropValue, Rgba};

/// What a design node is, in the normalized vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignRole {
    /// A box: with auto-layout it maps to a `Stack`, without it a `Container`.
    Frame,
    /// A text run.
    Text,
    /// A labelled, variant-bearing control.
    Button,
}

/// Auto-layout direction, in the design's own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignAxis {
    Horizontal,
    Vertical,
}

/// Auto-layout: direction, spacing between children, spacing inside the box.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AutoLayout {
    pub axis: DesignAxis,
    pub gap: f32,
    pub padding: f32,
}

/// A design node's size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DesignSize {
    pub width: f32,
    pub height: f32,
}

/// A design node's stroke.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignStroke {
    pub width: f32,
    /// `#rrggbb[aa]`.
    pub color: String,
}

/// A text run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextSpec {
    pub content: String,
    pub font_size: f32,
}

/// One node of normalized design data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignNode {
    pub name: String,
    pub role: DesignRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_layout: Option<AutoLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<DesignSize>,
    /// `#rrggbb[aa]` — the design world's spelling; parsed on the way in.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fill: Option<String>,
    #[serde(default)]
    pub corner_radius: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stroke: Option<DesignStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<TextSpec>,
    /// The component's variants, spelled the way the IR's domain spells them
    /// (`filled`, `outlined`, …). Anything else is refused, not coerced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<DesignNode>,
}

/// A whole normalized design file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DesignFile {
    pub name: String,
    pub root: DesignNode,
}

/// Why a design file could not be imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImporterError {
    /// A variant name the IR's domain does not carry. The allowed set rides
    /// along so the design tool can fix its data without reading our source.
    UnknownVariant {
        node: String,
        variant: String,
        allowed: Vec<String>,
    },
    /// Two nodes would map to the same key. Keys are identity, so the
    /// importer refuses rather than uniquifying silently.
    DuplicateKey { key: String },
    /// A name that cannot become a legal `NodeKey` segment.
    BadName { node: String, name: String },
    /// A `Button` without a text run has nothing to announce.
    MissingText { node: String },
    /// A colour the design side spelled in a way we cannot parse.
    BadColor { node: String, value: String },
}

impl std::fmt::Display for ImporterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImporterError::UnknownVariant {
                node,
                variant,
                allowed,
            } => write!(
                f,
                "`{node}` declares variant {variant:?}; allowed: {}",
                allowed.join(", ")
            ),
            ImporterError::DuplicateKey { key } => {
                write!(f, "two nodes map to the key {key:?}; keys are identity")
            }
            ImporterError::BadName { node, name } => {
                write!(
                    f,
                    "node {node:?} has a name {name:?} that cannot become a key"
                )
            }
            ImporterError::MissingText { node } => {
                write!(f, "button {node:?} has no text to announce")
            }
            ImporterError::BadColor { node, value } => {
                write!(
                    f,
                    "node {node:?} has a colour {value:?} that is not #rgb/#rrggbb/#rrggbbaa"
                )
            }
        }
    }
}

impl std::error::Error for ImporterError {}

/// What an import produces: the document *and* the contract the design agrees
/// to. Both halves come from the same walk, so they cannot disagree.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedDesign {
    pub document: UiDocument,
    pub contract: DesignContract,
}

/// The variant domain the IR declares, verbatim from the property table —
/// the importer does not restate it.
const VARIANT_DOMAIN: &[&str] = &["filled", "outlined", "text", "elevated", "tonal"];

/// Import a normalized design file.
///
/// The walk is depth-first in child order, so key assignment, colour-role
/// naming and the contract's lists are all deterministic: importing the same
/// file twice produces byte-identical documents.
pub fn import(file: &DesignFile) -> Result<ImportedDesign, ImporterError> {
    let mut nodes: Vec<UiNode> = Vec::new();
    let mut used_keys: Vec<String> = Vec::new();
    let mut colors: Vec<ColorRole> = Vec::new();
    let mut typography: Vec<TypeStyle> = Vec::new();
    let mut states: Vec<StateStyle> = Vec::new();

    let root_key = walk(
        &file.root,
        true,
        &mut nodes,
        &mut used_keys,
        &mut colors,
        &mut typography,
        &mut states,
    )?;

    let document = UiDocument {
        version: crate::IrVersion::CURRENT,
        root: root_key,
        bindings: Vec::new(),
        nodes,
    };

    let (width, height) = file
        .root
        .size
        .map(|s| (s.width, s.height))
        .unwrap_or((0.0, 0.0));
    let (gap, padding) = file
        .root
        .auto_layout
        .map(|l| (l.gap, l.padding))
        .unwrap_or((0.0, 0.0));
    let contract = DesignContract {
        version: ContractVersion::CURRENT,
        name: file.name.clone(),
        dimensions: Dimensions { width, height },
        spacing: Spacing {
            gap,
            // One design padding value expands to the CSS order.
            padding: [padding; 4],
        },
        radius: Radius {
            corner: file.root.corner_radius,
        },
        typography,
        colors,
        states,
        animation: Animation {
            duration_ms: 0,
            easing: "none".into(),
        },
        accessibility: Accessibility {
            role: role_name(file.root.role).to_string(),
            label: file.root.name.clone(),
            // The WCAG AA floor and the 44 pt touch target: the defaults a
            // design that says nothing still promises.
            min_contrast: 4.5,
            min_touch_target: 44.0,
        },
    };

    Ok(ImportedDesign { document, contract })
}

/// Walk one design node into IR nodes, collecting contract material on the
/// way. `is_root` decides whether the node's key is used as-is or nested.
#[allow(clippy::too_many_arguments)]
fn walk(
    design: &DesignNode,
    is_root: bool,
    nodes: &mut Vec<UiNode>,
    used_keys: &mut Vec<String>,
    colors: &mut Vec<ColorRole>,
    typography: &mut Vec<TypeStyle>,
    states: &mut Vec<StateStyle>,
) -> Result<NodeKey, ImporterError> {
    let key = key_for(design, is_root, used_keys)?;
    used_keys.push(key.as_str().to_string());

    // Style properties shared by every role.
    let mut props: Vec<Prop> = Vec::new();
    if let Some(hex) = &design.fill {
        let fill = parse_hex(hex, &design.name)?;
        props.push(Prop::new(
            style_prop_for_role(design.role),
            PropValue::Color(fill),
        ));
        // Colour roles are the distinct fills, named after their first user —
        // so two nodes sharing a fill share a role, which is the point of a
        // role.
        if !colors.iter().any(|c| c.value == fill) {
            colors.push(ColorRole {
                name: design.name.to_lowercase(),
                value: fill,
            });
        }
    }
    if design.corner_radius > 0.0 {
        props.push(Prop::new(
            "radius",
            PropValue::Number(design.corner_radius as f64),
        ));
    }
    if let Some(stroke) = &design.stroke {
        props.push(Prop::new(
            "border_width",
            PropValue::Number(stroke.width as f64),
        ));
        let color = parse_hex(&stroke.color, &design.name)?;
        props.push(Prop::new("border_color", PropValue::Color(color)));
    }
    if let Some(size) = &design.size {
        props.push(Prop::new("width", PropValue::Number(size.width as f64)));
        props.push(Prop::new("height", PropValue::Number(size.height as f64)));
    }

    let kind = match design.role {
        DesignRole::Frame => {
            // Auto-layout is the Flex intent; a box without it is a plain
            // container. Padding only appears when the design actually pads.
            if let Some(layout) = &design.auto_layout {
                props.push(Prop::new(
                    "axis",
                    PropValue::Ident(axis_name(layout.axis).to_string()),
                ));
                props.push(Prop::new("gap", PropValue::Number(layout.gap as f64)));
                if layout.padding > 0.0 {
                    props.push(Prop::new(
                        "padding",
                        PropValue::Number(layout.padding as f64),
                    ));
                }
                NodeKind::Stack
            } else {
                NodeKind::Container
            }
        }
        DesignRole::Text => {
            let text = design
                .text
                .as_ref()
                .ok_or_else(|| ImporterError::MissingText {
                    node: design.name.clone(),
                })?;
            props.push(Prop::new("label", PropValue::Text(text.content.clone())));
            // The text style becomes a named typography token in the contract,
            // and `size` on the node — the IR's font-size spelling for `Text`.
            props.push(Prop::new("size", PropValue::Number(text.font_size as f64)));
            typography.push(TypeStyle {
                name: design.name.to_lowercase(),
                font_size: text.font_size,
                line_height: None,
                weight: None,
            });
            NodeKind::Text
        }
        DesignRole::Button => {
            let text = design
                .text
                .as_ref()
                .ok_or_else(|| ImporterError::MissingText {
                    node: design.name.clone(),
                })?;
            props.push(Prop::new("label", PropValue::Text(text.content.clone())));
            for variant in &design.variants {
                if !VARIANT_DOMAIN.contains(&variant.as_str()) {
                    return Err(ImporterError::UnknownVariant {
                        node: design.name.clone(),
                        variant: variant.clone(),
                        allowed: VARIANT_DOMAIN.iter().map(|s| s.to_string()).collect(),
                    });
                }
                if !states.iter().any(|s| s.name == *variant) {
                    states.push(StateStyle {
                        name: variant.clone(),
                        overrides: Vec::new(),
                    });
                }
            }
            // A single declared variant is the node's variant property; more
            // than one is the component's state set, and the node takes the
            // first — the contract carries the whole set.
            if let Some(first) = design.variants.first() {
                props.push(Prop::new("variant", PropValue::Ident(first.clone())));
            }
            NodeKind::Button
        }
    };

    let mut node = UiNode::new(key.clone(), kind).with_props(props);
    if !design.children.is_empty() {
        let children: Result<Vec<NodeKey>, _> = design
            .children
            .iter()
            .map(|child| walk(child, false, nodes, used_keys, colors, typography, states))
            .collect();
        node = node.with_slot(Slot::default_slot(children?));
    }
    nodes.push(node);
    Ok(key)
}

fn style_prop_for_role(role: DesignRole) -> &'static str {
    // A text node's fill is its foreground; every other fill is a background.
    match role {
        DesignRole::Text => "foreground",
        _ => "background",
    }
}

fn parse_hex(value: &str, node: &str) -> Result<Rgba, ImporterError> {
    Rgba::from_hex(value).ok_or_else(|| ImporterError::BadColor {
        node: node.to_string(),
        value: value.to_string(),
    })
}

fn axis_name(axis: DesignAxis) -> &'static str {
    match axis {
        DesignAxis::Horizontal => "horizontal",
        DesignAxis::Vertical => "vertical",
    }
}

fn role_name(role: DesignRole) -> &'static str {
    match role {
        DesignRole::Frame => "frame",
        DesignRole::Text => "text",
        DesignRole::Button => "button",
    }
}

/// The key a node mounts as: its name, lowercased with spaces to hyphens.
/// Refuses rather than repairs — a design with unnamed or colliding nodes
/// needs fixing where it was drawn.
fn key_for(design: &DesignNode, is_root: bool, used: &[String]) -> Result<NodeKey, ImporterError> {
    if is_root {
        return Ok(NodeKey::new(design.name.as_str()));
    }
    let key = design.name.to_lowercase().replace(' ', "-");
    if !NodeKey::is_valid(&key) {
        return Err(ImporterError::BadName {
            node: design.name.clone(),
            name: key,
        });
    }
    if used.iter().any(|k| k == &key) {
        return Err(ImporterError::DuplicateKey { key });
    }
    Ok(NodeKey::new(key))
}
