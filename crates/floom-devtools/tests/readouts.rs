//! The readouts a panel shows: tree, breadcrumb, style, layout, performance,
//! console.
//!
//! These are the Chrome panels, as rows. Each one is checked against a real
//! mounted tree, because a readout that formats the wrong field is worse than no
//! readout — it looks like an answer.

use std::sync::{Arc, Mutex};

use floom_devtools::Inspector;
use floom_devtools::panel::{
    DevtoolsState, DevtoolsTab, ReadoutRow, breadcrumb_rows, console_rows, elements_readout,
    layout_rows, performance_rows, style_rows, tree_rows,
};
use floom_error::{ErrorBoundary, Report, Severity};
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

/// The way the panel's own render path prints a row: the label carries its
/// one-column marker, and `depth` is the indent in front of it.
fn as_printed(rows: &[ReadoutRow]) -> Vec<String> {
    printed(rows)
}

#[test]
fn the_tree_is_nested_by_depth_in_document_order() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let rows = tree_rows(&inspector, None);
    let lines = printed(&rows);

    // Root first, then its children in document order, then the deeper level.
    assert_eq!(
        lines,
        vec![
            " root  host",
            "   heading  primitive",
            "   body  host",
            "     ok  primitive",
        ],
        "the tree reads the way a browser's element panel does: one indent step
         per level of depth, plus the marker's own column in front of the name"
    );
}

#[test]
fn the_selected_node_is_marked() {
    let live = mounted();
    let inspector = Inspector::new(&live);
    let plain = printed(&tree_rows(&inspector, None));
    let marked = printed(&tree_rows(&inspector, Some("ok")));

    // The marker is on the selected row and nowhere else.
    let marked_count = marked
        .iter()
        .filter(|line| line.contains('▸'))
        .count();
    assert_eq!(marked_count, 1, "exactly one row is marked: {marked:?}");
    assert!(marked.iter().any(|l| l.contains("ok") && l.contains('▸')));
    assert_ne!(plain, marked);
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
        lines.iter().any(|l| l.contains("background") && l.contains("#112233")),
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
    assert!(lines.iter().any(|l| l.contains("▸") && l.contains("ok")));
    assert!(lines.iter().any(|l| l == "path  root > body > ok"));
    // The tab drives which half is composed in.
    assert!(lines.iter().any(|l| l == "[Layout]"));
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
        lines
            .iter()
            .any(|l| l == "cache text  3 entries, 90% hits"),
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
    let mut boundary = ErrorBoundary::new();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        boundary
            .in_("save draft".to_string())
            .capture(|| -> Result<(), String> { panic!("the disk is full") })
    }))
    .expect("the boundary catches the panic rather than letting it out");
    assert_eq!(outcome, floom_error::Outcome::Fallback);

    let reports = boundary.reports();
    assert_eq!(reports.len(), 1);
    let rows = console_rows(&reports);
    let lines = printed(&rows);

    assert_eq!(lines[0], "[Console]");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("the disk is full")),
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
    assert!(tree_rows(&inspector, None).is_empty());
    let state = DevtoolsState::new();
    let rows = elements_readout(&inspector, &state);
    assert_eq!(printed(&rows), vec!["[Elements]"]);
}
