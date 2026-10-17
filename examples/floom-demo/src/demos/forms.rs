//! Forms: declarative rules, dirty/touched tracking, and the control binding.
//!
//! The forms crate never names a widget: a [`FieldBinding`] pairs a
//! getter/setter with whatever control the app uses. These demos mount real
//! `TextInput`s and render the controller's own verdict beside them, so what
//! the gallery shows is the same `FormController` an app would drive.

use floom_core::{Axis, Color, LayoutRequest, WidgetAny};
use floom_forms::{FieldBinding, FormController, rules};
use floom_widgets::{Stack, Text, TextInput};

use super::{Demo, Section};

const ERROR: Color = Color::rgb(200, 60, 60);
const OK: Color = Color::rgb(50, 140, 90);

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "form-validation",
            title: "Validation",
            section: Section::Forms,
            build: build_validation,
        },
        Demo {
            id: "form-touched",
            title: "Dirty & touched",
            section: Section::Forms,
            build: build_touched,
        },
    ]
}

/// A message and the colour its verdict reads in: an unmet rule is an error,
/// not a note.
fn status(message: String, valid: bool) -> (String, Color) {
    if valid { (message, OK) } else { (message, ERROR) }
}

fn build_validation() -> Box<dyn WidgetAny> {
    // The binding is the bridge: the app owns the control, the form owns the
    // value, and neither has to know the other's type.
    let binding = FieldBinding::new(
        |field: &TextInput| field.text(),
        |field: &mut TextInput, value: &str| {
            field.set_text(value);
        },
    );

    let mut form = FormController::new()
        .field("email", "")
        .field("password", "");
    form.add_rule("email", rules::required());
    form.add_rule("email", rules::email());
    form.add_rule("password", rules::min_length(8));

    // The controls exist only long enough to be read: a `TextInput` owns a
    // `Mutex`, so it cannot be captured by a `Child` builder. What the builders
    // capture is the *value* a form field ended up holding, which is also the
    // direction the data really flows.
    let email_value = binding.read(&TextInput::with_text("alice@example"));
    let password_value = binding.read(&TextInput::with_text("short"));
    form.set_value("email", email_value.clone());
    form.set_value("password", password_value.clone());
    form.touch_all();
    form.validate();

    let email_note = status(
        form.error("email").unwrap_or("ok").to_string(),
        form.error("email").is_none(),
    );
    let password_note = status(
        form.error("password").unwrap_or("ok").to_string(),
        form.error("password").is_none(),
    );
    let summary = if form.is_valid() {
        "form is valid".to_string()
    } else {
        format!("{} field(s) invalid", form.errors().len())
    };

    Box::new(
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .push_layout(LayoutRequest::new().height(18.0), || {
                Box::new(Text::new("Email").size(12.0))
            })
            .push_layout(LayoutRequest::new().width(240.0).height(34.0), move || {
                Box::new(TextInput::with_text(email_value.clone()))
            })
            .push_layout(LayoutRequest::new().height(16.0), move || {
                Box::new(
                    Text::new(email_note.0.clone())
                        .size(11.0)
                        .color(email_note.1),
                )
            })
            .push_layout(LayoutRequest::new().height(18.0), || {
                Box::new(Text::new("Password").size(12.0))
            })
            .push_layout(LayoutRequest::new().width(240.0).height(34.0), move || {
                Box::new(TextInput::with_text(password_value.clone()))
            })
            .push_layout(LayoutRequest::new().height(16.0), move || {
                Box::new(
                    Text::new(password_note.0.clone())
                        .size(11.0)
                        .color(password_note.1),
                )
            })
            .push_layout(LayoutRequest::new().height(18.0), move || {
                Box::new(Text::new(summary.clone()).size(12.0))
            }),
    )
}

/// Dirty and touched are different questions, and the demo states both: a
/// changed value (dirty) is not the same as a visited field (touched).
fn build_touched() -> Box<dyn WidgetAny> {
    let mut form = FormController::new().field("nickname", "x");
    form.add_rule("nickname", rules::min_length(3));

    let pristine_dirty = form.any_dirty();
    let before_touch = form.is_touched("nickname");
    form.validate();
    form.touch_all();
    let after_touch = form.is_touched("nickname");
    let error = form.error("nickname").unwrap_or("ok").to_string();
    let rule_ok = form.error("nickname").is_none();

    Box::new(
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .push_layout(LayoutRequest::new().height(18.0), move || {
                Box::new(
                    Text::new(format!("pristine dirty = {pristine_dirty}"))
                        .size(12.0)
                        .color(OK),
                )
            })
            .push_layout(LayoutRequest::new().height(18.0), move || {
                Box::new(
                    Text::new(format!("touched before submit = {before_touch}"))
                        .size(12.0)
                        .color(OK),
                )
            })
            .push_layout(LayoutRequest::new().height(18.0), move || {
                Box::new(
                    Text::new(format!("touched after touch_all = {after_touch}"))
                        .size(12.0)
                        .color(OK),
                )
            })
            .push_layout(LayoutRequest::new().height(18.0), move || {
                let color = if rule_ok { OK } else { ERROR };
                Box::new(Text::new(format!("nickname: {error}")).size(12.0).color(color))
            }),
    )
}
