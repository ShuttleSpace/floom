// SPDX-License-Identifier: MIT
//! Platform-neutral IME (input method) state machine for Floom 2.0.
//!
//! Moved here from the standalone `floom-ime` crate during crate
//! consolidation (2026-11-04). `floom-platform::Ime` is the *bridge* to an OS
//! text-input surface; this is the platform-neutral *state* that bridge and
//! widgets agree on. Editors own an [`ImeState`], feed platform events through
//! [`ImeState::apply`], and render preedit / candidate UI from the resulting
//! state.
//!
//! All offsets are byte offsets and are clamped to UTF-8 character boundaries,
//! so a platform that reports a mid-codepoint caret can never corrupt state.

/// Whether an input-method composition is currently in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompositionState {
    #[default]
    Inactive,
    Composing,
}

/// Everything an editor needs to render an IME interaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImeState {
    /// The input-method panel/window is open.
    pub open: bool,
    pub composition: CompositionState,
    /// Uncommitted composition text ("preedit").
    pub preedit: String,
    /// Caret offset within `preedit` (bytes, always a char boundary).
    pub preedit_caret: usize,
    /// Selection within `preedit` as `(anchor, caret)` byte offsets.
    pub preedit_selection: Option<(usize, usize)>,
    /// Candidate strings offered for the current composition.
    pub candidates: Vec<String>,
    /// Index into `candidates`.
    pub selected_candidate: Option<usize>,
    /// Caret offset within the committed buffer (bytes).
    pub caret: usize,
    /// Selection within the committed buffer as `(anchor, caret)` byte offsets.
    pub selection: Option<(usize, usize)>,
    /// Whether key events should also be delivered as direct text input.
    pub text_allowed: bool,
}

impl Default for ImeState {
    fn default() -> Self {
        Self {
            open: false,
            composition: CompositionState::Inactive,
            preedit: String::new(),
            preedit_caret: 0,
            preedit_selection: None,
            candidates: Vec::new(),
            selected_candidate: None,
            caret: 0,
            selection: None,
            // Without an IME swallowing keys, text input is allowed by default.
            text_allowed: true,
        }
    }
}

/// An event arriving from the platform text-input bridge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImeEvent {
    /// The IME panel opened or closed.
    OpenChanged(bool),
    /// Uncommitted composition text changed.
    Preedit {
        text: String,
        caret: usize,
        selection: Option<(usize, usize)>,
    },
    /// Candidate list changed.
    Candidates {
        list: Vec<String>,
        selected: Option<usize>,
    },
    /// Text was committed (finalized) by the IME.
    Commit { text: String },
    /// The editor's own caret/selection changed; used to position the IME panel.
    SelectionChanged {
        caret: usize,
        selection: Option<(usize, usize)>,
    },
    /// Whether key events should be delivered as direct text input.
    TextAllowed(bool),
}

/// What changed as a result of applying an [`ImeEvent`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImeApplyResult {
    /// Text finalized by this event, if any.
    pub committed: Option<String>,
    pub preedit_changed: bool,
    pub candidates_changed: bool,
    pub selection_changed: bool,
    /// `Some(new_state)` when the open state actually flipped.
    pub open_changed: Option<bool>,
}

impl ImeApplyResult {
    /// True when nothing observable changed.
    pub fn is_noop(&self) -> bool {
        self.committed.is_none()
            && !self.preedit_changed
            && !self.candidates_changed
            && !self.selection_changed
            && self.open_changed.is_none()
    }
}

/// Clamp a byte offset into `text`, backing up to a UTF-8 char boundary.
fn clamp_offset(mut offset: usize, text: &str) -> usize {
    offset = offset.min(text.len());
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn clamp_selection(selection: Option<(usize, usize)>, text: &str) -> Option<(usize, usize)> {
    selection.map(|(anchor, caret)| (clamp_offset(anchor, text), clamp_offset(caret, text)))
}

impl ImeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_composing(&self) -> bool {
        matches!(self.composition, CompositionState::Composing)
    }

    /// Apply one platform event and report what changed.
    pub fn apply(&mut self, event: ImeEvent) -> ImeApplyResult {
        let mut result = ImeApplyResult::default();

        match event {
            ImeEvent::OpenChanged(open) => {
                if self.open != open {
                    self.open = open;
                    result.open_changed = Some(open);
                }
            }
            ImeEvent::Preedit {
                text,
                caret,
                selection,
            } => {
                let had_preedit = !self.preedit.is_empty();
                self.preedit = text;
                self.preedit_caret = clamp_offset(caret, &self.preedit);
                self.preedit_selection = clamp_selection(selection, &self.preedit);
                self.composition = if self.preedit.is_empty() {
                    CompositionState::Inactive
                } else {
                    CompositionState::Composing
                };
                result.preedit_changed = true;
                // Silence the flag when an already-empty preedit stayed empty.
                if !had_preedit && self.preedit.is_empty() {
                    result.preedit_changed = false;
                }
            }
            ImeEvent::Candidates { list, selected } => {
                let selected = selected.filter(|index| *index < list.len());
                self.candidates = list;
                self.selected_candidate = selected;
                result.candidates_changed = true;
            }
            ImeEvent::Commit { text } => {
                if !self.preedit.is_empty() {
                    self.preedit.clear();
                    self.preedit_caret = 0;
                    self.preedit_selection = None;
                    result.preedit_changed = true;
                }
                if !self.candidates.is_empty() {
                    self.candidates.clear();
                    self.selected_candidate = None;
                    result.candidates_changed = true;
                }
                self.composition = CompositionState::Inactive;
                result.committed = Some(text);
            }
            ImeEvent::SelectionChanged { caret, selection } => {
                self.caret = caret;
                self.selection = selection;
                result.selection_changed = true;
            }
            ImeEvent::TextAllowed(allowed) => {
                self.text_allowed = allowed;
            }
        }

        result
    }

    /// Begin a composition explicitly (before any preedit text arrives).
    pub fn begin_composition(&mut self) {
        self.composition = CompositionState::Composing;
        self.open = true;
    }

    /// Move the candidate highlight; returns `false` when out of range.
    pub fn select_candidate(&mut self, index: usize) -> bool {
        if index < self.candidates.len() {
            self.selected_candidate = Some(index);
            true
        } else {
            false
        }
    }

    /// The currently highlighted candidate, if any.
    pub fn selected_candidate_text(&self) -> Option<&str> {
        self.selected_candidate
            .and_then(|index| self.candidates.get(index))
            .map(String::as_str)
    }

    /// Discard the in-flight composition without committing anything.
    pub fn cancel(&mut self) {
        self.preedit.clear();
        self.preedit_caret = 0;
        self.preedit_selection = None;
        self.candidates.clear();
        self.selected_candidate = None;
        self.composition = CompositionState::Inactive;
    }
}
