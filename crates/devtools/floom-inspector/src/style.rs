// SPDX-License-Identifier: MIT
//! The resolved-style snapshot.

use serde::{Deserialize, Serialize};

use crate::{JsonSnapshot, ProtocolVersion, Rgba};

/// The six states the style layer resolves. A closed set, by name, so a
/// frontend can spell them without linking the runtime.
pub const STYLE_STATES: [&str; 6] = [
    "normal", "hover", "pressed", "focused", "selected", "disabled",
];

pub fn is_known_style_state(name: &str) -> bool {
    STYLE_STATES.contains(&name)
}

/// The style properties on the wire — the runtime's `StyleProps` with `Color`
/// swapped for [`Rgba`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StylePropsDto {
    pub background: Option<Rgba>,
    pub foreground: Option<Rgba>,
    pub opacity: Option<f32>,
    pub font_size: Option<f32>,
    pub padding: Option<f32>,
    pub radius: Option<f32>,
    pub border_width: Option<f32>,
    pub border_color: Option<Rgba>,
    pub elevation: Option<f32>,
}

/// One element's resolved properties in one state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleEntry {
    pub id: u64,
    pub state: String,
    pub style: StylePropsDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StyleSnapshot {
    pub version: ProtocolVersion,
    pub entries: Vec<StyleEntry>,
}

impl JsonSnapshot for StyleSnapshot {}
