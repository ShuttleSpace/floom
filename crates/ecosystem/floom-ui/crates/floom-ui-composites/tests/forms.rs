//! `floom-forms` × `Select`.
//!
//! This is the half of `floom-app/tests/forms.rs` that moved with the widget.
//! The form model is generic over the control, so binding it to a `TextField`
//! and a `Switch` stayed below and binding it to a `Select` came up here with
//! the widget — same test, same rules, one control each side of the move.

use floom_forms::{FieldBinding, FormController, rules};
use floom_ui_composites::Select;

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
