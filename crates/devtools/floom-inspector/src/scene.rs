// SPDX-License-Identifier: MIT
//! The painted-scene snapshot.

use serde::{Deserialize, Serialize};

use crate::{JsonSnapshot, ProtocolVersion};

/// The scene as text — one line per node, the scene graph's own `Debug`
/// rendering, exactly what the golden-file engine (Phase 24.2) hashes.
///
/// Text rather than a re-modelled node tree: the scene graph already has a
/// canonical spelling, and re-modelling it here would be a second
/// implementation of the same shape to keep in step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneLines {
    pub version: ProtocolVersion,
    pub lines: Vec<String>,
}

impl JsonSnapshot for SceneLines {}
