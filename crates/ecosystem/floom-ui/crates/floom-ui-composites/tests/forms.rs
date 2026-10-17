//! `floom-forms` x the composite controls.
//!
//! This file is both halves of `floom-app/tests/forms.rs`, which no longer exists:
//! the form model is generic over the control, so the `Select` binding came up
//! here with its widget, and the `TextField` binding came up with this move.
//! Same test, same rules, one control each side of the layering.

use floom_forms::{FieldBinding, FormController, rules};
use floom_ui_composites::{Select, TextField};
use floom_widgets::Switch;

fn plan_binding() -> FieldBinding<Select> {
    FieldBinding::new(
        |control: &Select| control.selected_label().to_string(),
        |control: &mut Select, value: &str| {
            if let Some(index) = control.options().iter().position(|option| option == value) {
                control.select(index);
            }
        },
    )
}

#[test]
fn a_form_binds_a_select() {
    // The control an app would own.
    let mut plan = Select::new(["Free", "Pro"]);
    assert!(plan.select(0), "default plan selectable");

    // The form model, with the rule cross-checking the control's own
    // representation: a select reads as its label, not as an index.
    let mut form = FormController::new().field("plan", "Free");
    form.add_rule("plan", rules::one_of(["Free", "Pro"]));

    // 1) Pull the control in; nothing has changed yet.
    form.pull("plan", &plan, &plan_binding());
    assert_eq!(form.value("plan"), Some("Free"));
    assert!(form.is_valid());

    // 2) A value the control does not offer is rejected by the rule.
    form.set_value("plan", "Enterprise");
    assert_eq!(form.error("plan"), Some("must be one of: Free, Pro"));

    // 3) Choosing through the control is what makes the form valid again.
    plan.select(1);
    form.pull("plan", &plan, &plan_binding());
    assert_eq!(form.value("plan"), Some("Pro"));
    assert!(form.is_valid());
}

// --- the `TextField` half, which moved up from `floom-app/tests/forms.rs` ----

fn name_binding() -> FieldBinding<TextField> {
    FieldBinding::new(
        |control: &TextField| control.text().to_string(),
        |control: &mut TextField, value: &str| {
            control.set_value(value);
        },
    )
}

fn terms_binding() -> FieldBinding<Switch> {
    FieldBinding::new(
        |control: &Switch| control.is_on().to_string(),
        |control: &mut Switch, value: &str| {
            let want = value == "true";
            if control.is_on() != want {
                control.toggle();
            }
        },
    )
}

#[test]
fn a_form_binds_a_text_field_and_a_switch() {
    // The controls an app would own.
    let mut name = TextField::new();
    let mut terms = Switch::new("Accept terms");

    // The form model: two fields with rules, one of them cross-checking the
    // control's own representation ("false" for an off switch).
    let mut form = FormController::new()
        .field("name", "")
        .field("terms", "false");
    form.add_rule("name", rules::required());
    form.add_rule("name", rules::min_length(2));
    form.add_rule("terms", rules::equals("true"));

    // 1) Pull the controls into the form; nothing is filled in yet.
    form.pull("name", &name, &name_binding());
    form.pull("terms", &terms, &terms_binding());
    assert_eq!(form.value("name"), Some(""));
    assert_eq!(form.value("terms"), Some("false"));

    // 2) Submit fails while the required field and the terms toggle are unmet.
    let errors = form.submit().expect_err("empty form is invalid");
    assert!(errors.iter().any(|(field, _)| field == "name"));
    assert!(errors.iter().any(|(field, _)| field == "terms"));

    // 3) Interact with the real controls, then pull again.
    name.set_value("Ada");
    terms.toggle();
    form.pull("name", &name, &name_binding());
    form.pull("terms", &terms, &terms_binding());
    assert!(
        form.submit().is_ok(),
        "form should now be valid: {:?}",
        form.errors()
    );

    // 4) Push form state back out into a different control.
    let mut mirror = TextField::new();
    form.set_value("name", "Grace");
    form.push("name", &mut mirror, &name_binding());
    assert_eq!(mirror.text(), "Grace");
}
