//! Taffy-backed layout engine with opaque [`LayoutNode`] handles.

use std::collections::HashMap;

use floem_types::{Point, Rect, Size as GeoSize};
use taffy::prelude::{
    AlignItems as TAlign, AvailableSpace, Dimension, Display, FlexDirection as TFlex,
    JustifyContent as TJustify, LengthPercentage, NodeId, Style, TaffyTree,
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

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutStyle {
    pub flex_direction: FlexDirection,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub flex_grow: f32,
    pub gap: f32,
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
            ..Style::DEFAULT
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutOutput {
    pub rect: Rect,
}

pub type MeasureFn = Box<dyn Fn(GeoSize) -> GeoSize + Send + Sync>;

/// Layout engine wrapping Taffy without exposing its node IDs.
pub struct LayoutEngine {
    tree: TaffyTree<()>,
    nodes: Vec<NodeId>,
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

    pub fn new_with_children(
        &mut self,
        style: LayoutStyle,
        children: &[LayoutNode],
    ) -> LayoutNode {
        let child_ids: Vec<NodeId> = children.iter().map(|n| self.nodes[n.0 as usize]).collect();
        let id = self
            .tree
            .new_with_children(style.to_taffy(), &child_ids)
            .expect("taffy children");
        self.register(id)
    }

    pub fn set_style(&mut self, node: LayoutNode, style: LayoutStyle) {
        let id = self.nodes[node.0 as usize];
        self.tree
            .set_style(id, style.to_taffy())
            .expect("set_style");
    }

    pub fn add_child(&mut self, parent: LayoutNode, child: LayoutNode) {
        let p = self.nodes[parent.0 as usize];
        let c = self.nodes[child.0 as usize];
        self.tree.add_child(p, c).expect("add_child");
    }

    pub fn compute(&mut self, root: LayoutNode, available: GeoSize) {
        let id = self.nodes[root.0 as usize];
        let avail = taffy::Size {
            width: AvailableSpace::Definite(available.width as f32),
            height: AvailableSpace::Definite(available.height as f32),
        };

        // Collect measure results keyed by NodeId for the closure.
        let measures: HashMap<NodeId, GeoSize> = self
            .measure
            .iter()
            .map(|(ln, f)| {
                let nid = self.nodes[ln.0 as usize];
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
        let id = self.nodes[node.0 as usize];
        let l = self.tree.layout(id).expect("layout");
        LayoutOutput {
            rect: Rect::from_origin_size(
                Point::new(l.location.x as f64, l.location.y as f64),
                GeoSize::new(l.size.width as f64, l.size.height as f64),
            ),
        }
    }

    /// Debug/test helper: ensure no `NodeId` type is part of the public surface.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    fn register(&mut self, id: NodeId) -> LayoutNode {
        let node = LayoutNode(self.nodes.len() as u32);
        self.nodes.push(id);
        self.reverse.insert(id, node);
        node
    }
}
