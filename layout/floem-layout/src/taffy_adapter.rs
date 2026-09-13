//! Taffy-backed layout engine with opaque [`LayoutNode`] handles.

use std::collections::HashMap;

use floem_types::{Point, Rect, Size as GeoSize};
use taffy::prelude::{
    AlignItems as TAlign, AvailableSpace, Dimension, Display, FlexDirection as TFlex,
    JustifyContent as TJustify, LengthPercentage, LengthPercentageAuto, NodeId,
    Position as TPosition, Style, TaffyTree,
};

/// Opaque layout node (not a Taffy `NodeId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutNode(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexDirection {
    #[default]
    Row,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum JustifyContent {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum AlignItems {
    #[default]
    Stretch,
    Start,
    Center,
    End,
}

/// Whether a node participates in its parent's flow.
///
/// Overlays (dialogs, popups, bottom sheets) must escape the flow of whatever
/// box they are mounted in, otherwise they are laid out as one more sibling
/// row/column item and cannot cover ancestor content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Position {
    #[default]
    Relative,
    Absolute,
}

/// Offsets of an absolutely positioned box from its containing block.
///
/// `None` means `auto`: taffy resolves it from the box's own size, so leaving
/// both sides auto centres along that axis (with `justify_content` deciding).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AbsoluteInset {
    pub left: Option<f32>,
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
}

impl AbsoluteInset {
    pub const ZERO: Self = Self {
        left: Some(0.0),
        top: Some(0.0),
        right: Some(0.0),
        bottom: Some(0.0),
    };

    /// Fills the containing block on every side.
    pub const fn fill() -> Self {
        Self::ZERO
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub flex_grow: f32,
    pub gap: f32,
    pub position: Position,
    pub inset: AbsoluteInset,
}

impl Default for LayoutStyle {
    fn default() -> Self {
        Self {
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::Start,
            align_items: AlignItems::Stretch,
            width: None,
            height: None,
            flex_grow: 0.0,
            gap: 0.0,
            position: Position::Relative,
            inset: AbsoluteInset::default(),
        }
    }
}

impl LayoutStyle {
    /// A column that grows to fill whatever box it was given.
    pub fn column() -> Self {
        Self {
            flex_direction: FlexDirection::Column,
            flex_grow: 1.0,
            ..Self::default()
        }
    }

    /// A row that grows to fill whatever box it was given.
    pub fn row() -> Self {
        Self {
            flex_grow: 1.0,
            ..Self::default()
        }
    }

    /// A box removed from its parent's flow and anchored to all four sides,
    /// i.e. covering the whole containing block.
    pub fn absolute_fill() -> Self {
        Self {
            position: Position::Absolute,
            inset: AbsoluteInset::fill(),
            ..Self::default()
        }
    }
}

impl LayoutStyle {
    fn to_taffy(&self) -> Style {
        Style {
            display: Display::Flex,
            flex_direction: match self.flex_direction {
                FlexDirection::Row => TFlex::Row,
                FlexDirection::Column => TFlex::Column,
            },
            justify_content: Some(match self.justify_content {
                JustifyContent::Start => TJustify::Start,
                JustifyContent::Center => TJustify::Center,
                JustifyContent::End => TJustify::End,
                JustifyContent::SpaceBetween => TJustify::SpaceBetween,
            }),
            align_items: Some(match self.align_items {
                AlignItems::Stretch => TAlign::Stretch,
                AlignItems::Start => TAlign::Start,
                AlignItems::Center => TAlign::Center,
                AlignItems::End => TAlign::End,
            }),
            size: taffy::Size {
                width: self
                    .width
                    .map(Dimension::length)
                    .unwrap_or(Dimension::auto()),
                height: self
                    .height
                    .map(Dimension::length)
                    .unwrap_or(Dimension::auto()),
            },
            flex_grow: self.flex_grow,
            gap: taffy::Size {
                width: LengthPercentage::length(self.gap),
                height: LengthPercentage::length(self.gap),
            },
            position: match self.position {
                Position::Relative => TPosition::Relative,
                Position::Absolute => TPosition::Absolute,
            },
            inset: taffy::Rect {
                left: auto_or_length(self.inset.left),
                top: auto_or_length(self.inset.top),
                right: auto_or_length(self.inset.right),
                bottom: auto_or_length(self.inset.bottom),
            },
            ..Style::DEFAULT
        }
    }
}

/// `auto` when unset, otherwise a fixed offset from the containing block edge.
fn auto_or_length(value: Option<f32>) -> LengthPercentageAuto {
    value
        .map(LengthPercentageAuto::length)
        .unwrap_or(LengthPercentageAuto::auto())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOutput {
    pub rect: Rect,
}

pub type MeasureFn = Box<dyn Fn(GeoSize) -> GeoSize + Send + Sync>;

/// Layout engine wrapping Taffy without exposing its node IDs.
pub struct LayoutEngine {
    tree: TaffyTree<()>,
    /// Slot per [`LayoutNode`]; `None` once the node has been removed. Slots are
    /// reused so handles stay stable across insert/remove churn.
    nodes: Vec<Option<NodeId>>,
    free: Vec<usize>,
    reverse: HashMap<NodeId, LayoutNode>,
    measure: HashMap<LayoutNode, MeasureFn>,
}

impl Default for LayoutEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            nodes: Vec::new(),
            free: Vec::new(),
            reverse: HashMap::new(),
            measure: HashMap::new(),
        }
    }

    pub fn new_leaf(&mut self, style: LayoutStyle) -> LayoutNode {
        let id = self.tree.new_leaf(style.to_taffy()).expect("taffy leaf");
        self.register(id)
    }

    pub fn new_leaf_with_measure(&mut self, style: LayoutStyle, measure: MeasureFn) -> LayoutNode {
        let node = self.new_leaf(style);
        self.measure.insert(node, measure);
        node
    }

    pub fn new_with_children(&mut self, style: LayoutStyle, children: &[LayoutNode]) -> LayoutNode {
        let child_ids: Vec<NodeId> = children.iter().map(|n| self.nid(*n)).collect();
        let id = self
            .tree
            .new_with_children(style.to_taffy(), &child_ids)
            .expect("taffy children");
        self.register(id)
    }

    pub fn set_style(&mut self, node: LayoutNode, style: LayoutStyle) {
        let id = self.nid(node);
        self.tree
            .set_style(id, style.to_taffy())
            .expect("set_style");
    }

    pub fn add_child(&mut self, parent: LayoutNode, child: LayoutNode) {
        let p = self.nid(parent);
        let c = self.nid(child);
        self.tree.add_child(p, c).expect("add_child");
    }

    /// Inserts `child` at `index` among `parent`'s children.
    ///
    /// Needed by the reconciler: appending would reorder nodes that survived a
    /// diff, and order is part of what the user sees.
    pub fn insert_child_at_index(&mut self, parent: LayoutNode, index: usize, child: LayoutNode) {
        let p = self.nid(parent);
        let c = self.nid(child);
        self.tree
            .insert_child_at_index(p, index, c)
            .expect("insert_child_at_index");
    }

    /// Detaches `child` from `parent`; `child` stays alive as its own root.
    pub fn remove_child(&mut self, parent: LayoutNode, child: LayoutNode) {
        let p = self.nid(parent);
        let c = self.nid(child);
        self.tree.remove_child(p, c).expect("remove_child");
    }

    /// Replaces `parent`'s entire child list, in order.
    ///
    /// The reconciler uses this to push the diffed, key-ordered children in one
    /// shot: an append-only `add_child` would leave re-ordered survivors in the
    /// wrong slot, and taffy keeps child order as the visual order.
    pub fn set_children(&mut self, parent: LayoutNode, children: &[LayoutNode]) {
        let p = self.nid(parent);
        let child_ids: Vec<NodeId> = children.iter().map(|n| self.nid(*n)).collect();
        self.tree.set_children(p, &child_ids).expect("set_children");
    }

    /// Detaches `node` from its parent (if any) and drops it from the tree.
    ///
    /// Every descendant must have been detached first: `remove_child` is what
    /// keeps this engine's bookkeeping and taffy's tree in step.
    pub fn remove(&mut self, node: LayoutNode) {
        let id = self.nid(node);
        let slot = node.0 as usize;
        // Detach from whatever still claims it before dropping it, or taffy
        // leaves a dangling child pointer in the old parent.
        if let Some(parent) = self.tree.parent(id) {
            let _ = self.tree.remove_child(parent, id);
        }
        let _ = self.tree.remove(id);
        self.nodes[slot] = None;
        self.reverse.remove(&id);
        self.measure.remove(&node);
        self.free.push(slot);
    }

    pub fn compute(&mut self, root: LayoutNode, available: GeoSize) {
        let id = self.nid(root);
        let avail = taffy::Size {
            width: AvailableSpace::Definite(available.width as f32),
            height: AvailableSpace::Definite(available.height as f32),
        };

        // Collect measure results keyed by NodeId for the closure.
        let measures: HashMap<NodeId, GeoSize> = self
            .measure
            .iter()
            .map(|(ln, f)| {
                let nid = self.nid(*ln);
                (nid, f(available))
            })
            .collect();

        self.tree
            .compute_layout_with_measure(id, avail, move |known, _avail, node_id, _ctx, _style| {
                if let Some(size) = measures.get(&node_id) {
                    return taffy::Size {
                        width: known.width.unwrap_or(size.width as f32),
                        height: known.height.unwrap_or(size.height as f32),
                    };
                }
                taffy::Size {
                    width: known.width.unwrap_or(0.0),
                    height: known.height.unwrap_or(0.0),
                }
            })
            .expect("compute_layout");
    }

    pub fn layout(&self, node: LayoutNode) -> LayoutOutput {
        let id = self.nid(node);
        let l = self.tree.layout(id).expect("layout");
        LayoutOutput {
            rect: Rect::from_origin_size(
                Point::new(l.location.x as f64, l.location.y as f64),
                GeoSize::new(l.size.width as f64, l.size.height as f64),
            ),
        }
    }

    /// Whether `node` still exists. Handles to removed nodes are stale but not
    /// dangling: asking for them again returns `false`.
    pub fn contains(&self, node: LayoutNode) -> bool {
        self.nodes.get(node.0 as usize).is_some_and(|n| n.is_some())
    }

    /// Debug/test helper: ensure no `NodeId` type is part of the public surface.
    pub fn node_count(&self) -> usize {
        self.nodes.len() - self.free.len()
    }

    fn nid(&self, node: LayoutNode) -> NodeId {
        self.nodes[node.0 as usize].expect("layout node was removed")
    }

    fn register(&mut self, id: NodeId) -> LayoutNode {
        let slot = self.free.pop().unwrap_or_else(|| {
            self.nodes.push(None);
            self.nodes.len() - 1
        });
        self.nodes[slot] = Some(id);
        let node = LayoutNode(slot as u32);
        self.reverse.insert(id, node);
        node
    }
}
