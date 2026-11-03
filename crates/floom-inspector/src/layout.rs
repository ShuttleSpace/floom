//! The layout-request snapshot.

use serde::{Deserialize, Serialize};

use crate::{JsonSnapshot, ProtocolVersion};

/// The layout properties on the wire — the runtime's `LayoutRequest` with its
/// enums spelled as strings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutDto {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub flex_grow: Option<f32>,
    /// `"horizontal"` or `"vertical"`.
    pub axis: Option<String>,
    pub gap: Option<f32>,
    pub main_align: Option<String>,
    pub cross_align: Option<String>,
    /// `"relative"` or `"absolute"`.
    pub position: Option<String>,
    /// `[left, top, right, bottom]`, `null` = auto.
    pub inset: Option<[Option<f32>; 4]>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutEntry {
    pub id: u64,
    pub layout: LayoutDto,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutSnapshot {
    pub version: ProtocolVersion,
    pub entries: Vec<LayoutEntry>,
}

impl JsonSnapshot for LayoutSnapshot {}
