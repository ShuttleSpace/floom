/// A node in the tree, carrying an optional leading icon name and children.
#[derive(Debug, Clone)]
pub struct TreeviewNode {
    /// Text shown for the node.
    pub label: String,
    /// Optional leading icon name (resolved by the renderer's icon set).
    pub icon: Option<String>,
    /// Child nodes (rendered indented beneath this node).
    pub children: Vec<TreeviewNode>,
    /// Initial expand/collapse state.
    pub expanded: bool,
}

impl Default for TreeviewNode {
    fn default() -> Self {
        Self {
            label: String::new(),
            icon: None,
            children: Vec::new(),
            expanded: false,
        }
    }
}

/// Neutral, design-system-agnostic props for the tree view.
pub struct TreeviewProps {
    /// Root-level nodes.
    pub nodes: Vec<TreeviewNode>,
    /// Called with the label of the node that was clicked.
    pub on_node_click: Option<Box<dyn Fn(String)>>,
}

impl Clone for TreeviewProps {
    fn clone(&self) -> Self {
        Self {
            nodes: self.nodes.clone(),
            on_node_click: None,
        }
    }
}

impl std::fmt::Debug for TreeviewProps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeviewProps")
            .field("nodes", &self.nodes)
            .field("on_node_click", &self.on_node_click.as_ref().map(|_| "Fn(String)"))
            .finish()
    }
}

impl Default for TreeviewProps {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            on_node_click: None,
        }
    }
}
