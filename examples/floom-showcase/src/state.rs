// SPDX-License-Identifier: MIT
//! Application state that outlives a page rebuild.
//!
//! `SyncBaseSignal` (not the `Rc`-based `RwSignal`) is the one that crosses
//! `Child::new`'s `Send + Sync` builder boundary; the editor is behind a
//! `Mutex` for the same reason.

use std::sync::{Arc, Mutex};

use floom_editor::EditorWidget;
use floom_signal::SyncBaseSignal;

use crate::routes::AppRoute;

/// The editor's initial document, also used to reset the editor.
pub const SAMPLE: &str = "\
Floom 2.0 — editor

This page mounts floom-editor's document model:
  - a rope-backed TextDocument
  - transactions whose inverse records undo
  - selection highlight and a caret drawn by the widget

Type here. Mod+Z undoes, Mod+Y redoes.
Backspace and Delete edit around the caret.
Clicking the editor focuses it; keys go to the focused element.";

/// State that has to outlive a page rebuild.
pub struct AppState {
    pub clicks: SyncBaseSignal<i32>,
    pub bright: SyncBaseSignal<bool>,
    pub signal_count: SyncBaseSignal<i32>,
    pub signal_flag: SyncBaseSignal<bool>,
    pub kit_clicks: SyncBaseSignal<i32>,
    pub kit_checked: SyncBaseSignal<bool>,
    pub route: SyncBaseSignal<u8>,
    /// The editor outlives any single page mount, so its text and selection
    /// survive navigating away and back.
    pub editor: Mutex<Arc<Mutex<EditorWidget>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            clicks: SyncBaseSignal::new(0),
            bright: SyncBaseSignal::new(true),
            signal_count: SyncBaseSignal::new(3),
            signal_flag: SyncBaseSignal::new(true),
            kit_clicks: SyncBaseSignal::new(0),
            kit_checked: SyncBaseSignal::new(true),
            route: SyncBaseSignal::new(AppRoute::Home.id() as u8),
            editor: Mutex::new(Arc::new(Mutex::new(
                EditorWidget::from_text(SAMPLE).autofocus(true),
            ))),
        }
    }

    pub fn editor(&self) -> Arc<Mutex<EditorWidget>> {
        self.editor.lock().unwrap().clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
