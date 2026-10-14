//! `floom-forms` × the widget controls.
//!
//! `floom-forms` lives below the widget layer, so it never names a control.
//! This test supplies the missing half at the app layer: three [`FieldBinding`]s
//! that read and write a real `TextField`, `Switch` and `Select`, then drives a
//! whole form through validation.

use floom_forms::{FieldBinding, FormController, rules};
use floom_widgets::{Switch, TextField};

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
