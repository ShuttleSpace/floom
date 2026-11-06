// SPDX-License-Identifier: MIT
//! The readouts a panel shows: tree, breadcrumb, style, layout, performance,
//! console.
//!
//! These are the Chrome panels, as rows. Each one is checked against a real
//! mounted tree, because a readout that formats the wrong field is worse than no
//! readout — it looks like an answer.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use floom_core::error::{ErrorBoundary, Report, Severity};
use floom_devtools::Inspector;
use floom_devtools::panel::{
    DevtoolsState, DevtoolsTab, ReadoutRow, breadcrumb_rows, console_rows, elements_readout,
    layout_rows, performance_rows, style_rows, tree_rows,
};
use floom_hot_reload::LiveTree;
use floom_inspector::{AllocSummary, CacheStats, FrameSummary, PerformanceSnapshot, StageSpan};

const DOC: &str = r#"
root = stack axis=vertical gap=8 {
    heading = text label="hello" background=#112233
    body = stack axis=horizontal gap=4 width=100 {
        ok = button label="OK"
    }
}
"#;

fn mounted() -> LiveTree {
    let document = floom_dsl::compile(DOC).expect("valid document").document;
    let mut tree = LiveTree::new();
    tree.mount(&document);
    tree
}

/// Every row's `label: value`, the way the panel prints it.
///
/// A tree row prints as its key and its kind like any other row: the selection
/// is no longer a marker in the label. The interactive row paints a filled
/// background instead, which a string cannot carry, so a test that wants the
/// selection reads [`ReadoutRow::node`] rather than grepping for a glyph.
fn printed(rows: &[ReadoutRow]) -> Vec<String> {
    rows.iter()
        .map(|r| {
            let indent = "  ".repeat(r.depth);
            if r.heading {
                format!("{indent}[{}]", r.label)
            } else if r.value.is_empty() {
                format!("{indent}{}", r.label)
            } else {
                format!("{indent}{}  {}", r.label, r.value)
            }
        })
        .collect()
}

#[test]
fn the_tree_is_nested_by_depth_in_document_order() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = tree_rows(&inspector, None, &BTreeSet::new());
    let lines = printed(&rows);

    // Root first, then its children in document order, then the deeper level.
    assert_eq!(
        lines,
        vec![
            "root  host",
            "  heading  primitive",
            "  body  host",
            "    ok  primitive",
        ],
        "the tree reads the way a browser's element panel does: one indent step
         per level of depth"
    );
}

#[test]
fn the_selected_node_is_the_only_row_marked_selected() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let plain = tree_rows(&inspector, None, &BTreeSet::new());
    let marked = tree_rows(&inspector, Some("ok"), &BTreeSet::new());

    let selected: Vec<&str> = marked
        .iter()
        .filter_map(|row| row.node.as_ref())
        .filter(|node| node.selected)
        .map(|node| node.key.as_str())
        .collect();
    assert_eq!(selected, vec!["ok"], "exactly one row is selected");

    let plain_selected = plain
        .iter()
        .filter_map(|row| row.node.as_ref())
        .filter(|node| node.selected)
        .count();
    assert_eq!(
        plain_selected, 0,
        "nothing is selected when `selected` is None"
    );
}

#[test]
fn every_tree_row_is_an_interactive_row() {
    // The whole point of the tree tab: each row can be clicked. A row that came
    // back as plain text would be a row the reader cannot select, which is the
    // bug that made Style and Layout look dead.
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = tree_rows(&inspector, None, &BTreeSet::new());
    assert!(!rows.is_empty());
    for row in &rows {
        assert!(
            row.node.is_some(),
            "tree row {:?} is not interactive",
            row.label
        );
    }
}

#[test]
fn a_collapsed_node_hides_its_subtree() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let mut collapsed = BTreeSet::new();
    collapsed.insert("body".to_string());

    let lines = printed(&tree_rows(&inspector, None, &collapsed));
    assert_eq!(
        lines,
        vec!["root  host", "  heading  primitive", "  body  host"]
    );

    // The row itself still says it has children, so the chevron can reopen it.
    let rows = tree_rows(&inspector, None, &collapsed);
    let body = rows
        .iter()
        .filter_map(|row| row.node.as_ref())
        .find(|node| node.key == "body")
        .expect("the body row is still shown");
    assert!(body.has_children);
    assert!(!body.expanded);
}

#[test]
fn the_ancestors_of_the_selection_are_always_expanded() {
    // Collapsing is a display choice; hiding the element the Style tab is about
    // would make the selection invisible from the tab that selected it.
    let live = mounted();
    let inspector = Inspector::new(&live);
    let mut collapsed = BTreeSet::new();
    collapsed.insert("body".to_string());

    let lines = printed(&tree_rows(&inspector, Some("ok"), &collapsed));
    assert!(
        lines.iter().any(|line| line.contains("ok")),
        "the selected node's ancestor was collapsed and hid it: {lines:?}"
    );
}

#[test]
fn the_breadcrumb_walks_from_the_root_to_the_selection() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = breadcrumb_rows(&inspector, "ok");
    let lines = printed(&rows);
    assert_eq!(lines[0], "[Breadcrumb]");
    assert_eq!(lines[1], "path  root > body > ok", "{lines:?}");
}

#[test]
fn a_breadcrumb_for_nothing_selected_says_so() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = breadcrumb_rows(&inspector, "nope");
    let lines = printed(&rows);
    assert_eq!(lines[1], "(nothing selected)");
}

#[test]
fn the_style_readout_shows_every_inspected_state() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = style_rows(&inspector, "heading");
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Styles]");
    // One heading per state the inspector reports, and the heading's background
    // resolves to the colour the document asked for.
    assert!(
        lines.iter().any(|l| l == "  [normal]"),
        "normal is a state: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|l| l.contains("background") && l.contains("#112233")),
        "the document's background is what the readout prints: {lines:?}"
    );
}

#[test]
fn the_style_readout_for_nothing_selected_says_so() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = style_rows(&inspector, "nope");
    assert_eq!(printed(&rows)[1], "(nothing selected)");
}

#[test]
fn the_layout_readout_shows_the_requested_box() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = layout_rows(&inspector, "body");
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Layout]");
    assert!(
        lines.iter().any(|l| l == "size  100 x auto"),
        "the document's width is what the readout prints: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "axis  horizontal"),
        "the document's axis: {lines:?}"
    );
    assert!(lines.iter().any(|l| l == "gap  4"), "the gap: {lines:?}");
}

#[test]
fn a_float_prints_without_trailing_zeros() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = layout_rows(&inspector, "body");
    let lines = printed(&rows);
    // `100` not `100.0`, and `4` not `4.0`.
    assert!(lines.iter().any(|l| l == "size  100 x auto"), "{lines:?}");
    assert!(lines.iter().any(|l| l == "gap  4"), "{lines:?}");
}

#[test]
fn the_elements_panel_composes_tree_breadcrumb_and_the_selection() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    {
        let mut guard = state.lock().unwrap();
        guard.show();
        guard.select("ok");
        guard.set_tab(DevtoolsTab::Layout);
    }
    let rows = elements_readout(&inspector, &guard(&state));
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Elements]");
    assert!(lines.iter().any(|l| l == "path  root > body > ok"));
    // The tab drives which half is composed in — and only that half: a panel
    // that drew the tree underneath the layout section is a panel whose Layout
    // tab is below the fold.
    assert!(lines.iter().any(|l| l == "[Layout]"));
    assert!(
        !lines.iter().any(|l| l.contains("heading")),
        "the tree is not composed under the Layout tab: {lines:?}"
    );
}

fn guard(state: &Arc<Mutex<DevtoolsState>>) -> DevtoolsState {
    state.lock().unwrap().clone()
}

#[test]
fn the_elements_panel_with_no_selection_shows_only_the_tree() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let state = DevtoolsState::new();
    let rows = elements_readout(&inspector, &state);
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Elements]");
    assert!(!lines.iter().any(|l| l == "[Breadcrumb]"));
    assert!(!lines.iter().any(|l| l == "[Styles]"));
    assert!(!lines.iter().any(|l| l == "[Layout]"));
}

#[test]
fn the_performance_panel_lists_newest_frame_first_with_the_verdict() {
    let snapshot = PerformanceSnapshot {
        version: floom_inspector::ProtocolVersion::current(),
        frames: vec![
            FrameSummary {
                index: 1,
                budget_exceeded: false,
                stages: vec![StageSpan {
                    stage: "layout".into(),
                    duration_ns: 4_000_000,
                }],
            },
            FrameSummary {
                index: 2,
                budget_exceeded: true,
                stages: vec![
                    StageSpan {
                        stage: "layout".into(),
                        duration_ns: 4_000_000,
                    },
                    StageSpan {
                        stage: "paint".into(),
                        duration_ns: 12_000_000,
                    },
                ],
            },
        ],
        allocations: AllocSummary {
            allocations: 10,
            deallocations: 4,
            bytes_allocated: 2048,
        },
        caches: vec![CacheStats {
            name: "text".into(),
            entries: 3,
            hits: 9,
            misses: 1,
            bytes: 512,
        }],
    };

    let rows = performance_rows(&snapshot);
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Performance]");
    // Newest first, and the over-budget frame says so.
    assert_eq!(lines[1], "frame 2  16.00 ms  OVER");
    assert_eq!(lines[2], "  layout  4.00 ms");
    assert_eq!(lines[3], "  paint  12.00 ms");
    assert_eq!(lines[4], "frame 1  4.00 ms  ok");
    // The summaries follow.
    assert!(lines.iter().any(|l| l == "live  6"), "10 - 4 live");
    assert!(lines.iter().any(|l| l == "bytes  2048"));
    assert!(
        lines.iter().any(|l| l == "cache text  3 entries, 90% hits"),
        "the hit rate is computed, not stored: {lines:?}"
    );
}

#[test]
fn an_idle_cache_is_not_a_hundred_percent_hit_rate() {
    let snapshot = PerformanceSnapshot {
        version: floom_inspector::ProtocolVersion::current(),
        frames: Vec::new(),
        allocations: AllocSummary::default(),
        caches: vec![CacheStats {
            name: "shape".into(),
            entries: 0,
            hits: 0,
            misses: 0,
            bytes: 0,
        }],
    };
    let rows = performance_rows(&snapshot);
    let lines = printed(&rows);
    assert!(lines.iter().any(|l| l == "(no frames recorded)"));
    assert!(
        lines
            .iter()
            .any(|l| l == "cache shape  0 entries, idle hits"),
        "an idle cache reads idle, not 100%: {lines:?}"
    );
}

#[test]
fn the_console_panel_lists_what_the_boundary_caught() {
    let boundary = ErrorBoundary::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        boundary
            .in_("save draft".to_string())
            .capture(|| -> Result<(), String> { panic!("the disk is full") })
    }))
    .expect("the boundary catches the panic rather than letting it out");
    assert_eq!(outcome, floom_core::error::Outcome::Fallback);

    let reports = boundary.reports();
    assert_eq!(reports.len(), 1);
    let rows = console_rows(&reports);
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Console]");
    assert!(
        lines.iter().any(|l| l.contains("the disk is full")),
        "the panic's message is the row's value: {lines:?}"
    );
    assert!(
        lines.iter().any(|l| l == "    context  save draft"),
        "and what the app was doing, nested under its message: {lines:?}"
    );
}

#[test]
fn an_empty_console_says_so() {
    let rows = console_rows(&[]);
    let lines = printed(&rows);
    assert_eq!(lines[1], "(no errors caught)");
}

#[test]
fn a_report_can_be_built_by_hand_for_the_row_shape() {
    // The row shape is part of the contract: severity, message, context, and
    // where it happened, newest last.
    let report = Report {
        location: "src/main.rs:10".into(),
        context: "boot".into(),
        message: "no adapter".into(),
        backtrace: None,
        severity: Severity::Recoverable,
        sequence: 1,
    };
    let rows = console_rows(&[report]);
    let lines = printed(&rows);
    assert_eq!(lines[0], "[Console]");
    assert_eq!(lines[1], "  Recoverable  no adapter");
    assert_eq!(lines[2], "    context  boot");
    assert_eq!(lines[3], "    at  src/main.rs:10");
}

#[test]
fn the_state_clones_so_a_panel_and_a_caller_agree() {
    // `elements_readout` takes the state by value, which means the shared cell
    // has to be readable into one. That is the whole contract: a clone is the
    // same answer the panel will draw.
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    {
        let mut g = state.lock().unwrap();
        g.show();
        g.select("body");
    }
    let cloned = state.lock().unwrap().clone();
    assert!(cloned.visible());
    assert_eq!(cloned.selected(), Some("body"));
}

#[test]
fn a_live_tree_with_no_style_still_reads_out() {
    // An empty tree: no crash, and the readout says there is nothing there
    // rather than printing a header over zero rows.
    let tree = LiveTree::new();
    let inspector = Inspector::new(&tree);
    assert!(tree_rows(&inspector, None, &BTreeSet::new()).is_empty());
    let state = DevtoolsState::new();
    let rows = elements_readout(&inspector, &state);
    assert_eq!(printed(&rows), vec!["[Elements]"]);
}
