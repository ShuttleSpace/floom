//! `floom-forms` × the widget controls.
//!
//! `floom-forms` lives below the widget layer, so it never names a control.
//! This test supplies the missing half at the app layer: three [`FieldBinding`]s
//! that read and write a real `TextField`, `Switch` and `Select`, then drives a
//! whole form through validation.

use floom_forms::{FieldBinding, FormController, rules};
use floom_widgets::{Select, Switch, TextField};

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
fn a_form_binds_text_field_switch_and_select() {
    // The controls an app would own.
    let mut name = TextField::new();
    let mut terms = Switch::new("Accept terms");
    let mut plan = Select::new(["Free", "Pro"]);
    assert!(plan.select(0), "default plan selectable");

    // The form model: three fields with rules, two of them cross-checking the
    // control's own representation ("false" for an off switch, a label for a
    // select).
    let mut form = FormController::new()
        .field("name", "")
        .field("terms", "false")
        .field("plan", "Free");
    form.add_rule("name", rules::required());
    form.add_rule("name", rules::min_length(2));
    form.add_rule("terms", rules::equals("true"));
    form.add_rule("plan", rules::one_of(["Free", "Pro"]));

    // 1) Pull the controls into the form; nothing is filled in yet.
    form.pull("name", &name, &name_binding());
    form.pull("terms", &terms, &terms_binding());
    form.pull("plan", &plan, &plan_binding());
    assert_eq!(form.value("name"), Some(""));
    assert_eq!(form.value("terms"), Some("false"));
    assert_eq!(form.value("plan"), Some("Free"));

    // 2) Submit fails while the required field and the terms toggle are unmet.
    let errors = form.submit().expect_err("empty form is invalid");
    assert!(errors.iter().any(|(field, _)| field == "name"));
    assert!(errors.iter().any(|(field, _)| field == "terms"));
    assert!(!errors.iter().any(|(field, _)| field == "plan"));

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

    // Selecting an unsupported plan is rejected by the rule.
    form.set_value("plan", "Enterprise");
    assert_eq!(form.error("plan"), Some("must be one of: Free, Pro"));
    form.set_value("plan", "Pro");
    assert!(form.is_valid());
}
