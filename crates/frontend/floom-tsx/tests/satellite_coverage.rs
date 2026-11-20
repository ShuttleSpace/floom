// SPDX-License-Identifier: MIT
//! The vocabulary against the satellite's component set.
//!
//! The TSX vocabulary is deliberately design-system-neutral: `<Button>` is the
//! button, and which button it draws is the platform's business. So it does
//! not carry the satellite's ninety-eight components one for one — it carries
//! the *concepts* they are, and several satellite components share a concept
//! (a data table and a tree are both a list).
//!
//! This file is that mapping, written down, and the tests are what hold it to
//! the tree:
//!
//! * a component added to the satellite without a row here fails the test,
//!   so the vocabulary cannot quietly fall behind the library;
//! * a row naming a kind the vocabulary does not have fails the test, so the
//!   table cannot claim a coverage that is not there;
//! * the set of components with no equivalent is asserted exactly, so closing
//!   a gap is a deliberate change to this file rather than a silent drift.
//!
//! The gap used to be twenty-six concepts. They are closed now: the
//! design-system-neutral vocabulary either already carried the concept (an
//! avatar is an image, a tooltip is a popup, a toolbar is a container) or gained
//! a first-class kind for it (a calendar, a date/time picker, a colour picker,
//! a sparkline). The list is empty on purpose — a gap that reopens is a change
//! somebody makes deliberately, and the assertions below would fail if it did.

use std::collections::BTreeSet;
use std::path::PathBuf;

use floom_dsl::ui_ir::NodeKind;

/// The satellite's kit crates, as the component names the gallery uses.
fn satellite_components() -> Vec<String> {
    // One crate per design system since the floom-ui merge: the components are
    // the modules of `floom-material/src/widgets/`, not `md-*` crates.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../crates/ecosystem/floom-ui/crates/floom-material/src/widgets");
    let mut names: Vec<String> = std::fs::read_dir(&root)
        .expect("the satellite's material widgets")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let rest = name.strip_suffix(".rs")?.to_string();
            if std::path::Path::new(&root.join(&name)).is_file() {
                Some(rest)
            } else {
                None
            }
        })
        .collect();
    names.sort();
    names
}

/// The components that are not node kinds at all.
///
/// The shell a TSX app is wrapped in, and the service layers a node reads
/// rather than draws. The `<App>` wrapper is the shell; the providers publish
/// a value to their descendants and have nothing to paint.
const NOT_NODES: &[&str] = &[
    "app",
    "date_adapter_provider",
    "defaults_provider",
    "display_provider",
    "icons_provider",
    "locale_provider",
    "main",
    "theme_provider",
    "window",
];

/// The DSL kind that expresses each satellite component.
///
/// `None` means the vocabulary has no equivalent yet.
const COVERAGE: &[(&str, Option<&str>)] = &[
    ("alert", Some("overlay")),
    ("app_bar", Some("container")),
    ("autocomplete", Some("text_field")),
    ("avatar", Some("image")),
    ("badge", Some("container")),
    ("banner", Some("overlay")),
    ("bottom_navigation", Some("navigation_bar")),
    ("breadcrumbs", Some("list")),
    ("button", Some("button")),
    ("button_group", Some("stack")),
    ("calendar", Some("calendar")),
    ("card", Some("container")),
    ("carousel", Some("list")),
    ("checkbox", Some("checkbox")),
    ("checkbox_btn", Some("checkbox")),
    ("chip", Some("container")),
    ("chip_group", Some("stack")),
    ("code", Some("text")),
    ("color_picker", Some("color_picker")),
    ("combobox", Some("select")),
    ("confirm_edit", Some("text_field")),
    ("container", Some("container")),
    ("counter", Some("text")),
    ("data_iterator", Some("list")),
    ("datatable", Some("list")),
    ("date_picker", Some("date_picker")),
    ("dialog", Some("dialog")),
    ("divider", Some("divider")),
    ("empty_state", Some("container")),
    ("expand_transition", Some("if")),
    ("expansion_panels", Some("list")),
    ("fade_transition", Some("if")),
    ("file_input", Some("text_field")),
    ("floating_action_button", Some("button")),
    ("footer", Some("container")),
    ("form", Some("container")),
    ("grid", Some("grid")),
    ("hover", Some("container")),
    ("icon", Some("icon")),
    ("icon_button", Some("button")),
    ("image", Some("image")),
    ("infinite_scroller", Some("list")),
    ("item_group", Some("stack")),
    ("kbd", Some("text")),
    ("list", Some("list")),
    ("list_group", Some("stack")),
    ("list_item", Some("list")),
    ("list_subheader", Some("text")),
    ("menu", Some("popup")),
    ("navigation_drawer", Some("navigation_drawer")),
    ("number_input", Some("text_field")),
    ("otp_input", Some("text_field")),
    ("overlay", Some("overlay")),
    ("pagination", Some("stack")),
    ("parallax", Some("container")),
    ("progress_circular", Some("progress_circular")),
    ("progress_linear", Some("progress_linear")),
    ("pull_to_refresh", Some("list")),
    ("radio", Some("radio")),
    ("radio-group", Some("stack")),
    ("range_slider", Some("slider")),
    ("rating", Some("list")),
    ("responsive", Some("container")),
    ("scale_transition", Some("if")),
    ("select", Some("select")),
    ("sheet", Some("popup")),
    ("skeleton_loader", Some("container")),
    ("slide_group", Some("stack")),
    ("slide_transition", Some("if")),
    ("slider", Some("slider")),
    ("snackbar", Some("overlay")),
    ("spacer", Some("spacer")),
    ("sparkline", Some("sparkline")),
    ("speed_dial", Some("popup")),
    ("stepper", Some("stack")),
    ("switch", Some("switch")),
    ("system_bar", Some("container")),
    ("table", Some("list")),
    ("tabs", Some("tabs")),
    ("text", Some("text")),
    ("text-field", Some("text_field")),
    ("textarea", Some("text_field")),
    ("time_picker", Some("time_picker")),
    ("timeline", Some("list")),
    ("toolbar", Some("container")),
    ("tooltip", Some("popup")),
    ("treeview", Some("list")),
    ("validation_messages", Some("list")),
    ("virtual_scroller", Some("list")),
];

/// The components the vocabulary cannot express yet.
///
/// Asserted exactly, so a gap that closes is a change somebody makes on
/// purpose: the coverage table and this list have to agree. It is empty because
/// every satellite component now maps to a kind — see the note at the top of
/// this file.
const GAPS: &[&str] = &[];

/// Every satellite component is accounted for.
///
/// A component added to the satellite without a row here fails: the table is
/// the vocabulary's statement of what it covers, and it has to be updated
/// when the library grows.
#[test]
fn every_satellite_component_is_accounted_for() {
    let components = satellite_components();
    assert_eq!(components.len(), 98, "the satellite's component set");

    let mut missing = Vec::new();
    for component in &components {
        let covered = COVERAGE.iter().any(|(name, _)| name == component)
            || NOT_NODES.contains(&component.as_str());
        if !covered {
            missing.push(component.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "the satellite has components the coverage table does not mention: {missing:?}"
    );
}

/// Every kind the table names is a kind the vocabulary has.
///
/// A row that names a kind that does not exist would claim a coverage the
/// author cannot actually write.
#[test]
fn every_kind_the_table_names_exists() {
    for (component, kind) in COVERAGE {
        let Some(kind) = kind else { continue };
        assert!(
            NodeKind::parse(kind).is_some(),
            "the coverage table maps {component} to `{kind}`, which is not a kind"
        );
    }
}

/// The gap list is exactly the components with no equivalent.
///
/// Two ways this can be wrong, and both matter: a component that gained an
/// equivalent but is still listed as a gap, and a component that lost one and
/// is not.
#[test]
fn the_gap_list_is_exactly_the_uncovered_components() {
    let uncovered: BTreeSet<&str> = COVERAGE
        .iter()
        .filter_map(|(name, kind)| kind.is_none().then_some(*name))
        .collect();
    let listed: BTreeSet<&str> = GAPS.iter().copied().collect();
    assert_eq!(
        uncovered, listed,
        "the gap list and the coverage table disagree about what is missing"
    );
}

/// The `.d.ts` declares every kind the table relies on.
///
/// The table is about the vocabulary; the declaration is what a TSX author
/// actually sees. A kind the vocabulary has and the declaration does not is a
/// coverage the author cannot use.
#[test]
fn the_declaration_declares_every_kind_the_table_relies_on() {
    let dts = floom_tsx::generate_dts();
    let mut kinds: BTreeSet<&str> = BTreeSet::new();
    for (_, kind) in COVERAGE {
        let Some(kind) = kind else { continue };
        kinds.insert(*kind);
    }
    for kind in kinds {
        let interface = format!(
            "{}Props",
            kind.split('_')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<String>()
        );
        assert!(
            dts.contains(&format!("export interface {interface} ")),
            "the declaration does not declare {interface}, which the coverage table relies on"
        );
    }
}
