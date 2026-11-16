//! The Floom 2.0 text editor.
//!
//! ## What this is
//!
//! The v1-era editor satellite, ported onto 2.0's `Widget` API. The model —
//! a rope buffer, a cursor, a selection, modes, registers, movement and word
//! motions — lives in [`core`], which is framework-free. This crate is the
//! *view* layer: the widget, the style, and the document.
//!
//! ## How the text is drawn
//!
//! The editor does not draw text itself. It asks the engine to lay a line out
//! ([`floom_text::TextLayout`]) and asks the host to paint the *same
//! description* it laid out ([`floom_core::PaintContext::fill_styled_text`]).
//! The two agree because they are given the same value, which is what keeps
//! the caret on the glyphs rather than near them.
//!
//! ## What is here
//!
//! The editing experience: the text, the caret, the selection, the gutter, the
//! keyboard, the mouse, the scrolling — and the three things that were named
//! as gaps in the first port and are now closed:
//!
//! * **Soft wrap.** A line too long for the text area breaks into several
//!   rows, and the engine decides where. The gutter still numbers the *line*
//!   once, and the caret, the selection and the scroll all agree about which
//!   row is which.
//! * **Multiple cursors.** The selection is one or more regions, and every
//!   edit applies to all of them. Alt and a click adds a caret;
//!   Control-Alt with the up and down arrows adds one above or below.
//! * **Visible whitespace and indent guides.** A space is a dot and a tab is
//!   an arrow; a guide is a line at each tab stop the indentation reaches.
//!   Both are off by default and both are drawn over the text.
//!
//! ## What is not here
//!
//! The v1 editor was eleven thousand lines, and a port that claimed to have
//! reproduced all of it would be a port that had reproduced none of it. What
//! is *not* here is named, in [`editor`], at the place where it would have
//! been:
//!
//! * **Modal editing.** The v1 editor had a modal mode with its own keymap.
//! * **Phantom text and IME preedit.** v1 drew both: the phantom text is the
//!   text an editor shows without holding it, and the preedit is the text the
//!   input method is composing. The preedit needs the host's composition, and
//!   the host does not hand one to a widget yet — so the editor cannot draw
//!   what it is never told.
//! * **The command system.** v1 had a `Command` enum, a command registry, and
//!   a keymap that dispatched into it. This port's keyboard handling is
//!   direct.

pub mod document;
pub mod editor;
pub mod style;
pub mod styling;
pub mod core;
pub mod text_editor;

pub use document::{Document, TextDocument};
pub use editor::{Editor, EditorState};
pub use style::EditorStyle;
pub use styling::{SimpleStyling, Styling};
pub use text_editor::{TextEditor, text_editor};
