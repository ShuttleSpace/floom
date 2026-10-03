//! Commands Floom emits for a backend to execute, plus command batching.

use super::{EmbedderWindowId, ImeState};
use crate::style::CursorStyle;

/// A command Floom emits for the backend to execute on the platform.
#[derive(Clone, Debug, PartialEq)]
pub enum EmbedderCommand {
    /// Ask the backend to schedule a frame for this window.
    RequestRedraw {
        /// Target window.
        window_id: EmbedderWindowId,
    },
    /// Set the cursor shape for this window.
    SetCursor {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Cursor shape, or `None` to reset.
        cursor: Option<CursorStyle>,
    },
    /// Configure the IME / soft keyboard for this window.
    SetImeState {
        /// Target window.
        window_id: EmbedderWindowId,
        /// Requested IME state.
        state: ImeState,
    },
    /// Set the window title.
    SetWindowTitle {
        /// Target window.
        window_id: EmbedderWindowId,
        /// New title.
        title: String,
    },
    /// Show the context menu for this window.
    ShowContextMenu {
        /// Target window.
        window_id: EmbedderWindowId,
    },
    /// Open a file picker dialog.
    OpenFileDialog {
        /// Target window.
        window_id: EmbedderWindowId,
    },
    /// Open a save-as dialog.
    SaveFileDialog {
        /// Target window.
        window_id: EmbedderWindowId,
    },
    /// Write text to the system clipboard.
    WriteClipboard {
        /// Text to write.
        text: String,
    },
    /// Read text from the system clipboard.
    ReadClipboard,
    /// Open an external URL / app.
    OpenExternal {
        /// URL to open.
        url: String,
    },
    /// Notify the platform accessibility tree of an update.
    AccessibilityUpdate {
        /// Target window.
        window_id: EmbedderWindowId,
    },
}

/// A batch of commands accumulated during a single event-handling pass.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommandBatch {
    commands: Vec<EmbedderCommand>,
}

impl CommandBatch {
    /// Append a command to the batch.
    pub fn push(&mut self, command: EmbedderCommand) {
        self.commands.push(command);
    }

    /// Take all commands, leaving the batch empty.
    pub fn drain(&mut self) -> Vec<EmbedderCommand> {
        std::mem::take(&mut self.commands)
    }

    /// Whether the batch has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// Trait for collecting commands emitted by Floom.
///
/// Implemented by [`VecCommandSink`] and by real backends that forward
/// commands to their event loop.
pub trait CommandSink {
    /// Push a single command.
    fn push_command(&mut self, command: EmbedderCommand);
    /// Take all pending commands.
    fn take_commands(&mut self) -> Vec<EmbedderCommand>;
}

/// A simple in-memory [`CommandSink`] used by tests and the headless backend.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct VecCommandSink {
    commands: Vec<EmbedderCommand>,
}

impl CommandSink for VecCommandSink {
    fn push_command(&mut self, command: EmbedderCommand) {
        self.commands.push(command);
    }

    fn take_commands(&mut self) -> Vec<EmbedderCommand> {
        std::mem::take(&mut self.commands)
    }
}
