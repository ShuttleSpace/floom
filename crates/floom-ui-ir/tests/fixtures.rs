//! Round-trip fixtures and the validation contract of the UI IR.
//!
//! The fixtures are the *baseline*: each one is parsed, re-serialized and
//! compared byte for byte. A field rename, a reordered list or a changed enum
//! tag therefore fails here rather than silently producing a document that a
//! consumer reads differently. Regenerate deliberately with
//! `UPDATE_IR_FIXTURES=1` — and then re-run without it, because a baseline that
//! is only ever written is a baseline nobody reads.

use std::sync::atomic::{AtomicUsize, Ordering};

use floem_ui_ir::{
    EventName, IrError, IrVersion, MIGRATIONS, MigrationStep, NativeKind, NodeKind, UiDocument,
    ValueType, migrate_with,
};

const SAMPLE: &str = include_str!("../fixtures/sample.json");
const NATIVE: &str = include_str!("../fixtures/native_participant.json");
const MINIMAL: &str = include_str!("../fixtures/minimal.json");

fn load(json: &str) -> UiDocument {
    UiDocument::load_json(json)
        .unwrap_or_else(|e| panic!("fixture should load: {e}"))
        .document
}

/// Parse + re-serialize `json` and compare against the stored bytes.
fn assert_roundtrip(name: &str, json: &str) {
    let doc = load(json);
    let written = doc.to_json();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name);
    if std::env::var("UPDATE_IR_FIXTURES").is_ok_and(|v| v == "1") {
        std::fs::write(&path, &written).expect("write fixture");
        return;
    }
    assert_eq!(
        written, json,
        "{name} no longer round-trips; if the schema changed on purpose, \
         re-run with UPDATE_IR_FIXTURES=1 and review the diff"
    );
}

// --- positive control ------------------------------------------------------
//
// Every rejection test below is satisfied by a `validate()` that fails
// unconditionally. These two have to pass for those to mean anything.

#[test]
fn the_sample_fixture_is_valid() {
    let doc = load(SAMPLE);
    doc.validate().expect("the sample fixture is valid");
    assert_eq!(doc.nodes.len(), 10, "the sample is the wide-looking one");
    assert_eq!(doc.root.as_str(), "app.root");
}

#[test]
fn the_minimal_fixture_is_valid() {
    let doc = load(MINIMAL);
    doc.validate().expect("the minimal fixture is valid");
    assert_eq!(doc.nodes.len(), 1);
}

// --- round trip ------------------------------------------------------------

#[test]
fn every_stored_fixture_round_trips_byte_for_byte() {
    assert_roundtrip("sample.json", SAMPLE);
    assert_roundtrip("native_participant.json", NATIVE);
    assert_roundtrip("minimal.json", MINIMAL);
}

#[test]
fn reserializing_twice_is_stable() {
    // Guards against anything that depends on map iteration order.
    let once = load(SAMPLE).to_json();
    let twice = UiDocument::load_json(&once)
        .expect("reload")
        .document
        .to_json();
    assert_eq!(once, twice);
}

#[test]
fn props_keep_their_written_order() {
    // Ordered, not sorted: the IR is a source of truth, and "the author put
    // `label` before `size`" is information a formatter may want to preserve.
    let doc = load(SAMPLE);
    let title = doc.node("app.title").expect("app.title exists");
    let names: Vec<&str> = title.props.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["label", "size", "foreground"]);
}

// --- version gate ----------------------------------------------------------

#[test]
fn a_newer_minor_is_refused() {
    // We cannot faithfully round-trip a field we do not know about, and
    // silently dropping it would corrupt the document.
    let json = MINIMAL.replace(r#""minor": 0"#, r#""minor": 1"#);
    let err = UiDocument::from_json(&json).expect_err("a newer minor is refused");
    assert!(matches!(err, IrError::VersionTooNew { .. }), "{err:?}");
}

#[test]
fn a_different_major_is_refused() {
    let json = MINIMAL.replace(r#""major": 1"#, r#""major": 2"#);
    let err = UiDocument::from_json(&json).expect_err("another major is refused");
    assert!(matches!(err, IrError::MajorMismatch { .. }), "{err:?}");
}

#[test]
fn the_current_version_loads_with_nothing_applied() {
    let out = UiDocument::load_json(SAMPLE).expect("current version loads");
    assert_eq!(out.source_version, IrVersion::CURRENT);
    assert!(
        out.applied.is_empty(),
        "no migration ran: {:?}",
        out.applied
    );
}

static STEPS_RUN: AtomicUsize = AtomicUsize::new(0);

fn bump(doc: &mut UiDocument) {
    STEPS_RUN.fetch_add(1, Ordering::SeqCst);
    doc.nodes.shrink_to_fit();
}

#[test]
fn an_older_minor_is_upgraded_through_the_migration_table() {
    let mut doc = load(MINIMAL);
    STEPS_RUN.store(0, Ordering::SeqCst);
    let table = [MigrationStep {
        from_minor: 0,
        apply: bump,
    }];
    let applied = migrate_with(&table, &mut doc, IrVersion::new(1, 0), IrVersion::new(1, 1))
        .expect("a path exists");
    assert_eq!(applied, vec![0]);
    assert_eq!(STEPS_RUN.load(Ordering::SeqCst), 1, "the step ran");
}

#[test]
fn a_missing_migration_step_blocks_the_upgrade() {
    let mut doc = load(MINIMAL);
    let table: [MigrationStep; 0] = [];
    let err = migrate_with(&table, &mut doc, IrVersion::new(1, 0), IrVersion::new(1, 2))
        .expect_err("no path");
    assert_eq!(err, IrError::NoMigrationPath { minor: 0 });
}

#[test]
fn the_shipped_migration_table_is_empty_while_the_current_minor_is_zero() {
    // A step in the shipped table means the IR gained a new minor. When that
    // happens, ship the fixture for the older-minor document the step upgrades
    // and relax this assertion — a step exercised only by its own unit test is
    // not exercised at all.
    assert_eq!(IrVersion::CURRENT.minor, 0, "update this pair together");
    assert!(MIGRATIONS.is_empty(), "{MIGRATIONS:?}");
}

// --- structural validation -------------------------------------------------

fn minimal_with(nodes: &str) -> String {
    format!(r#"{{"version":{{"major":1,"minor":0}},"root":"r","bindings":[],"nodes":[{nodes}]}}"#)
}

fn minimal_with_bindings(bindings: &str, nodes: &str) -> String {
    format!(
        r#"{{"version":{{"major":1,"minor":0}},"root":"r","bindings":[{bindings}],"nodes":[{nodes}]}}"#
    )
}

fn rejection(json: &str) -> IrError {
    match UiDocument::load_json(json) {
        Ok(_) => panic!("expected a rejection, got a valid document"),
        Err(e) => e,
    }
}

fn text_node(key: &str, extra: &str) -> String {
    format!(
        r#"{{"key":"{key}","kind":"Text","props":[{{"name":"label","value":{{"Text":"x"}}}}{extra}],"slots":[],"events":[]}}"#
    )
}

fn stack_with(key: &str, children: &[&str]) -> String {
    let slots = children
        .iter()
        .map(|c| format!("\"{c}\""))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"key":"{key}","kind":"Stack","props":[],"slots":[{{"name":"default","children":[{slots}]}}],"events":[]}}"#
    )
}

#[test]
fn an_unknown_node_kind_is_refused() {
    let json = minimal_with(r#"{"key":"r","kind":"Carousel","props":[],"slots":[],"events":[]}"#);
    let err = UiDocument::from_json(&json).expect_err("Carousel is not in the vocabulary");
    assert!(matches!(err, IrError::Json(_)), "{err:?}");
}

#[test]
fn a_root_that_is_not_a_node_is_refused() {
    let json = MINIMAL.replace(r#""root": "r""#, r#""root": "nope""#);
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::MissingRoot {
            root: "nope".to_string()
        }
    );
}

#[test]
fn a_duplicate_node_key_is_refused() {
    let json = minimal_with(&format!("{},{}", text_node("r", ""), text_node("r", "")));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::DuplicateKey {
            key: "r".to_string()
        }
    );
}

#[test]
fn a_missing_child_is_refused() {
    let json = minimal_with(&format!(
        "{},{}",
        stack_with("r", &["ghost"]),
        text_node("a", "")
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::UnknownChild {
            parent: "r".to_string(),
            child: "ghost".to_string()
        }
    );
}

#[test]
fn a_child_with_two_parents_is_refused() {
    let json = minimal_with(&format!(
        "{},{},{}",
        stack_with("r", &["a"]),
        stack_with("b", &["a"]),
        text_node("a", "")
    ));
    let err = rejection(&json);
    assert!(
        matches!(err, IrError::TwoParents { ref child, .. } if child == "a"),
        "{err:?}"
    );
}

#[test]
fn an_unreachable_node_is_refused() {
    // `orphan` is in the node list but no slot claims it, so a walk from the
    // root would never reach it: the document describes a tree that is not the
    // tree it claims.
    let json = minimal_with(&format!(
        "{},{}",
        text_node("r", ""),
        text_node("orphan", "")
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::Unreachable {
            key: "orphan".to_string()
        }
    );
}

#[test]
fn a_cycle_between_nodes_is_refused() {
    // Two containers each claiming the other: neither is reachable from the
    // root, and following either slot loops forever.
    let json = minimal_with(&format!(
        "{},{},{}",
        text_node("r", ""),
        stack_with("a", &["b"]),
        stack_with("b", &["a"])
    ));
    let err = rejection(&json);
    assert!(
        matches!(err, IrError::Cycle { ref at } if at == "a" || at == "b"),
        "{err:?}"
    );
}

#[test]
fn a_duplicate_slot_name_is_refused() {
    let json = minimal_with(&format!(
        r#"{{"key":"r","kind":"Stack","props":[],"slots":[{{"name":"default","children":[]}},{{"name":"default","children":[]}}],"events":[]}}"#
    ));
    let err = rejection(&json);
    assert!(
        matches!(err, IrError::DuplicateSlot { ref slot, .. } if slot == "default"),
        "{err:?}"
    );
}

// --- property validation ---------------------------------------------------

#[test]
fn an_unknown_property_is_refused() {
    let json = minimal_with(&text_node(
        "r",
        r#",{"name":"sparkle","value":{"Bool":true}}"#,
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::UnknownProp {
            key: "r".to_string(),
            prop: "sparkle".to_string()
        }
    );
}

#[test]
fn a_property_the_kind_does_not_accept_is_refused() {
    // `checked` is a real property, but of Checkbox/Switch/Radio, not Text.
    let json = minimal_with(&text_node(
        "r",
        r#",{"name":"checked","value":{"Bool":true}}"#,
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::PropNotAllowed {
            key: "r".to_string(),
            kind: NodeKind::Text,
            prop: "checked".to_string()
        }
    );
}

#[test]
fn a_property_written_twice_is_refused() {
    let json = minimal_with(&text_node(
        "r",
        r#",{"name":"size","value":{"Number":9.0}}"#,
    ));
    // `label` is already there; add a second `label`.
    let json = json.replace(
        r#"{"name":"size","value":{"Number":9.0}}"#,
        r#"{"name":"label","value":{"Text":"y"}}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::DuplicateProp {
            key: "r".to_string(),
            prop: "label".to_string()
        }
    );
}

#[test]
fn a_value_of_the_wrong_type_is_refused() {
    let json = minimal_with(&text_node(
        "r",
        r#",{"name":"size","value":{"Text":"big"}}"#,
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::WrongValueType {
            key: "r".to_string(),
            prop: "size".to_string(),
            expected: ValueType::Number,
            found: "Text",
        }
    );
}

#[test]
fn an_identifier_outside_its_domain_is_refused() {
    // `axis` is an identifier, but only one of two of them.
    let json = minimal_with(&format!(
        r#"{{"key":"r","kind":"Stack","props":[{{"name":"axis","value":{{"Ident":"diagonal"}}}}],"slots":[],"events":[]}}"#
    ));
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::ValueOutsideDomain {
            key: "r".to_string(),
            prop: "axis".to_string(),
            value: "diagonal".to_string(),
        }
    );
}

#[test]
fn a_native_participant_accepts_its_own_properties() {
    // Positive control for the three rejections above: the per-kind table must
    // accept what it lists, not just reject what it does not.
    let doc = load(NATIVE);
    let web = doc.node("app.web").expect("app.web exists");
    assert_eq!(web.kind, NodeKind::Native(NativeKind::Web));
    assert!(
        web.props.iter().any(|p| p.name == "composition"),
        "the fixture carries a composition"
    );
}

// --- bindings and events ---------------------------------------------------

#[test]
fn an_unresolved_binding_is_refused() {
    let json = minimal_with(
        r#"{"key":"r","kind":"Text","props":[{"name":"label","value":{"Ref":"ghost"}}],"slots":[],"events":[]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::UnresolvedBinding {
            key: "r".to_string(),
            name: "ghost".to_string()
        }
    );
}

#[test]
fn a_binding_of_the_wrong_type_is_refused() {
    // The reference resolves, but a Text slot cannot take a Number binding.
    let json = minimal_with_bindings(
        r#"{"name":"n","kind":{"Value":"Number"}}"#,
        r#"{"key":"r","kind":"Text","props":[{"name":"label","value":{"Ref":"n"}}],"slots":[],"events":[]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::BindingTypeMismatch {
            name: "n".to_string(),
            expected: ValueType::Text,
            found: ValueType::Number,
        }
    );
}

#[test]
fn a_handler_used_as_a_value_is_refused() {
    let json = minimal_with_bindings(
        r#"{"name":"save","kind":"Handler"}"#,
        r#"{"key":"r","kind":"Text","props":[{"name":"label","value":{"Ref":"save"}}],"slots":[],"events":[]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::HandlerUsedAsValue {
            name: "save".to_string()
        }
    );
}

#[test]
fn a_value_used_as_an_event_handler_is_refused() {
    let json = minimal_with_bindings(
        r#"{"name":"n","kind":{"Value":"Number"}}"#,
        r#"{"key":"r","kind":"Button","props":[{"name":"label","value":{"Text":"Save"}}],"slots":[],"events":[{"event":"Click","handler":"n"}]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::ValueUsedAsHandler {
            name: "n".to_string()
        }
    );
}

#[test]
fn an_event_the_kind_does_not_emit_is_refused() {
    let json = minimal_with_bindings(
        r#"{"name":"h","kind":"Handler"}"#,
        r#"{"key":"r","kind":"Text","props":[{"name":"label","value":{"Text":"x"}}],"slots":[],"events":[{"event":"Click","handler":"h"}]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::EventNotAllowed {
            key: "r".to_string(),
            kind: NodeKind::Text,
            event: EventName::Click,
        }
    );
}

#[test]
fn an_undeclared_event_handler_is_refused() {
    let json = minimal_with(
        r#"{"key":"r","kind":"Button","props":[{"name":"label","value":{"Text":"Save"}}],"slots":[],"events":[{"event":"Click","handler":"ghost"}]}"#,
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::UnresolvedBinding {
            key: "r".to_string(),
            name: "ghost".to_string()
        }
    );
}

#[test]
fn a_duplicate_binding_name_is_refused() {
    let json = minimal_with_bindings(
        r#"{"name":"n","kind":{"Value":"Number"}},{"name":"n","kind":"Handler"}"#,
        &text_node("r", ""),
    );
    let err = rejection(&json);
    assert_eq!(
        err,
        IrError::DuplicateBinding {
            name: "n".to_string()
        }
    );
}

#[test]
fn the_events_and_bindings_of_the_sample_survive_a_reload() {
    let doc = load(SAMPLE);
    assert_eq!(doc.bindings.len(), 4, "three values and a handler");
    let save = doc.node("app.save").expect("app.save exists");
    assert_eq!(save.events.len(), 1);
    assert_eq!(save.events[0].event, EventName::Click);
    assert_eq!(save.events[0].handler, "save");
}
