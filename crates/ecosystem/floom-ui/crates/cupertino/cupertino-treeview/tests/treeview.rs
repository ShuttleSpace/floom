//! The ported Cupertino tree view.
//!
//! The engine suite (`floom-widgets/tests/tree_view.rs`) owns the structure and the
//! shared expansion state; this suite owns the translation, which is Cupertino's six
//! numbers, its colours, and its icon names.

use floom_core::Widget;
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::treeview::TreeviewNode;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_ui_composites::{visible_rows};
use floom_ui_composites::{TreeNode, TreeView};

use floom_widget_cupertino_treeview::{CupertinoTreeview, to_engine_nodes};

/// Activate Material and hand back a theme that will read it.
///
/// `MultiTheme::new_for(ds)` does **not** select `ds` — it reads a process-global at
/// every access (the porting plan's correction 9).
fn cupertino() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// A three-level tree, both parents expanded so all four rows are visible.
fn props_nodes() -> Vec<TreeviewNode> {
    vec![TreeviewNode {
        label: "root".into(),
        icon: None,
        expanded: true,
        children: vec![TreeviewNode {
            label: "child".into(),
            icon: Some("folder".into()),
            expanded: true,
            children: vec![TreeviewNode {
                label: "grandchild".into(),
                icon: None,
                expanded: false,
                children: Vec::new(),
            }],
        }],
    }]
}

fn tree() -> CupertinoTreeview {
    let _ = cupertino();
    CupertinoTreeview::new().nodes(props_nodes())
}

fn to_paint(c: floom_core_theme::tokens::color::Color) -> floom_types::Color {
    floom_types::Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

// --- the tokens -----------------------------------------------------------

/// The six scalar tokens, read from Cupertino's table.
#[test]
fn the_tokens_reach_the_engine_style() {
    let theme = cupertino();
    let style = tree().resolved_style();
    let ComponentTokens::Treeview(tokens) = theme.component(ComponentId::Treeview) else {
        unreachable!("ComponentId::Treeview resolves to TreeviewTokens")
    };

    assert_eq!(style.indent, tokens.indent, "the per-level indent");
    assert_eq!(style.row_height, tokens.row_height, "the row's height");
    assert_eq!(style.expander_size, tokens.expander_size, "the chevron's box");
    assert_eq!(style.icon_size, tokens.icon_size, "the icon's box");
    assert_eq!(style.font_size, tokens.font_size, "the label's size");
    assert_eq!(
        style.item_gap, tokens.item_gap,
        "which v1 never read — kept, and unused"
    );

    assert_eq!(
        style.label_color,
        to_paint(theme.colors().on_surface),
        "v1's `.color(on_surface)` on the label"
    );
    assert_eq!(
        style.hover_background,
        to_paint(floom_core_theme::tokens::color_ext::with_alpha(
            theme.colors().on_surface,
            8
        )),
        "and v1's `.hover(with_alpha(on_surface, 8))`"
    );

    // The three literals v1 wrote at their call sites rather than reading tokens.
    assert_eq!(style.gap, 4.0, "v1's `.gap(4.0)`");
    assert_eq!(style.padding_horizontal, 8.0, "and `.padding_horiz(8.0)`");
    assert_eq!(style.icon_scale, 0.7, "and `icon_size * 0.7`");
}

/// `item_gap` is in the style and **not used**, because v1 read it nowhere. Asserting
/// that it is *present* is the point: a port that deleted the field would be hiding a
/// decision rather than making one.
#[test]
fn the_unused_token_is_carried_rather_than_deleted() {
    let theme = cupertino();
    let ComponentTokens::Treeview(tokens) = theme.component(ComponentId::Treeview) else {
        unreachable!()
    };
    assert_eq!(
        tree().resolved_style().item_gap,
        tokens.item_gap,
        "the token exists, the style carries it, and nothing reads it — exactly as \\
         in v1"
    );
}

// --- the node translation -------------------------------------------------

/// Field for field, including the nesting and the initial `expanded`.
#[test]
fn the_props_nodes_become_engine_nodes() {
    let nodes = to_engine_nodes(&props_nodes());
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].label, "root");
    assert!(nodes[0].expanded, "the props' `expanded` is the initial state");
    assert!(nodes[0].icon.is_none(), "and a node with no icon says so");

    let child = &nodes[0].children[0];
    assert_eq!(child.label, "child");
    assert_eq!(child.icon.as_deref(), Some("folder"), "the icon name is carried");
    assert_eq!(
        child.children[0].label, "grandchild",
        "and the nesting survives to the third level"
    );
    assert!(
        !child.children[0].expanded,
        "a leaf's initial state is carried too"
    );
}

/// The control for the assertion above: the two node types really are different, so
/// the translation is doing something rather than aliasing.
#[test]
fn the_two_node_types_are_different_shapes() {
    let props = props_nodes();
    let engine = to_engine_nodes(&props);
    assert_eq!(
        props.len(),
        engine.len(),
        "the same number of top-level nodes"
    );
    assert_ne!(
        std::any::type_name_of_val(&props[0]),
        std::any::type_name_of_val(&engine[0]),
        "CONTROL: two distinct types, so the mapping is a real conversion"
    );
}

// --- the tree -------------------------------------------------------------

/// The rows the tree shows, through the kit.
#[test]
fn the_kit_shows_the_same_rows_v1_would_have() {
    let view = tree().view();
    let paths: Vec<Vec<usize>> = visible_rows(&match view.tree().lock() {
        Ok(f) => f.clone(),
        Err(_) => Vec::new(),
    })
    .iter()
    .map(|r| r.path.clone())
    .collect();

    assert_eq!(
        paths,
        vec![vec![0], vec![0, 0], vec![0, 0, 0]],
        "three rows: both parents are expanded and the leaf has no children"
    );
}

/// A toggle through the kit's shared forest is visible from the tree the kit built —
/// which is the whole of the "shared" claim at the kit boundary.
#[test]
fn a_toggle_through_the_shared_forest_is_visible() {
    let view = tree().view();
    let shared = view.tree();
    assert_eq!(view.rows().len(), 3);

    floom_ui_composites::toggle_path(&shared, &[0, 0]);
    assert_eq!(
        view.rows().len(),
        2,
        "and the tree the kit handed out sees it without being rebuilt"
    );
}

/// The kit's forest and the widget's are the *same* allocation, not two copies.
///
/// Two copies would be a silent bug rather than a wrong answer: a toggle through
/// either handle would work, and the two would drift apart. `Arc::ptr_eq` is the
/// only thing that says "one", which is why this is a pointer comparison and not an
/// equality check on the contents.
#[test]
fn the_kits_forest_and_the_widgets_are_one_allocation() {
    let kit = tree();
    let from_kit = kit.tree();
    let view = kit.view();
    assert!(
        std::sync::Arc::ptr_eq(&from_kit, &view.tree()),
        "the kit's handle and the widget's are the same `Arc`, so a toggle through \
         either is seen by both"
    );
}

// --- the props ------------------------------------------------------------

/// `props()` reports the nodes and the callback, as v1 did.
#[test]
fn the_props_report_the_nodes_and_the_callback() {
    let hits = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let tree = {
        let hits = hits.clone();
        tree().on_node_click(move |_label| {
            hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        })
    };
    assert_eq!(tree.props().nodes.len(), 1, "the nodes are on the props");
    assert!(
        tree.props().on_node_click.is_some(),
        "and so is the callback, in v1's `Box<dyn Fn(String)>` shape"
    );
}

/// `TreeView` is what the kit hands back, and it is a 2.0 widget.
#[test]
fn the_kit_hands_back_a_tree_view() {
    let view = tree().view();
    assert_eq!(view.resolved_style().row_height, tree().resolved_style().row_height);
    assert!(
        view.children().len() >= 1,
        "and it assembles into rows without any further help"
    );
}
