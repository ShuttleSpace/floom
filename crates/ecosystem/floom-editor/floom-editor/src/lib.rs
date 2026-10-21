//! The Floom 2.0 text editor.
//!
//! ## What this is
//!
//! The v1-era editor satellite, ported onto 2.0's `Widget` API. The model —
//! a rope buffer, a cursor, a selection, modes, registers, movement and word
//! motions — is `floom-editor-core`, which is framework-free and needed no
//! change at all. This crate is the *view* layer: the widget, the style, and
//! the document.
//!
//! ## How the text is drawn
//!
//! The editor does not draw text itself. It asks the engine to lay a line out
//! ([`floom_text::TextLayout`]) and asks the host to paint the *same
//! description* it laid out ([`floom_core::PaintContext::fill_styled_text`]).
//! The two agree because they are given the same value, which is what keeps
//! the caret on the glyphs rather than near them.
//!
//! ## What is not here
//!
//! The v1 editor was eleven thousand lines, and a port that claimed to have
//! reproduced all of it would be a port that had reproduced none of it. What
//! is here is the editing experience — the text, the caret, the selection, the
//! gutter, the keyboard, the mouse, the scrolling — and what is *not* here is
//! named, in [`editor`], at the place where it would have been:
//!
//! * **Soft wrap.** The engine wraps; the editor does not use it yet. A long
//!   line runs past the right edge.
//! * **Multiple cursors.** One selection, one caret.
//! * **Modal editing.** The v1 editor had a modal mode with its own keymap.
//! * **Phantom text, IME preedit, indent guides, visible whitespace.** The v1
//!   editor drew all four.
//! * **Per-line styling.** v1's `Styling` answered per line; this one answers
//!   per document, because the line height has to be the same on every line
//!   for the scroll position to mean anything.
//! * **The command system.** v1 had a `Command` enum, a command registry, and
//!   a keymap that dispatched into it. This port's keyboard handling is
//!   direct.

pub mod document;
pub mod editor;
pub mod style;
pub mod styling;
pub mod text_editor;

pub use document::{Document, TextDocument};
pub use editor::{Editor, EditorState};
pub use style::EditorStyle;
pub use styling::{SimpleStyling, Styling};
pub use text_editor::{text_editor, TextEditor};
