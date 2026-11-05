// SPDX-License-Identifier: MIT
//! The AI-friendly component schema (Task 25.2).
//!
//! The schema is *emitted*, not hand-written: every widget row, domain value
//! and event comes out of the same `floom-ui-ir` tables the validator enforces.
//! These tests pin that derivation — if a table gains a property and the
//! schema does not show it, an AI consumer would author documents the
//! validator rejects, which is exactly the failure the single-table rule
//! exists to prevent.

use floom_accuracy::agent::component_schema;
use floom_dsl::ui_ir::{NodeKind, prop_spec};

fn widget_named(name: &str) -> floom_accuracy::agent::WidgetInfo {
    component_schema()
        .widgets
        .into_iter()
        .find(|w| w.name == name)
        .unwrap_or_else(|| panic!("{name} is missing from the schema"))
}

#[test]
fn every_kind_is_described() {
    let schema = component_schema();
    for kind in NodeKind::ALL {
        let name = kind.to_string();
        assert!(
            schema.widgets.iter().any(|w| w.name == name),
            "{name} is not in the schema"
        );
    }
    // And nothing invented: the schema is exactly the closed vocabulary.
    assert_eq!(schema.widgets.len(), NodeKind::ALL.len());
}

#[test]
fn properties_carry_the_tables_class_type_and_domain() {
    let button = widget_named("Button");
    let variant = button
        .properties
        .iter()
        .find(|p| p.name == "variant")
        .expect("Button declares variant");
    let spec = prop_spec(NodeKind::Button, "variant").expect("table row");
    assert_eq!(variant.class, "widget");
    assert_eq!(variant.value_type, spec.ty.to_string());
    assert_eq!(
        variant.domain,
        spec.domain
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
    );

    // A style property is served to every kind that can wear one.
    for name in ["Text", "Button", "Stack"] {
        let widget = widget_named(name);
        let background = widget
            .properties
            .iter()
            .find(|p| p.name == "background")
            .unwrap_or_else(|| panic!("{name} has no background"));
        assert_eq!(background.class, "style");
        assert_eq!(background.value_type, "Color");
    }
}

#[test]
fn events_match_the_event_table() {
    let text_field = widget_named("TextField");
    // Exactly what the table declares for the field, in declaration order —
    // and nothing more.
    assert_eq!(text_field.events, vec!["Input", "Submit", "Focus", "Blur"]);

    let text = widget_named("Text");
    assert!(
        text.events.is_empty(),
        "a leaf emits nothing: {:?}",
        text.events
    );
}

#[test]
fn containers_declare_slots_and_leaves_do_not() {
    let stack = widget_named("Stack");
    assert_eq!(stack.slots, vec!["children"]);
    let text = widget_named("Text");
    assert!(text.slots.is_empty(), "a leaf takes no children");
}

#[test]
fn examples_are_derived_from_the_tables() {
    let button = widget_named("Button");
    // The example names the variant with its *first* allowed value, straight
    // from the domain — not a hand-written string that can drift.
    assert!(
        button.example.contains("variant: \"filled\""),
        "Button example does not name the first variant: {:?}",
        button.example
    );
    assert!(
        button.example.starts_with("Button {"),
        "Button example does not start with the widget name: {:?}",
        button.example
    );
}

#[test]
fn token_relationships_come_from_the_resolved_properties() {
    let button = widget_named("Button");
    let tokens: Vec<&str> = button.tokens.iter().map(|t| t.token.as_str()).collect();
    assert!(tokens.contains(&"variant"), "Button is styled by variant");
    assert!(
        tokens.contains(&"control_size"),
        "Button is styled by control size"
    );
    assert!(
        tokens.contains(&"interaction_state"),
        "every styled widget varies over states"
    );

    let text = widget_named("Text");
    let tokens: Vec<&str> = text.tokens.iter().map(|t| t.token.as_str()).collect();
    assert!(!tokens.contains(&"variant"), "Text has no variant");
    assert!(
        tokens.contains(&"interaction_state"),
        "Text still varies over states"
    );

    // The values a token allows are the domains the validator enforces.
    let variant = button
        .tokens
        .iter()
        .find(|t| t.token == "variant")
        .expect("variant token");
    assert_eq!(
        variant.values,
        vec!["filled", "outlined", "text", "elevated", "tonal"]
    );
}
