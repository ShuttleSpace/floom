use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;

use floom_ui_composites::{TreeNode, TreeView, TreeViewStyle};
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::treeview::{TreeviewNode, TreeviewProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

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
    crate::widgets::icon::MdIcon::new(name).resolve().into()
}

/// Material's tree view.
pub struct MdTreeview {
    props: TreeviewProps,
    theme: RwSignal<MultiTheme>,
    tree: floom_ui_composites::SharedTree,
    on_node_click: Option<std::sync::Arc<dyn Fn(String) + Send + Sync + 'static>>,
}

impl MdTreeview {
    pub fn new() -> Self {
        let props = TreeviewProps::default();
        MdTreeview {
            tree: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Material,
            )),
            props,
            on_node_click: None,
        }
    }

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
            icon_scale: 0.7,
            font_size: tokens.font_size,
            label_color: to_paint(theme.colors().on_surface),
            hover_background: to_paint(floom_core_theme::tokens::color_ext::with_alpha(
                theme.colors().on_surface,
                8,
            )),
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

impl Default for MdTreeview {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for `floom-elements` compatibility.
pub type MdTreeviewStyle = TreeViewStyle;
