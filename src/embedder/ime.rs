//! Text input method editor (IME) protocol types.
//!
//! These are the neutral building blocks a backend uses to tell Floem about
//! composition state and to receive IME enable/configuration requests. Mobile
//! and HarmonyOS backends map their soft keyboard, candidate UI, secure input,
//! and CJK composition behavior into this protocol.

use peniko::kurbo::Rect;

/// The semantic purpose of a text input, used to configure the platform IME.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImePurpose {
    /// Default free-form text.
    Text,
    /// Password / hidden text.
    Password,
    /// Numeric input.
    Number,
    /// Email address input.
    Email,
    /// URL input.
    Url,
    /// Search field.
    Search,
}

/// Hints that adjust IME behavior without changing purpose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImeHint {
    /// No special hint.
    #[default]
    Default,
    /// Disable spellcheck.
    NoSpellcheck,
    /// Prefer lowercase.
    Lowercase,
    /// Prefer uppercase.
    Uppercase,
    /// Prefer title case.
    Titlecase,
}

/// The IME state Floem wants the platform to apply to a window.
#[derive(Clone, Debug, PartialEq)]
pub struct ImeState {
    /// Whether the IME / soft keyboard should be active.
    pub enabled: bool,
    /// Candidate rectangle in window coordinates (for candidate window placement).
    pub candidate_rect: Option<Rect>,
    /// Semantic input purpose.
    pub purpose: ImePurpose,
    /// Behavioral hint.
    pub hint: ImeHint,
    /// Secure (hidden) text entry.
    pub secure: bool,
    /// Single-line vs multi-line mode.
    pub multiline: bool,
    /// Optional surrounding text summary for richer composition.
    pub surrounding_text: Option<String>,
}

/// IME events sent from a backend into Floem.
#[derive(Clone, Debug, PartialEq)]
pub enum ImeEvent {
    /// Composition text updated (user is still typing).
    PreeditChanged {
        /// Current composition text.
        text: String,
        /// Selection byte range within `text`, if any.
        cursor_byte_range: Option<(usize, usize)>,
    },
    /// Final text committed (composition complete).
    CommitText(String),
    /// Composition ended without commit.
    CompositionEnd,
    /// IME enabled/disabled state changed.
    EnabledChanged(bool),
}
