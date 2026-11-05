// SPDX-License-Identifier: MIT
//! The headless inspector (Phase 23.2).
//!
//! Navigates a retained [`LiveTree`] the way a devtools panel would: find a
//! node by key, walk to the root, read its resolved style and layout request,
//! say where it came from in the source, list the semantics tree the host
//! holds, serve the wire format the protocol defines, and answer "why did
//! this update?" — which signal fired, and which ancestors the invalidation
//! propagates to.
//!
//! Boundary: this crate inspects; it does not drive. The semantics tree and
//! the scene graph belong to the host (it composes them each frame), so the
//! inspector takes them by value/reference rather than re-deriving its own
//! copy of either.

#![forbid(unsafe_code)]

pub mod panel;
pub mod pick;
pub mod profiler;
pub mod tree;

use std::collections::HashMap;

use floom_accessibility::{SemanticsRole, SemanticsTree};
use floom_core::{Align, Axis, ElementKind, LayoutRequest, Position};
use floom_hot_reload::{ElementRef, LiveTree, NodeKey};
use floom_inspector::{
    LayoutDto, LayoutEntry, LayoutSnapshot, ProtocolVersion, SceneLines, SourceLocation,
    StyleEntry, StylePropsDto, StyleSnapshot, TreeNodeInfo, TreeSnapshot,
};
use floom_runtime_core::{BindingRegistry, BindingReport};
use floom_style::{StyleProps, StyleState};
use floom_types::ElementId;
use floom_dsl::ui_ir::NodeKind;

/// The style states an inspector reports, in precedence order.
pub const INSPECTED_STATES: [StyleState; 6] = [
    StyleState::NORMAL,
    StyleState::HOVER,
    StyleState::PRESSED,
    StyleState::FOCUSED,
    StyleState::SELECTED,
    StyleState::DISABLED,
];

fn state_name(state: StyleState) -> &'static str {
    if state == StyleState::NORMAL {
        "normal"
    } else if state == StyleState::HOVER {
        "hover"
    } else if state == StyleState::PRESSED {
        "pressed"
    } else if state == StyleState::FOCUSED {
        "focused"
    } else if state == StyleState::SELECTED {
        "selected"
    } else if state == StyleState::DISABLED {
        "disabled"
    } else {
        "normal"
    }
}

/// The element tier an IR node mounts as. The retained core only knows these
/// three; the mapping is the inspector's because it is the only reader that
/// spells them out.
fn element_kind_of(kind: NodeKind) -> ElementKind {
    if matches!(kind, NodeKind::Native(_)) {
        ElementKind::Composition
    } else if kind.is_container() {
        ElementKind::Host
    } else {
        ElementKind::Primitive
    }
}

fn kind_name(kind: ElementKind) -> &'static str {
    match kind {
        ElementKind::Host => "host",
        ElementKind::Primitive => "primitive",
        ElementKind::Composition => "composition",
    }
}

fn axis_name(axis: Axis) -> &'static str {
    match axis {
        Axis::Horizontal => "horizontal",
        Axis::Vertical => "vertical",
    }
}

fn align_name(align: Align) -> &'static str {
    match align {
        Align::Start => "start",
        Align::Center => "center",
        Align::End => "end",
        Align::Stretch => "stretch",
        Align::SpaceBetween => "space-between",
    }
}

fn position_name(position: Position) -> &'static str {
    match position {
        Position::Relative => "relative",
        Position::Absolute => "absolute",
    }
}

/// The inspector over one live tree.
pub struct Inspector<'a> {
    live: &'a LiveTree,
    sources: HashMap<NodeKey, SourceLocation>,
    semantics: Option<SemanticsTree>,
}

impl<'a> Inspector<'a> {
    pub fn new(live: &'a LiveTree) -> Self {
        Self {
            live,
            sources: HashMap::new(),
            semantics: None,
        }
    }

    // --- navigation ---------------------------------------------------------

    /// The id a key is mounted as.
    pub fn id_of(&self, key: &NodeKey) -> Option<ElementId> {
        self.live.id_of(key)
    }

    /// Document-order children of a key.
    pub fn children(&self, key: &NodeKey) -> Option<Vec<NodeKey>> {
        let id = self.live.id_of(key)?;
        let elements = self.live.elements();
        let element = elements.iter().find(|e| e.id == id)?;
        Some(
            element
                .children
                .iter()
                .filter_map(|c| self.live.key_of(*c).cloned())
                .collect(),
        )
    }

    /// The walk from `key` up to the root, `key` first, root last.
    pub fn path_to_root(&self, key: &NodeKey) -> Option<Vec<NodeKey>> {
        let elements = self.live.elements();
        let by_id: HashMap<u64, &ElementRef> = elements.iter().map(|e| (e.id.raw(), e)).collect();
        let start = self.live.id_of(key)?;
        let mut path = vec![key.clone()];
        let mut current = by_id.get(&start.raw())?.parent;
        while let Some(parent) = current {
            let element = by_id.get(&parent.raw())?;
            path.push(element.key.clone());
            current = element.parent;
        }
        Some(path)
    }

    /// Every key mounted from elements of `kind`.
    pub fn find_by_kind(&self, kind: ElementKind) -> Vec<NodeKey> {
        self.live
            .elements()
            .into_iter()
            .filter(|e| element_kind_of(e.kind) == kind)
            .map(|e| e.key)
            .collect()
    }

    // --- property inspection ------------------------------------------------

    /// The resolved style of a key at one interaction state.
    pub fn style_of(&self, key: &NodeKey, state: StyleState) -> Option<StyleProps> {
        self.live.style_of(key, state)
    }

    /// The layout request recorded for a key.
    pub fn layout_of(&self, key: &NodeKey) -> Option<&LayoutRequest> {
        self.live.layout_of(key)
    }

    // --- source mapping -----------------------------------------------------

    /// The host records where a node was written (DSL span, proc macro, …).
    pub fn set_source(&mut self, key: NodeKey, source: SourceLocation) {
        self.sources.insert(key, source);
    }

    pub fn source_of(&self, key: &NodeKey) -> Option<&SourceLocation> {
        self.sources.get(key)
    }

    // --- semantics ----------------------------------------------------------

    /// The host hands over the semantics tree it composed this frame.
    pub fn set_semantics(&mut self, tree: SemanticsTree) {
        self.semantics = Some(tree);
    }

    /// The semantics entries, sorted by element id for a stable panel order.
    pub fn semantics(&self) -> Vec<SemanticsEntry> {
        let Some(tree) = &self.semantics else {
            return Vec::new();
        };
        let mut entries: Vec<SemanticsEntry> = tree
            .iter()
            .map(|(id, node)| SemanticsEntry {
                element: id,
                role: node.role,
                label: node.label.clone(),
            })
            .collect();
        entries.sort_by_key(|e| e.element.raw());
        entries
    }

    // --- why did this update? ------------------------------------------------

    /// The update path for an element: every reactive binding targeting it
    /// (with the signal that fired, the app-level label, the property it
    /// drives, and how many times it has run), plus the ancestor chain a
    /// layout invalidation would propagate to.
    pub fn why_did_update(&self, registry: &BindingRegistry, element: ElementId) -> UpdatePath {
        let bindings: Vec<BindingReport> = registry.reports_for_element(element);
        let ancestors = self
            .live
            .key_of(element)
            .cloned()
            .and_then(|key| self.path_to_root(&key))
            .map(|mut path| {
                path.remove(0); // the element itself is not its own ancestor
                path
            })
            .unwrap_or_default();
        UpdatePath {
            element,
            bindings,
            ancestors,
        }
    }

    // --- protocol snapshots ---------------------------------------------------

    /// The element tree as the protocol serves it: raw ids, parent links,
    /// child order, keys and source locations.
    pub fn tree_snapshot(&self) -> TreeSnapshot {
        let elements = self.live.elements();
        let root = elements
            .iter()
            .find(|e| e.parent.is_none())
            .map(|e| e.id.raw())
            .unwrap_or(0);
        let nodes = elements
            .iter()
            .map(|e| TreeNodeInfo {
                id: e.id.raw(),
                kind: kind_name(element_kind_of(e.kind)).to_string(),
                key: Some(e.key.as_str().to_string()),
                source: self.sources.get(&e.key).cloned(),
                parent: e.parent.map(|p| p.raw()),
                children: e.children.iter().map(|c| c.raw()).collect(),
            })
            .collect();
        TreeSnapshot {
            version: ProtocolVersion::current(),
            root,
            nodes,
        }
    }

    /// Resolved styles for every element across the inspected states.
    pub fn style_snapshot(&self) -> StyleSnapshot {
        let mut entries = Vec::new();
        for e in self.live.elements() {
            for state in INSPECTED_STATES {
                let Some(resolved) = self.live.style_of(&e.key, state) else {
                    continue;
                };
                let style = StylePropsDto {
                    background: resolved.background.map(Into::into),
                    foreground: resolved.foreground.map(Into::into),
                    opacity: resolved.opacity,
                    font_size: resolved.font_size,
                    padding: resolved.padding,
                    radius: resolved.radius,
                    border_width: resolved.border_width,
                    border_color: resolved.border_color.map(Into::into),
                    elevation: resolved.elevation,
                };
                entries.push(StyleEntry {
                    id: e.id.raw(),
                    state: state_name(state).to_string(),
                    style,
                });
            }
        }
        StyleSnapshot {
            version: ProtocolVersion::current(),
            entries,
        }
    }

    /// Layout requests for every element.
    pub fn layout_snapshot(&self) -> LayoutSnapshot {
        let entries = self
            .live
            .elements()
            .iter()
            .filter_map(|e| {
                let layout = self.live.layout_of(&e.key)?;
                Some(LayoutEntry {
                    id: e.id.raw(),
                    layout: LayoutDto {
                        width: layout.width.map(|l| l.px().unwrap_or(0.0)),
                        height: layout.height.map(|l| l.px().unwrap_or(0.0)),
                        flex_grow: layout.flex_grow,
                        axis: layout.axis.map(|a| axis_name(a).to_string()),
                        gap: layout.gap,
                        main_align: layout.main_align.map(|a| align_name(a).to_string()),
                        cross_align: layout.cross_align.map(|a| align_name(a).to_string()),
                        position: layout.position.map(|p| position_name(p).to_string()),
                        inset: layout.inset.map(|i| [i.left, i.top, i.right, i.bottom]),
                    },
                })
            })
            .collect();
        LayoutSnapshot {
            version: ProtocolVersion::current(),
            entries,
        }
    }

    /// Serve a scene the host composed as the protocol's scene snapshot.
    pub fn scene_lines(&self, lines: Vec<String>) -> SceneLines {
        SceneLines {
            version: ProtocolVersion::current(),
            lines,
        }
    }
}

/// One semantics node as the viewer lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsEntry {
    pub element: ElementId,
    pub role: SemanticsRole,
    pub label: String,
}

/// The answer to "why did this update?".
#[derive(Debug, Clone)]
pub struct UpdatePath {
    pub element: ElementId,
    /// Bindings targeting the element, registration order.
    pub bindings: Vec<BindingReport>,
    /// The ancestor chain a layout invalidation propagates to, parent first.
    pub ancestors: Vec<NodeKey>,
}
