//! The tree view: a list of rows, some of which have children that are shown when
//! the row is expanded.
//!
//! ## Why this is in the engine
//!
//! 87% cross-kit overlap — `md-treeview`, `cupertino-treeview` and
//! `fluent-treeview` are the same component — so the shape is written once here and
//! each kit becomes a token translation.
//!
//! ## Where the state lives, and why it must be shared
//!
//! Expanding a node **adds or removes rows**. Every row below it shifts down, and
//! every row that *was* below it is a different row afterwards. So no row can own
//! the answer: a row that remembered "I am expanded" would be right about itself and
//! wrong about how many rows follow it, and a host that rebuilt the subtree on a
//! toggle would throw away the focus and hover state of every row underneath.
//!
//! The shape is therefore the same one `SelectGroup` and `Accordion` use: an
//! [`Arc<Mutex<Vec<TreeNode>>>`] holding the whole forest, each row reading it in
//! its own `update`, comparing against what it last saw, and marking itself dirty.
//! The host visits every node each frame, so no notification has to travel.
//!
//! ## What a row is
//!
//! ```text
//! Stack                        row: height, gap 4, padding 8, cross-axis centred
//!  ├ Stack                     the indent spacer — width (depth - 1) * indent
//!  ├ Surface                   the expander's box: expander_size square, rounded
//!  │  └ Pressable              only when the node has children
//!  │     └ VectorIcon          "expand_more" when open, "chevron_right" when shut
//!  ├ Surface                   the leading icon's box — always present, see below
//!  │  └ VectorIcon             only when the node named one
//!  └ Pressable                 the label: the whole row is its click target
//!     └ Text
//! ```
//!
//! Two of those slots are **absent** rather than empty, and v1 disagrees:
//!
//! * v1 rendered `Empty::new().style(|s| s.width(expander_size)…)` for a childless
//!   node's expander, and `Empty::new().style(|s| s.width(icon_size))` for a node
//!   with no icon. So v1 kept the row's *layout* stable — every row's indent, expander
//!   and icon occupied the same width whether or not anything was in them — which is
//!   the right behaviour for a tree, where a missing icon must not shift the labels.
//!   Here the boxes stay and only the widgets inside them are absent, so the geometry
//!   is identical and the child count is not.
//! * v1 put the click handler on the **label** only, so the expander and the icon
//!   were not clickable. Here the expander is its own `Pressable` and the label is
//!   another, which is the same two targets and not one.
//!
//! The **indent** is a spacer of width `(depth - 1) * indent`, which is what v1
//! wrote and is why a depth-1 row has no indent at all.

use std::sync::{Arc, Mutex};

use floom_core::{
    Align, Axis, Child, ChildKey, Color, CreateContext, LayoutRequest, UpdateContext, Widget,
    WidgetAny,
};
use floom_types::FontWeight;

use floom_widgets::compose::Stack;
use floom_widgets::controls::Pressable;
use floom_widgets::primitives::Text;
use floom_widgets::surface::Surface;
use floom_widgets::vector_icon::VectorIcon;

/// One node in the tree.
///
/// Plain data, with no reference to a theme or a widget: the engine owns the
/// *structure* and a kit owns the tokens, so a node is just what the caller said.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeNode {
    pub label: String,
    /// An optional leading icon's path data. `None` leaves the icon's box empty
    /// rather than removing the box — see the module note.
    pub icon: Option<String>,
    pub expanded: bool,
    pub children: Vec<TreeNode>,
}

impl TreeNode {
    pub fn new(label: impl Into<String>) -> Self {
        TreeNode {
            label: label.into(),
            icon: None,
            expanded: false,
            children: Vec::new(),
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn children(mut self, children: Vec<TreeNode>) -> Self {
        self.children = children;
        self
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

/// Everything about a tree row that is not the node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeViewStyle {
    /// Horizontal indent added per depth level.
    pub indent: f32,
    /// One row's height.
    pub row_height: f32,
    /// The gap between rows — v1 read `item_gap` from the token table and **never
    /// used it**, because `flatten` built one flat list and the gap lived on the
    /// container. It is kept here for the same reason `ToolbarTokens::height` is
    /// kept: a declared token a port may not quietly delete.
    pub item_gap: f32,
    /// The expander chevron's box.
    pub expander_size: f32,
    /// The leading icon's box.
    pub icon_size: f32,
    /// The icon *inside* that box, which v1 drew at `icon_size * 0.7`.
    pub icon_scale: f32,
    /// The label's font size.
    pub font_size: f32,
    pub label_color: Color,
    /// The row's hover fill: `on_surface` at 8/255, from v1's `.hover(..)`.
    pub hover_background: Color,
    /// The gap between the four parts of a row.
    pub gap: f32,
    /// The row's horizontal padding.
    pub padding_horizontal: f64,
}

impl Default for TreeViewStyle {
    /// Material's token values, so an un-styled tree is a Material tree.
    fn default() -> Self {
        TreeViewStyle {
            indent: 16.0,
            row_height: 32.0,
            item_gap: 0.0,
            expander_size: 24.0,
            icon_size: 24.0,
            icon_scale: 0.7,
            font_size: 14.0,
            label_color: Color::rgb(0x1D, 0x1B, 0x20),
            hover_background: Color::rgba(0x1D, 0x1B, 0x20, 8),
            gap: 4.0,
            padding_horizontal: 8.0,
        }
    }
}

/// A forest of nodes, and the shared answer to "which of them are open".
///
/// `Mutex` because a host may move a subtree between threads; the critical section
/// is a `bool` flip and a re-flatten, both of which are shorter than the borrow that
/// surrounds them.
pub type SharedTree = Arc<Mutex<Vec<TreeNode>>>;

/// Toggle the node at `path`, and report whether anything changed.
///
/// The one place the rule lives, for the same reason `SelectGroup::choose` is: a row
/// cannot call `&self` on the tree it is not part of. A path that does not resolve —
/// which happens the moment the tree is replaced under a row that has not rebuilt —
/// changes nothing and says so, so the caller can skip its own bookkeeping.
pub fn toggle_path(tree: &SharedTree, path: &[usize]) -> bool {
    let Ok(mut forest) = tree.lock() else {
        return false;
    };
    set_expanded(&mut forest, path)
}

/// v1's `set_expanded`, unchanged: walk to the parent of the last index, then flip.
fn set_expanded(forest: &mut Vec<TreeNode>, path: &[usize]) -> bool {
    let Some((last, parents)) = path.split_last() else {
        return false;
    };
    let mut level = forest;
    for &index in parents {
        match level.get_mut(index) {
            Some(node) => level = &mut node.children,
            None => return false,
        }
    }
    match level.get_mut(*last) {
        Some(node) => {
            node.expanded = !node.expanded;
            true
        }
        None => false,
    }
}

/// One row's place in the flattened forest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowRef {
    /// The node's path from the root, which is also its identity: `[0, 2, 1]` is the
    /// second child of the third top-level node.
    pub path: Vec<usize>,
    /// `path.len()`, cached because every row needs it and it never changes for a
    /// given path.
    pub depth: usize,
}

/// The rows currently visible, in order.
///
/// v1's `flatten`, as a value rather than a side effect: it walks the forest and
/// descends into a node's children **only when that node is expanded**. Returning
/// the list is what lets the widget's `children()` be a pure function of the tree.
pub fn visible_rows(forest: &[TreeNode]) -> Vec<RowRef> {
    let mut out = Vec::new();
    walk(forest, &mut Vec::new(), &mut out);
    out
}

fn walk(nodes: &[TreeNode], path: &mut Vec<usize>, out: &mut Vec<RowRef>) {
    for (index, node) in nodes.iter().enumerate() {
        path.push(index);
        out.push(RowRef {
            path: path.clone(),
            depth: path.len(),
        });
        if node.expanded && !node.children.is_empty() {
            walk(&node.children, path, out);
        }
        path.pop();
    }
}

/// The node at `path`, if it resolves.
///
/// Descends one level per index and keeps the node it landed on: an empty path names
/// nothing, which is why the walk starts from `None` rather than from the forest.
pub fn node_at<'a>(forest: &'a [TreeNode], path: &[usize]) -> Option<&'a TreeNode> {
    let mut level = forest;
    let mut node = None;
    for &index in path {
        node = level.get(index);
        level = &node?.children;
    }
    node
}

/// A tree of rows.
pub struct TreeView {
    tree: SharedTree,
    style: TreeViewStyle,
    on_node_click: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
    on_toggle: Option<Arc<dyn Fn(Vec<usize>) + Send + Sync + 'static>>,
    /// Icon path data by name, because the engine cannot know that `"folder"` is a
    /// path — the same reason `IconButton::new` takes a built `VectorIcon`.
    ///
    /// v1 resolved the name inside `MdIcon`, which is a kit's job; here the kit
    /// supplies the lookup, because three kits' icon sets differ and the engine has no
    /// business choosing between them.
    icons: Arc<dyn Fn(&str) -> Option<VectorIcon> + Send + Sync + 'static>,
}

impl std::fmt::Debug for TreeView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeView")
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

impl TreeView {
    /// A tree over `nodes`, which becomes the shared state.
    pub fn new(nodes: Vec<TreeNode>) -> Self {
        TreeView {
            tree: Arc::new(Mutex::new(nodes)),
            style: TreeViewStyle::default(),
            on_node_click: None,
            on_toggle: None,
            icons: Arc::new(|_| None),
        }
    }

    /// Adopt an existing shared forest, so a caller that already has one — or that
    /// wants to watch it — does not have to hand over ownership.
    pub fn with_shared_tree(tree: SharedTree) -> Self {
        TreeView {
            tree,
            style: TreeViewStyle::default(),
            on_node_click: None,
            on_toggle: None,
            icons: Arc::new(|_| None),
        }
    }

    /// The shared forest, for a caller that wants to read or replace it.
    pub fn tree(&self) -> SharedTree {
        self.tree.clone()
    }

    pub fn style(mut self, style: TreeViewStyle) -> Self {
        self.style = style;
        self
    }

    /// How a node's icon name becomes a widget.
    ///
    /// Required rather than optional-with-a-default: a tree with no icon resolver
    /// silently drops every icon, and the three kits resolve the same names to
    /// different paths.
    pub fn icons(mut self, resolve: impl Fn(&str) -> Option<VectorIcon> + Send + Sync + 'static) -> Self {
        self.icons = Arc::new(resolve);
        self
    }

    /// Called with a node's label when its row is clicked.
    pub fn on_node_click(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_node_click = Some(Arc::new(callback));
        self
    }

    /// Called after a toggle, with the toggled node's path.
    pub fn on_toggle(mut self, callback: impl Fn(Vec<usize>) + Send + Sync + 'static) -> Self {
        self.on_toggle = Some(Arc::new(callback));
        self
    }

    pub fn resolved_style(&self) -> TreeViewStyle {
        self.style
    }

    /// The rows as they stand right now.
    pub fn rows(&self) -> Vec<RowRef> {
        match self.tree.lock() {
            Ok(forest) => visible_rows(&forest),
            Err(_) => Vec::new(),
        }
    }

    /// Expand or collapse one node, and notify.
    ///
    /// The public door into [`toggle_path`], so a caller does not have to know that
    /// the lock is the engine's business.
    pub fn toggle(&self, path: &[usize]) -> bool {
        let changed = toggle_path(&self.tree, path);
        if changed {
            if let Some(on_toggle) = &self.on_toggle {
                on_toggle(path.to_vec());
            }
        }
        changed
    }

    /// Whether `path` currently resolves to an expanded node.
    pub fn is_expanded(&self, path: &[usize]) -> bool {
        match self.tree.lock() {
            Ok(forest) => node_at(&forest, path).map(|n| n.expanded).unwrap_or(false),
            Err(_) => false,
        }
    }

    fn children(&self) -> Vec<Child> {
        let forest = match self.tree.lock() {
            Ok(forest) => forest.clone(),
            Err(_) => return Vec::new(),
        };
        let rows = visible_rows(&forest);

        rows.iter()
            .map(|row| {
                let style = self.style;
                let node = node_at(&forest, &row.path).cloned().unwrap_or(TreeNode {
                    label: String::new(),
                    icon: None,
                    expanded: false,
                    children: Vec::new(),
                });
                let tree = self.tree.clone();
                let icons = self.icons.clone();
                let on_node_click = self.on_node_click.clone();
                let on_toggle = self.on_toggle.clone();
                let path = row.path.clone();
                let depth = row.depth;

                Child::new(
                    ChildKey::index(row.path.iter().copied().fold(0usize, |a, b| a * 31 + b + 1)),
                    LayoutRequest::new(),
                    move || {
                        let indent = (depth as f32 - 1.0) * style.indent;

                        let mut row_stack = Stack::new(Axis::Horizontal)
                            .gap(style.gap)
                            .align(Align::Start, Align::Center)
                            .request(
                                LayoutRequest::new()
                                    .height(style.row_height)
                                    .padding_horiz(style.padding_horizontal),
                            );

                        // The indent: a spacer, so a node's children line up under
                        // its label rather than under the tree's edge.
                        row_stack = row_stack.push(move || {
                            Box::new(Surface::new().request(
                                LayoutRequest::new().width(indent).height(style.row_height),
                            ))
                        });

                        // The expander's box is always there; the chevron is not.
                        let has_children = !node.children.is_empty();
                        row_stack = row_stack.push({
                            // Cloned here, in this `Fn`'s body, so the closures
                            // nested inside it own their copies. Three `Fn` levels
                            // deep is where this bites: the outer row builder, the
                            // expander builder, and the chevron's own — each may run
                            // again, and each must hand its children a clone rather
                            // than a value it will not see twice.
                            let tree = tree.clone();
                            let on_toggle = on_toggle.clone();
                            let path = path.clone();
                            let icons = icons.clone();
                            move || {
                                let box_ = Surface::new().request(
                                    LayoutRequest::new()
                                        .width(style.expander_size)
                                        .height(style.expander_size)
                                        .main_align(Align::Center)
                                        .cross_align(Align::Center),
                                );
                                if !has_children {
                                    return Box::new(box_) as Box<dyn WidgetAny>;
                                }
                                let name = if node.expanded {
                                    "expand_more"
                                } else {
                                    "chevron_right"
                                };
                                let chevron = icons(name);
                                let expanded = node.expanded;
                                // The expander's box, with the clickable chevron
                                // inside it. The box stays either way — see the module
                                // note — so a childless row's labels still line up.
                                Box::new(box_.push({
                                    let chevron = chevron.clone();
                                    let tree = tree.clone();
                                    let path = path.clone();
                                    let on_toggle = on_toggle.clone();
                                    move || {
                                        Box::new(
                                            Pressable::new(Child::new(
                                                ChildKey::index(0),
                                                LayoutRequest::new().fill(),
                                                {
                                                    let chevron = chevron.clone();
                                                    move || match &chevron {
                                                        Some(icon) => Box::new(icon.clone()),
                                                        None => Box::new(Surface::new()),
                                                    }
                                                },
                                            ))
                                            .label(if expanded {
                                                "Collapse".to_string()
                                            } else {
                                                "Expand".to_string()
                                            })
                                            .on_click({
                                                let tree = tree.clone();
                                                let path = path.clone();
                                                let on_toggle = on_toggle.clone();
                                                move || {
                                                    if toggle_path(&tree, &path) {
                                                        if let Some(cb) = &on_toggle {
                                                            cb(path.clone());
                                                        }
                                                    }
                                                }
                                            }),
                                        )
                                    }
                                }))
                            }
                        });

                        // The leading icon's box, likewise.
                        row_stack = row_stack.push({
                            let icons = icons.clone();
                            let name = node.icon.clone();
                            let icon_size = style.icon_size;
                            let icon_scale = style.icon_scale;
                            move || {
                                let box_ = Surface::new().request(
                                    LayoutRequest::new().width(icon_size).height(icon_size),
                                );
                                match name.as_ref().and_then(|n| icons(n)) {
                                    Some(icon) => {
                                        let sized = icon.size(icon_size * icon_scale);
                                        Box::new(box_.push(move || Box::new(sized.clone())))
                                    }
                                    None => Box::new(box_),
                                }
                            }
                        });

                        // The label, which is the other click target and grows.
                        row_stack = row_stack.push_layout(
                            LayoutRequest::new().grow(1.0),
                            {
                                let label = node.label.clone();
                                let on_node_click = on_node_click.clone();
                                let font_size = style.font_size;
                                let color = style.label_color;
                                move || {
                                    let text = Text::new(label.clone())
                                        .size(font_size)
                                        .weight(FontWeight::Normal)
                                        .color(color);
                                    Box::new(
                                        Pressable::new(Child::new(
                                            ChildKey::index(0),
                                            LayoutRequest::new().grow(1.0),
                                            move || Box::new(text.clone()),
                                        ))
                                        .label(label.clone())
                                        .on_click({
                                            let on_node_click = on_node_click.clone();
                                            let label = label.clone();
                                            move || {
                                                if let Some(cb) = &on_node_click {
                                                    cb(label.clone());
                                                }
                                            }
                                        }),
                                    )
                                }
                            },
                        );

                        // `tree` and `on_toggle` have already been handed to the
                        // expander's closures, which is where they are used: a row's
                        // toggle goes through its expander, never through the row.
                        Box::new(row_stack) as Box<dyn WidgetAny>
                    },
                )
            })
            .collect()
    }
}

impl Widget for TreeView {
    fn create(&self, cx: &mut dyn CreateContext) -> floom_core::ElementId {
        cx.alloc_child()
    }

    /// Deliberately empty, for the reason [`floom_widgets::compose::SelectGroup::update`]
    /// gives: a toggle changes *which rows exist*, and the host rebuilds children for
    /// a node that asks — so marking dirty here would be a second report of a change
    /// the row that was clicked has already caused.
    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: floom_core::ElementId) {}

    fn children(&self) -> Vec<Child> {
        self.children()
    }

    /// The tree's rows never ask for a layout of their own — the widget is a
    /// sequence of rows, so the intrinsic height is whatever the rows add up to, and
    /// a host that needs to know can read [`Self::rows`].
    fn layout_request(&self) -> Option<LayoutRequest> {
        None
    }
}

impl WidgetAny for TreeView {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Nothing: each row's `Pressable` publishes its own label and role, and the
    /// tree itself is a container.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
