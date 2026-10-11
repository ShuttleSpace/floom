//! Golden example: form validation (Phase 30.1).
//!
//! Shows the three things an app needs from the forms layer:
//!
//! 1. **Declarative rules** — required / email / min-length attached per field.
//! 2. **Dirty + touched tracking** — "is it dirty" and "has it been touched"
//!    are different questions, and error messages should only show after a
//!    field has been visited (or after a submit attempt).
//! 3. **Control bindings** — the forms crate never names a widget; a
//!    [`FieldBinding`] pairs a getter/setter with the control type the app
//!    actually uses (here `floom::prelude::TextField`).
//!
//! Run with `cargo run -p floom-forms-demo`.

use floom::prelude::TextField;
use floom_forms::{FieldBinding, FormController, Validation, rules};

fn main() {
    let mut form = FormController::new()
        .field("email", "")
        .field("password", "");
    form.add_rule("email", rules::required());
    form.add_rule("email", rules::email());
    form.add_rule("password", rules::min_length(8));

    // (1) An untouched empty form is dirty-free but still invalid.
    assert!(!form.any_dirty(), "a pristine form is not dirty");
    assert!(!form.validate(), "empty required fields cannot be valid");
    println!("pristine : valid={} errors={:?}", form.is_valid(), form.errors());

    // (2) Typing into controls. The binding is the bridge: the app owns the
    // control, the form owns the value.
    let binding = FieldBinding::new(
        |field: &TextField| field.text().to_string(),
        |field: &mut TextField, value: &str| {
            *field = std::mem::replace(field, TextField::new()).value(value);
        },
    );

    let mut email = TextField::new().value("alice@example");
    let mut password = TextField::new().value("short");

    form.set_value("email", binding.read(&email));
    form.set_value("password", binding.read(&password));
    assert!(form.any_dirty(), "edits make the form dirty");

    let valid = form.validate();
    println!(
        "half-typed: valid={} errors={:?}",
        valid,
        form.errors().iter().map(|(f, _)| *f).collect::<Vec<_>>()
    );
    assert!(!valid, "alice@example has no interior dot; too-short password fails min_length(8)");
    assert!(matches!(form.validation("email"), Some(Validation::Invalid(_))));
    println!("email says : {:?}", form.error("email"));
    println!("password says: {:?}", form.error("password"));

    // (3) Fix the values through the same binding, so control and form agree.
    binding.write(&mut email, "alice@example.com");
    binding.write(&mut password, "correct horse battery");
    form.set_value("email", binding.read(&email));
    form.set_value("password", binding.read(&password));

    assert!(form.validate(), "a well-formed email and a long password validate");
    assert!(form.is_valid());
    assert!(form.errors().is_empty());
    println!(
        "complete : valid={} email={:?} password={:?} touched={}",
        form.is_valid(),
        form.value("email"),
        form.value("password").map(|v| "*".repeat(v.len())),
        form.is_touched("email") || form.is_touched("password")
    );

    // (4) Touch-all is the submit gesture: it reveals errors without changing
    // validity.
    let mut second = FormController::new().field("nickname", "x");
    second.add_rule("nickname", rules::min_length(3));
    second.validate();
    println!("\nbefore touch: touched={}", second.is_touched("nickname"));
    second.touch_all();
    println!("after touch : touched={} error={:?}", second.is_touched("nickname"), second.error("nickname"));
    assert!(second.is_touched("nickname"));

    println!("\nforms golden example: OK");
}
