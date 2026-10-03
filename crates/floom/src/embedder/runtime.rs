//! Core translation boundary from [`EmbedderEvent`] into the Floem runtime.
//!
//! This is the single place where neutral embedder events are turned into
//! Floem's existing window/event APIs. Backends (and the headless proof of
//! concept) drive Floem through [`EmbedderRuntimeState::handle_event`].

use super::{CommandSink, EmbedderEvent};

/// Stateless translation state for routing embedder events into Floem.
///
/// Kept as a struct so future phases can carry per-runtime caches without
/// changing the call sites.
#[derive(Debug, Default)]
pub struct EmbedderRuntimeState;

impl EmbedderRuntimeState {
    /// Create a new runtime translation state.
    pub fn new() -> Self {
        Self
    }

    /// Route a single embedder event into the given window, collecting any
    /// commands Floem wants the backend to execute.
    pub fn handle_event(
        &mut self,
        window: &mut crate::window::handle::WindowHandle,
        event: EmbedderEvent,
        commands: &mut dyn CommandSink,
    ) {
        window.handle_embedder_event(event, commands);
    }
}
