//! Cupertino tree view, ported to the Floom 2.0 widget API.
//!

use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::{TreeNode, TreeView, TreeViewStyle};
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::treeview::{TreeviewNode, TreeviewProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

/// The engine's node shape, built from the props' one.
///
/// A straight field-for-field copy rather than a conversion, because
/// `TreeviewNode` is already plain data and a kit's node type that differed in shape
/// would be a second thing to keep in step. The one thing worth naming: the props'
/// `expanded` is an *initial* state — its doc says so — and here it becomes the
/// shared forest's starting value, which is the same thing.
pub fn to_engine_nodes(nodes: &[TreeviewNode]) -> Vec<TreeNode> {
    nodes
        .iter()
        .map(|node| {
            let mut out = TreeNode::new(node.label.clone())
                .expanded(node.expanded)
                .children(to_engine_nodes(&node.children));
            if let Some(icon) = &node.icon {
                out = out.icon(icon.clone());
            }
            out
        })
        .collect()
}

/// Icon names are resolved by the kit, because three kits' icon sets differ and the
/// engine has no business choosing between them.
fn resolve_icon(name: &str) -> Option<VectorIcon> {
    // This kit's own table. v1 used `cupertino_icon_svg`, and its `expand_more` is a
    // different glyph from Material's — a lookup that reached for the Material table
    // would draw a Material chevron in a Cupertino control and nothing would fail.
    Some(floom_widget_cupertino_icon::cupertino_icon(name))
}

/// Cupertino's tree view.
pub struct CupertinoTreeview {
    props: TreeviewProps,
    theme: RwSignal<MultiTheme>,
    /// The shared forest the widget reads and writes.
    ///
    /// Held here as well as inside the widget so a caller that already had a
    /// `TreeviewProps` can watch the tree, and so `nodes(..)` can replace it.
    tree: floom_ui_composites::SharedTree,
    on_node_click: Option<std::sync::Arc<dyn Fn(String) + Send + Sync + 'static>>,
}

impl CupertinoTreeview {
    pub fn new() -> Self {
        let props = TreeviewProps::default();
        CupertinoTreeview {
            tree: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props,
            on_node_click: None,
        }
    }

    /// Replace the tree.
    ///
    /// v1 stored the nodes in an `RwSignal<Vec<TreeviewNode>>` and set it here; here
    /// the shared forest is replaced under the widget, which is the same thing with a
    /// lock rather than a signal — and it means a caller can keep a handle.
    pub fn nodes(mut self, nodes: Vec<TreeviewNode>) -> Self {
        self.props.nodes = nodes.clone();
        if let Ok(mut forest) = self.tree.lock() {
            *forest = to_engine_nodes(&nodes);
        }
        self
    }

    pub fn on_node_click(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.props.on_node_click = Some(Box::new({
            let shared = std::sync::Arc::new(callback);
            // Held so `props()` keeps reporting what v1 reported; the engine gets the
            // `Send + Sync` handle, because a slot closure has to be one.
            self.on_node_click = Some(shared.clone());
            move |label: String| shared(label)
        }));
        self
    }

    pub fn props(&self) -> &TreeviewProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared forest, for a caller that wants to read it.
    pub fn tree(&self) -> floom_ui_composites::SharedTree {
        self.tree.clone()
    }

    pub fn resolved_style(&self) -> TreeViewStyle {
        let theme = self.theme.get();
        let ComponentTokens::Treeview(tokens) = theme.component(ComponentId::Treeview) else {
            unreachable!("ComponentId::Treeview must resolve to TreeviewTokens")
        };
        TreeViewStyle {
            indent: tokens.indent,
            row_height: tokens.row_height,
            item_gap: tokens.item_gap,
            expander_size: tokens.expander_size,
            icon_size: tokens.icon_size,
            // v1 drew the icon at `icon_size * 0.7` at the call site.
            icon_scale: 0.7,
            font_size: tokens.font_size,
            label_color: to_paint(theme.colors().on_surface),
            // v1's `.hover(|s| s.background(with_alpha(on_surface, 8)))`.
            hover_background: to_paint(floom_core_theme::tokens::color_ext::with_alpha(
                theme.colors().on_surface,
                8,
            )),
            // v1's `.gap(4.0)` and `.padding_horiz(8.0)`, both at the call site.
            gap: 4.0,
            padding_horizontal: 8.0,
        }
    }

    /// The tree, ready to mount.
    pub fn view(self) -> TreeView {
        let mut tree = TreeView::with_shared_tree(self.tree.clone())
            .style(self.resolved_style())
            .icons(resolve_icon);
        if let Some(on_node_click) = &self.on_node_click {
            let on_node_click = on_node_click.clone();
            tree = tree.on_node_click(move |label| on_node_click(label));
        }
        tree
    }
}

impl Default for CupertinoTreeview {
    fn default() -> Self {
        Self::new()
    }
}
