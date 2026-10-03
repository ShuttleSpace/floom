//! Form state and validation for Floom 2.0.
//!
//! `floom-forms` is deliberately widget-free. It sits in `framework/`, *below*
//! the widget layer, so it models a form as plain data — fields, values,
//! touched flags and validation state — and exposes a closure-based
//! [`FieldBinding`] seam that a widgets-layer caller (or `floom-app`) uses to
//! read a control into the controller and push a value back out. Nothing here
//! reaches a renderer, a platform, or even `floom-widgets`.
//!
//! Validation has two halves:
//!
//! * **Synchronous rules** ([`rules`]) run inline on every
//!   [`FormController::set_value`] and on [`FormController::validate`].
//! * **Asynchronous rules** model the real race of a server round-trip: the
//!   controller issues a [`ValidationTicket`] when validation begins and only
//!   applies a result whose ticket is still current
//!   ([`FormController::complete`]), so a slow reply for an *old* value is
//!   dropped instead of overwriting the verdict for the new one.
//!
//! ```text
//! set_value ─► sync rules ─► Valid / Invalid
//!           └► async rules ─► Pending ─► complete(ticket) ─► Valid / Invalid
//! ```

#![forbid(unsafe_code)]

use std::sync::Arc;

/// Outcome of validating one field.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Validation {
    /// Not validated yet.
    #[default]
    Idle,
    /// Passed every rule.
    Valid,
    /// Failed a rule, carrying the first failure message.
    Invalid(String),
    /// An asynchronous rule is in flight.
    Pending,
}

impl Validation {
    pub fn is_invalid(&self) -> bool {
        matches!(self, Validation::Invalid(_))
    }

    pub fn is_pending(&self) -> bool {
        matches!(self, Validation::Pending)
    }

    pub fn is_valid(&self) -> bool {
        matches!(self, Validation::Valid)
    }

    /// The failure message, if any.
    pub fn message(&self) -> Option<&str> {
        match self {
            Validation::Invalid(message) => Some(message),
            _ => None,
        }
    }
}

/// A validation rule: `Ok(())` when the value passes, `Err(message)` otherwise.
///
/// The same shape serves synchronous and asynchronous rules; the difference is
/// how the controller runs them (inline vs. via a ticket).
pub type Rule = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

/// A token for one asynchronous validation attempt.
///
/// Callers hand it back with the result; the controller applies the result only
/// if the ticket still matches the field's current generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationTicket {
    field: String,
    generation: u64,
}

impl ValidationTicket {
    pub fn field(&self) -> &str {
        &self.field
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[derive(Clone)]
struct Field {
    name: String,
    initial: String,
    value: String,
    touched: bool,
    validation: Validation,
    generation: u64,
    rules: Vec<Rule>,
    async_rules: Vec<Rule>,
}

impl Field {
    fn new(name: String, initial: String) -> Self {
        Self {
            name,
            value: initial.clone(),
            initial,
            touched: false,
            validation: Validation::Idle,
            generation: 0,
            rules: Vec::new(),
            async_rules: Vec::new(),
        }
    }

    /// Runs every synchronous rule, returning the first failure or `Valid`.
    fn run_sync(&self) -> Validation {
        for rule in &self.rules {
            if let Err(message) = rule(&self.value) {
                return Validation::Invalid(message);
            }
        }
        Validation::Valid
    }
}

/// Owns the state of one form: its fields, their values and their validation.
#[derive(Clone, Default)]
pub struct FormController {
    fields: Vec<Field>,
    submit_attempted: bool,
}

impl std::fmt::Debug for FormController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormController")
            .field(
                "fields",
                &self
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .collect::<Vec<_>>(),
            )
            .field("submit_attempted", &self.submit_attempted)
            .finish()
    }
}

impl FormController {
    pub fn new() -> Self {
        Self::default()
    }

    // --- construction ------------------------------------------------------

    /// Builder form of [`FormController::register`].
    pub fn field(mut self, name: impl Into<String>, initial: impl Into<String>) -> Self {
        self.register(name, initial);
        self
    }

    /// Registers `name` with `initial`; a duplicate name is ignored.
    pub fn register(&mut self, name: impl Into<String>, initial: impl Into<String>) -> &mut Self {
        let name = name.into();
        if self.index_of(&name).is_none() {
            let initial = initial.into();
            self.fields.push(Field::new(name, initial));
        }
        self
    }

    /// Attaches a synchronous rule to `name`.
    pub fn add_rule(&mut self, name: &str, rule: Rule) -> &mut Self {
        if let Some(field) = self.find_mut(name) {
            field.rules.push(rule);
        }
        self
    }

    /// Attaches an asynchronous rule to `name`.
    pub fn add_async_rule(&mut self, name: &str, rule: Rule) -> &mut Self {
        if let Some(field) = self.find_mut(name) {
            field.async_rules.push(rule);
        }
        self
    }

    // --- values ------------------------------------------------------------

    /// Sets `name`'s value, marking it touched and re-running its sync rules.
    ///
    /// Returns whether the value actually changed.
    pub fn set_value(&mut self, name: &str, value: impl Into<String>) -> bool {
        let value = value.into();
        let Some(field) = self.find_mut(name) else {
            return false;
        };
        field.touched = true;
        if field.value == value {
            return false;
        }
        field.value = value;
        let sync = field.run_sync();
        field.validation = if sync.is_valid() && !field.async_rules.is_empty() {
            // Sync passed and there is more to check: wait for `complete`.
            Validation::Pending
        } else {
            sync
        };
        true
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.find(name).map(|field| field.value.as_str())
    }

    /// Whether `name` differs from its registered initial value.
    pub fn is_dirty(&self, name: &str) -> bool {
        self.find(name)
            .map(|field| field.value != field.initial)
            .unwrap_or(false)
    }

    /// Whether any field differs from its initial value.
    pub fn any_dirty(&self) -> bool {
        self.fields.iter().any(|field| field.value != field.initial)
    }

    pub fn is_touched(&self, name: &str) -> bool {
        self.find(name).map(|field| field.touched).unwrap_or(false)
    }

    pub fn mark_touched(&mut self, name: &str) {
        if let Some(field) = self.find_mut(name) {
            field.touched = true;
        }
    }

    /// Marks every field touched (e.g. when a submit is attempted).
    pub fn touch_all(&mut self) {
        for field in &mut self.fields {
            field.touched = true;
        }
    }

    // --- validation --------------------------------------------------------

    /// Runs every synchronous rule on every field.
    ///
    /// Fields with asynchronous rules that pass synchronously become
    /// [`Validation::Pending`] until [`FormController::complete`] resolves them.
    pub fn validate(&mut self) -> bool {
        for field in &mut self.fields {
            let sync = field.run_sync();
            field.validation = if sync.is_valid() && !field.async_rules.is_empty() {
                Validation::Pending
            } else {
                sync
            };
        }
        self.is_valid()
    }

    /// Whether every field is free of invalidity and pending work.
    pub fn is_valid(&self) -> bool {
        !self
            .fields
            .iter()
            .any(|field| field.validation.is_invalid() || field.validation.is_pending())
    }

    /// The validation state of `name`.
    pub fn validation(&self, name: &str) -> Option<&Validation> {
        self.find(name).map(|field| &field.validation)
    }

    /// The failure message for `name`, if it is invalid.
    pub fn error(&self, name: &str) -> Option<&str> {
        self.find(name).and_then(|field| field.validation.message())
    }

    /// Every `(field, message)` pair currently invalid.
    pub fn errors(&self) -> Vec<(&str, &str)> {
        self.fields
            .iter()
            .filter_map(|field| {
                field
                    .validation
                    .message()
                    .map(|message| (field.name.as_str(), message))
            })
            .collect()
    }

    /// Begins asynchronous validation of `name`, returning the ticket to hand
    /// back to [`FormController::complete`].
    ///
    /// Returns `None` when the field is unknown or has no async rules. Beginning
    /// a new attempt invalidates any earlier one.
    pub fn begin_validation(&mut self, name: &str) -> Option<ValidationTicket> {
        let field = self.find_mut(name)?;
        if field.async_rules.is_empty() {
            return None;
        }
        field.generation = field.generation.wrapping_add(1);
        field.validation = Validation::Pending;
        Some(ValidationTicket {
            field: field.name.clone(),
            generation: field.generation,
        })
    }

    /// Applies an asynchronous result.
    ///
    /// Returns whether it was applied: a result whose ticket is no longer the
    /// field's current generation is *stale* and dropped.
    pub fn complete(&mut self, ticket: ValidationTicket, result: Result<(), String>) -> bool {
        let Some(field) = self.find_mut(&ticket.field) else {
            return false;
        };
        if field.generation != ticket.generation {
            return false;
        }
        field.validation = match result {
            Ok(()) => Validation::Valid,
            Err(message) => Validation::Invalid(message),
        };
        true
    }

    /// Validates and, on success, clears the submit flag; on failure returns the
    /// sync errors. Sets `submit_attempted`, so callers can reveal errors.
    pub fn submit(&mut self) -> Result<(), Vec<(String, String)>> {
        self.submit_attempted = true;
        self.validate();
        let errors: Vec<(String, String)> = self
            .fields
            .iter()
            .filter_map(|field| {
                field
                    .validation
                    .message()
                    .map(|message| (field.name.clone(), message.to_string()))
            })
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    pub fn submit_attempted(&self) -> bool {
        self.submit_attempted
    }

    /// Restores every value to its initial state and invalidates pending work.
    pub fn reset(&mut self) {
        for field in &mut self.fields {
            field.value = field.initial.clone();
            field.touched = false;
            field.validation = Validation::Idle;
            field.generation = field.generation.wrapping_add(1);
        }
        self.submit_attempted = false;
    }

    // --- bindings ----------------------------------------------------------

    /// Reads `control` through `binding` and stores it as `name`'s value.
    pub fn pull<C>(&mut self, name: &str, control: &C, binding: &FieldBinding<C>) -> bool {
        self.set_value(name, binding.read(control))
    }

    /// Pushes `name`'s value into `control` through `binding`.
    pub fn push<C>(&self, name: &str, control: &mut C, binding: &FieldBinding<C>) {
        if let Some(value) = self.value(name) {
            binding.write(control, value);
        }
    }

    // --- introspection -----------------------------------------------------

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.fields.iter().map(|field| field.name.as_str())
    }

    pub fn len(&self) -> usize {
        self.fields.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    fn find(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|field| field.name == name)
    }

    fn find_mut(&mut self, name: &str) -> Option<&mut Field> {
        self.fields.iter_mut().find(|field| field.name == name)
    }

    fn index_of(&self, name: &str) -> Option<usize> {
        self.fields.iter().position(|field| field.name == name)
    }
}

/// Reads a control of type `C` into, and writes back from, a form field.
///
/// Held as two closures so the binding can live in the widgets/app layer where
/// the concrete control type is known — `floom-forms` itself never names a
/// widget.
pub struct FieldBinding<C> {
    get: Box<dyn Fn(&C) -> String + Send + Sync>,
    set: Box<dyn Fn(&mut C, &str) + Send + Sync>,
}

impl<C> FieldBinding<C> {
    pub fn new(
        get: impl Fn(&C) -> String + Send + Sync + 'static,
        set: impl Fn(&mut C, &str) + Send + Sync + 'static,
    ) -> Self {
        Self {
            get: Box::new(get),
            set: Box::new(set),
        }
    }

    /// The control's current value, as the form sees it.
    pub fn read(&self, control: &C) -> String {
        (self.get)(control)
    }

    /// Writes a form value into the control.
    pub fn write(&self, control: &mut C, value: &str) {
        (self.set)(control, value);
    }
}

/// Ready-made synchronous rules.
pub mod rules {
    use super::Rule;
    use std::sync::Arc;

    /// Rejects empty (or whitespace-only) input.
    pub fn required() -> Rule {
        Arc::new(|value: &str| {
            if value.trim().is_empty() {
                Err("this field is required".to_string())
            } else {
                Ok(())
            }
        })
    }

    /// Requires at least `n` characters (Unicode scalar values).
    pub fn min_length(n: usize) -> Rule {
        Arc::new(move |value: &str| {
            if value.chars().count() >= n {
                Ok(())
            } else {
                Err(format!("must be at least {n} characters"))
            }
        })
    }

    /// Allows at most `n` characters (Unicode scalar values).
    pub fn max_length(n: usize) -> Rule {
        Arc::new(move |value: &str| {
            if value.chars().count() <= n {
                Ok(())
            } else {
                Err(format!("must be at most {n} characters"))
            }
        })
    }

    /// Requires a length within `min..=max` characters.
    pub fn length_between(min: usize, max: usize) -> Rule {
        Arc::new(move |value: &str| {
            let len = value.chars().count();
            if len < min {
                Err(format!("must be at least {min} characters"))
            } else if len > max {
                Err(format!("must be at most {max} characters"))
            } else {
                Ok(())
            }
        })
    }

    /// A pragmatic email check: one `@`, a domain with an interior dot.
    pub fn email() -> Rule {
        Arc::new(|value: &str| {
            let ok = value.matches('@').count() == 1
                && value
                    .split_once('@')
                    .map(|(local, domain)| {
                        !local.is_empty()
                            && domain.contains('.')
                            && !domain.starts_with('.')
                            && !domain.ends_with('.')
                    })
                    .unwrap_or(false);
            if ok {
                Ok(())
            } else {
                Err("must be a valid email address".to_string())
            }
        })
    }

    /// Restricts the value to one of `options`.
    pub fn one_of(options: impl IntoIterator<Item = impl Into<String>>) -> Rule {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        Arc::new(move |value: &str| {
            if options.iter().any(|option| option == value) {
                Ok(())
            } else {
                Err(format!("must be one of: {}", options.join(", ")))
            }
        })
    }

    /// Requires the value to equal `expected` (e.g. a confirmation field).
    pub fn equals(expected: impl Into<String>) -> Rule {
        let expected = expected.into();
        Arc::new(move |value: &str| {
            if value == expected {
                Ok(())
            } else {
                Err("values do not match".to_string())
            }
        })
    }

    /// Escape hatch: fail with `message` whenever `predicate` is false.
    pub fn predicate(
        message: impl Into<String>,
        predicate: impl Fn(&str) -> bool + Send + Sync + 'static,
    ) -> Rule {
        let message = message.into();
        Arc::new(move |value: &str| {
            if predicate(value) {
                Ok(())
            } else {
                Err(message.clone())
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named() -> FormController {
        FormController::new().field("name", "").field("email", "")
    }

    #[test]
    fn required_rule_flags_empty_values() {
        let mut form = FormController::new().field("name", "");
        form.add_rule("name", rules::required());
        assert!(!form.validate());
        assert_eq!(form.error("name"), Some("this field is required"));
        form.set_value("name", "Ada");
        assert_eq!(form.validation("name"), Some(&Validation::Valid));
        assert!(form.is_valid());
    }

    #[test]
    fn length_and_email_rules() {
        let mut form = FormController::new().field("email", "").field("name", "");
        form.add_rule("email", rules::email());
        form.add_rule("name", rules::length_between(2, 4));

        form.set_value("email", "ada@example.com");
        form.set_value("name", "Ada");
        assert!(form.is_valid(), "{:?}", form.errors());

        form.set_value("email", "not-an-email");
        assert_eq!(form.error("email"), Some("must be a valid email address"));
        form.set_value("name", "A");
        assert_eq!(form.error("name"), Some("must be at least 2 characters"));
    }

    #[test]
    fn set_value_marks_touched_and_tracks_dirtiness() {
        let mut form = named();
        assert!(!form.any_dirty());
        form.set_value("name", "Ada");
        assert!(form.is_touched("name"));
        assert!(form.is_dirty("name"));
        assert!(form.any_dirty());
        form.set_value("name", "");
        assert!(!form.is_dirty("name"), "back to the initial value");
    }

    #[test]
    fn async_validation_goes_pending_then_resolves() {
        let mut form = FormController::new().field("username", "");
        form.add_async_rule("username", rules::min_length(3));
        form.set_value("username", "ada");
        assert_eq!(form.validation("username"), Some(&Validation::Pending));

        let ticket = form.begin_validation("username").expect("async ticket");
        assert!(!form.is_valid(), "pending blocks validity");
        assert!(form.complete(ticket, Ok(())));
        assert_eq!(form.validation("username"), Some(&Validation::Valid));
    }

    #[test]
    fn stale_async_results_are_dropped() {
        let mut form = FormController::new().field("username", "");
        form.add_async_rule("username", rules::min_length(3));

        form.set_value("username", "ad");
        let old = form.begin_validation("username").expect("ticket 1");
        // The user types on: a second attempt supersedes the first.
        form.set_value("username", "ada");
        let new = form.begin_validation("username").expect("ticket 2");
        assert_ne!(old.generation(), new.generation());

        // The slow reply for the old value must not clobber the new verdict.
        assert!(!form.complete(old, Err("too short".to_string())));
        assert_eq!(form.validation("username"), Some(&Validation::Pending));
        assert!(form.complete(new, Ok(())));
        assert_eq!(form.validation("username"), Some(&Validation::Valid));
    }

    #[test]
    fn submit_reports_errors_until_the_form_is_fixed() {
        let mut form = FormController::new().field("name", "");
        form.add_rule("name", rules::required());

        let errors = form.submit().expect_err("empty form is invalid");
        assert_eq!(
            errors,
            vec![("name".to_string(), "this field is required".to_string())]
        );
        assert!(form.submit_attempted());

        form.set_value("name", "Ada");
        assert!(form.submit().is_ok());
    }

    #[test]
    fn reset_restores_initial_state() {
        let mut form = named();
        form.add_rule("name", rules::required());
        form.set_value("name", "Ada");
        form.submit().ok();
        form.reset();

        assert_eq!(form.value("name"), Some(""));
        assert!(!form.is_touched("name"));
        assert_eq!(form.validation("name"), Some(&Validation::Idle));
        assert!(!form.any_dirty());
        assert!(!form.submit_attempted());
    }

    #[test]
    fn field_binding_reads_and_writes_a_control() {
        // A stand-in "control" the binding knows how to read and write.
        #[derive(Default)]
        struct FakeControl {
            text: String,
        }
        let binding = FieldBinding::new(
            |control: &FakeControl| control.text.clone(),
            |control: &mut FakeControl, value: &str| control.text = value.to_string(),
        );

        let mut form = FormController::new().field("city", "");
        let mut control = FakeControl {
            text: "Lisbon".to_string(),
        };
        assert!(form.pull("city", &control, &binding));
        assert_eq!(form.value("city"), Some("Lisbon"));

        form.set_value("city", "Porto");
        form.push("city", &mut control, &binding);
        assert_eq!(control.text, "Porto");
    }

    #[test]
    fn equals_rule_matches_a_confirmation_field() {
        let mut form = FormController::new()
            .field("password", "")
            .field("confirm", "");
        form.add_rule("confirm", rules::equals("s3cret"));
        form.set_value("confirm", "wrong");
        assert_eq!(form.error("confirm"), Some("values do not match"));
        form.set_value("confirm", "s3cret");
        assert!(form.validation("confirm").unwrap().is_valid());
    }

    #[test]
    fn one_of_rule_restricts_choices() {
        let mut form = FormController::new().field("plan", "");
        form.add_rule("plan", rules::one_of(["free", "pro"]));
        form.set_value("plan", "enterprise");
        assert_eq!(form.error("plan"), Some("must be one of: free, pro"));
        form.set_value("plan", "pro");
        assert!(form.is_valid());
    }
}
