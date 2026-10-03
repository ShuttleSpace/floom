//! Headless embedder proof of concept.
//!
//! Drives a Floom view through the same [`EmbedderEvent`]/[`EmbedderCommand`]
//! contract a real backend would use, but without a real platform window or a
//! paint surface. It is the canonical test harness for the embedder API.

use super::{
    CommandSink, EmbedderCommand, EmbedderEvent, EmbedderRuntimeState, EmbedderWindowId, ImeState,
    VecCommandSink,
};

/// A headless backend that exercises the embedder contract.
pub struct HeadlessEmbedder {
    harness: crate::headless::HeadlessHarness,
    runtime: EmbedderRuntimeState,
    commands: VecCommandSink,
}

impl HeadlessEmbedder {
    /// Create a headless embedder driving `view` under `root`.
    pub fn new(root: crate::headless::TestRoot, view: impl crate::IntoView) -> Self {
        Self {
            harness: crate::headless::HeadlessHarness::new(root, view),
            runtime: EmbedderRuntimeState::new(),
            commands: VecCommandSink::default(),
        }
    }

    /// Feed an embedder event into Floom and process updates (no paint).
    pub fn send(&mut self, event: EmbedderEvent) {
        self.runtime
            .handle_event(self.harness.window_handle_mut(), event, &mut self.commands);
        self.harness.process_update_no_paint();
    }

    /// Take all commands accumulated since the last call.
    pub fn take_commands(&mut self) -> Vec<EmbedderCommand> {
        self.commands.take_commands()
    }

    /// The protocol-level id of the headless window.
    pub fn window_id(&self) -> EmbedderWindowId {
        self.harness.embedder_window_id()
    }

    /// Read-only access to the underlying headless harness for introspection
    /// (focus state, layout, hit testing) in tests.
    pub fn harness(&self) -> &crate::headless::HeadlessHarness {
        &self.harness
    }

    /// Mutable access to the underlying headless harness for introspection.
    pub fn harness_mut(&mut self) -> &mut crate::headless::HeadlessHarness {
        &mut self.harness
    }

    // --- Test helpers -----------------------------------------------------

    /// Record a `RequestRedraw` command for the headless window.
    pub fn request_redraw_for_test(&mut self) {
        let window_id = self.window_id();
        self.commands
            .push_command(EmbedderCommand::RequestRedraw { window_id });
    }

    /// Record a `SetImeState` command for the headless window.
    pub fn set_ime_state_for_test(&mut self, state: ImeState) {
        let window_id = self.window_id();
        self.commands
            .push_command(EmbedderCommand::SetImeState { window_id, state });
    }

    /// Simulate a touch down at the given logical position.
    pub fn touch_down_for_test(&mut self, x: f64, y: f64) {
        self.harness.touch_down(x, y);
    }

    /// Simulate a touch up at the given logical position.
    pub fn touch_up_for_test(&mut self, x: f64, y: f64) {
        self.harness.touch_up(x, y);
    }

    /// Whether any pointer currently has implicit/explicit capture.
    pub fn has_pointer_capture_for_test(&self) -> bool {
        self.harness.has_any_pointer_capture()
    }
}
