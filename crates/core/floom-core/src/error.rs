// SPDX-License-Identifier: MIT
//! Error boundaries for Floom 2.0.
//!
//! Moved here from the standalone `floom-error` crate during crate
//! consolidation (2026-11-04). Widget-free (framework sits below widgets). A UI
//! callback that panics would otherwise unwind through the event loop and kill
//! the process; an [`ErrorBoundary`] catches recoverable failures instead:
//!
//! * [`ErrorBoundary::capture`] wraps a fallible section — a panic becomes a
//!   recorded [`Report`] and the caller gets the fallback verdict.
//! * [`ErrorBoundary::classify`] + [`Severity`] sort failures into
//!   *recoverable* (record, render the fallback, keep going) and *fatal*
//!   (record, then re-raise — a corrupted app state must not be papered over).
//! * [`Report`] preserves diagnostic context: the callback location, a
//!   caller-supplied context string, the message, and (debug builds) a
//!   backtrace. Release builds skip the backtrace to keep reports small.
//!
//! The boundary never swallows everything blindly: a *fatal* failure
//! re-panics after being recorded, so diagnostics survive a hard crash.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::rc::Rc;

/// How bad a failure is, decided by the app via
/// [`ErrorBoundary::classify`](ErrorBoundary::classify).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Record + fallback + continue: a view failed, the app lives.
    Recoverable,
    /// Record + re-raise: state is corrupt; do not pretend otherwise.
    Fatal,
}

/// One captured failure with its diagnostic context.
#[derive(Clone)]
pub struct Report {
    /// Where the failing section was installed (`#[track_caller]`).
    pub location: String,
    /// What the app was doing ("save draft", "route /settings").
    pub context: String,
    /// The failure message (panic payload or error `Display`).
    pub message: String,
    /// Debug-builds-only backtrace of the capture site.
    pub backtrace: Option<String>,
    /// How the boundary classified it.
    pub severity: Severity,
    /// Monotonic sequence number, so reports can be ordered across retries.
    pub sequence: u64,
}

impl fmt::Debug for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Report")
            .field("location", &self.location)
            .field("context", &self.context)
            .field("message", &self.message)
            .field("severity", &self.severity)
            .field("sequence", &self.sequence)
            .field("backtrace", &self.backtrace.as_deref().map(str::len))
            .finish()
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{:?}] #{} {} (at {}): {}",
            self.severity, self.sequence, self.context, self.location, self.message
        )
    }
}

/// What a captured section should do next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Keep rendering the normal UI.
    Proceed,
    /// Render the boundary's fallback UI instead.
    Fallback,
}

/// An error boundary over one coherent piece of UI (a route, a dialog, a
/// widget subtree).
///
/// Cheap to clone; state is shared between clones of the same boundary.
#[derive(Clone, Default)]
pub struct ErrorBoundary {
    inner: Rc<Inner>,
}

/// How a boundary decides a report's severity.
///
/// Named rather than written out at the field: the trait object appears in the
/// field type, the setter's signature and the default, and three copies of
/// `Box<dyn Fn(&Report) -> Severity>` is where they would start to disagree.
type Classifier = Box<dyn Fn(&Report) -> Severity>;

struct Inner {
    reports: RefCell<Vec<Report>>,
    counter: Cell<u64>,
    classify: RefCell<Classifier>,
    /// Context label for the next capture, set with [`ErrorBoundary::in_`].
    pending_context: RefCell<String>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            reports: RefCell::new(Vec::new()),
            counter: Cell::new(0),
            classify: RefCell::new(Box::new(|_| Severity::Recoverable)),
            pending_context: RefCell::new(String::new()),
        }
    }
}

impl ErrorBoundary {
    /// A boundary that treats every failure as recoverable.
    pub fn new() -> Self {
        Self::default()
    }

    /// Install a classifier deciding recoverable vs fatal per report.
    pub fn classify_with(&self, classify: impl Fn(&Report) -> Severity + 'static) -> &Self {
        *self.inner.classify.borrow_mut() = Box::new(classify);
        self
    }

    /// Label the next capture — a fluent `boundary.in_("save draft").capture(..)`.
    ///
    /// The label applies once; after a capture the context resets to `""`.
    pub fn in_(&self, context: impl Into<String>) -> Self {
        *self.inner.pending_context.borrow_mut() = context.into();
        self.clone()
    }

    /// Run `body`, catching panics and `Result` failures.
    ///
    /// Returns what the caller should do next: `Proceed` when `body` finished
    /// (including returning `Err` the app asked to handle as recoverable-but-
    /// proceed — treat every `Err` as a failure worth a report), `Fallback`
    /// after a recorded recoverable failure.
    ///
    /// A **fatal** classification records the report and then re-panics, so
    /// the failure is never silently absorbed.
    #[track_caller]
    pub fn capture<T>(&self, body: impl FnOnce() -> Result<T, String>) -> Outcome {
        let location = std::panic::Location::caller().to_string();
        let context = std::mem::take(&mut *self.inner.pending_context.borrow_mut());

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));
        match result {
            Ok(Ok(_value)) => Outcome::Proceed,
            Ok(Err(message)) => {
                let severity = self.record(location, context, message.clone());
                match severity {
                    Severity::Recoverable => Outcome::Fallback,
                    // A non-panic failure classified fatal is re-raised as a
                    // panic: the app asked not to continue past it.
                    Severity::Fatal => panic!("fatal error in boundary: {message}"),
                }
            }
            Err(payload) => {
                let message = payload
                    .downcast_ref::<String>()
                    .cloned()
                    .or_else(|| payload.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_else(|| "non-string panic payload".to_string());
                // Re-panic with the payload after recording a fatal report, so
                // `resume_unwind` keeps the original unwind semantics.
                let severity = self.record(location, context, message);
                if severity == Severity::Fatal {
                    std::panic::resume_unwind(payload);
                }
                Outcome::Fallback
            }
        }
    }

    /// Record a report for `message`, run the classifier, return the severity.
    fn record(&self, location: String, context: String, message: String) -> Severity {
        let sequence = self.inner.counter.get() + 1;
        self.inner.counter.set(sequence);

        let backtrace = if cfg!(debug_assertions) {
            Some(std::backtrace::Backtrace::force_capture().to_string())
        } else {
            None
        };

        let mut report = Report {
            location,
            context: if context.is_empty() {
                "<unlabelled>".to_string()
            } else {
                context
            },
            message,
            backtrace,
            severity: Severity::Recoverable,
            sequence,
        };

        let severity = (self.inner.classify.borrow())(&report);
        report.severity = severity;
        self.inner.reports.borrow_mut().push(report);
        severity
    }

    /// Every report captured so far, oldest first.
    pub fn reports(&self) -> Vec<Report> {
        self.inner.reports.borrow().clone()
    }

    /// The most recent report, if any.
    pub fn last_report(&self) -> Option<Report> {
        self.inner.reports.borrow().last().cloned()
    }

    /// Number of reports captured.
    pub fn count(&self) -> usize {
        self.inner.reports.borrow().len()
    }

    /// Drop recorded reports (e.g. after the user acknowledged them).
    pub fn clear(&self) {
        self.inner.reports.borrow_mut().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A classifier the test can re-arm per case.
    fn boundary_with_severity(severity: Rc<Cell<Severity>>) -> ErrorBoundary {
        let boundary = ErrorBoundary::new();
        boundary.classify_with(move |_| severity.get());
        boundary
    }

    #[test]
    fn success_proceeds_and_records_nothing() {
        let boundary = ErrorBoundary::new();
        let outcome = boundary.capture(|| Ok::<_, String>(42));
        assert_eq!(outcome, Outcome::Proceed);
        assert_eq!(boundary.count(), 0);
    }

    #[test]
    fn err_result_becomes_a_report_and_falls_back() {
        let boundary = ErrorBoundary::new();
        let outcome = boundary
            .in_("save draft")
            .capture(|| -> Result<(), String> { Err("disk full".into()) });
        assert_eq!(outcome, Outcome::Fallback);

        let report = boundary.last_report().expect("recorded");
        assert_eq!(report.message, "disk full");
        assert_eq!(report.context, "save draft");
        assert_eq!(report.severity, Severity::Recoverable);
        assert_eq!(report.sequence, 1);
        assert!(report.location.contains("lib.rs"), "{}", report.location);
    }

    #[test]
    fn panics_are_captured_as_recoverable_fallbacks() {
        let boundary = ErrorBoundary::new();
        let outcome = boundary.capture(|| -> Result<(), String> {
            panic!("index out of range");
        });
        assert_eq!(outcome, Outcome::Fallback);
        let report = boundary.last_report().expect("recorded");
        assert_eq!(report.message, "index out of range");
        assert_eq!(report.severity, Severity::Recoverable);
        // Context defaults when never labelled.
        assert_eq!(report.context, "<unlabelled>");
    }

    #[test]
    fn fatal_failures_record_then_re_raise() {
        let severity = Rc::new(Cell::new(Severity::Fatal));
        let boundary = boundary_with_severity(severity);

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = boundary.capture(|| -> Result<(), String> { panic!("state corrupt") });
        }));
        assert!(result.is_err(), "a fatal failure must re-raise");
        assert_eq!(boundary.count(), 1, "but it is recorded first");
        assert_eq!(boundary.last_report().unwrap().severity, Severity::Fatal);
    }

    #[test]
    fn debug_builds_capture_a_backtrace() {
        let boundary = ErrorBoundary::new();
        let _ = boundary.capture(|| -> Result<(), String> { Err("boom".into()) });
        let report = boundary.last_report().unwrap();
        if cfg!(debug_assertions) {
            let backtrace = report.backtrace.as_deref().expect("captured in debug");
            assert!(!backtrace.is_empty());
        } else {
            assert!(report.backtrace.is_none());
        }
    }

    #[test]
    fn sequence_numbers_are_monotonic_across_captures() {
        let boundary = ErrorBoundary::new();
        let _ = boundary
            .in_("first")
            .capture(|| -> Result<(), String> { Err("a".into()) });
        let _ = boundary
            .in_("second")
            .capture(|| -> Result<(), String> { Err("b".into()) });
        let _ = boundary
            .in_("third")
            .capture(|| -> Result<(), String> { Err("c".into()) });

        let reports = boundary.reports();
        assert_eq!(reports.len(), 3);
        assert_eq!(
            reports.iter().map(|r| r.sequence).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        // The context resets after each capture.
        let _ = boundary.capture(|| -> Result<(), String> { Err("d".into()) });
        assert_eq!(boundary.last_report().unwrap().context, "<unlabelled>");
    }

    #[test]
    fn classifier_sees_the_report_before_it_is_stored() {
        let boundary = ErrorBoundary::new();
        boundary.classify_with(|report| {
            if report.message.contains("corrupt") {
                Severity::Fatal
            } else {
                Severity::Recoverable
            }
        });

        let _ = boundary.capture(|| -> Result<(), String> { Err("merely missing".into()) });
        assert_eq!(
            boundary.last_report().unwrap().severity,
            Severity::Recoverable
        );

        // A fatal one still records before re-raising.
        let re_raised = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = boundary.capture(|| -> Result<(), String> { Err("state corrupt".into()) });
        }));
        assert!(re_raised.is_err());
        assert_eq!(boundary.count(), 2);
        assert_eq!(boundary.last_report().unwrap().severity, Severity::Fatal);
    }

    #[test]
    fn clear_resets_history_but_not_the_sequence() {
        let boundary = ErrorBoundary::new();
        let _ = boundary.capture(|| -> Result<(), String> { Err("one".into()) });
        boundary.clear();
        assert_eq!(boundary.count(), 0);

        let _ = boundary.capture(|| -> Result<(), String> { Err("two".into()) });
        assert_eq!(boundary.last_report().unwrap().sequence, 2);
    }

    #[test]
    fn display_is_human_readable() {
        let boundary = ErrorBoundary::new();
        let _ = boundary
            .in_("load session")
            .capture(|| -> Result<(), String> { Err("no such file".into()) });
        let text = boundary.last_report().unwrap().to_string();
        assert!(text.contains("Recoverable"));
        assert!(text.contains("load session"));
        assert!(text.contains("no such file"));
    }
}
