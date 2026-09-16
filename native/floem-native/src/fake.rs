//! A provider that records everything instead of doing anything (task 16.1).
//!
//! [`FakeNativeProvider`] is the test surface for the whole of Phase 16: 16.2's
//! per-frame diff, clipping, z-order, hit-test subtraction and focus yield, and
//! 16.4's snapshot transitions, are all assertions about *which commands a
//! provider received*, which means most of this phase is testable without any
//! platform backend existing.
//!
//! It is deliberately dumb. It does not track which slots are bound — that is
//! the controller's job, and a fake that enforced its own lifecycle would hide
//! exactly the bugs the controller is supposed to catch.

use crate::abi::{
    NativeCapabilities, NativeCompositionMode, NativeError, NativeSnapshot, NativeState,
    NativeViewKind, NativeViewProvider,
};
use crate::pool::NativeSlot;

/// One thing the framework asked a provider to do.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeCommand {
    Create {
        slot: NativeSlot,
        kind: NativeViewKind,
    },
    Destroy {
        slot: NativeSlot,
    },
    Apply {
        slot: NativeSlot,
        state: NativeState,
    },
    Focus {
        slot: NativeSlot,
    },
    Blur {
        slot: NativeSlot,
    },
    Snapshot {
        slot: NativeSlot,
    },
    Restore {
        slot: NativeSlot,
    },
}

impl NativeCommand {
    /// The slot this command targeted.
    pub fn slot(&self) -> NativeSlot {
        match self {
            Self::Create { slot, .. }
            | Self::Destroy { slot }
            | Self::Apply { slot, .. }
            | Self::Focus { slot }
            | Self::Blur { slot }
            | Self::Snapshot { slot }
            | Self::Restore { slot } => *slot,
        }
    }

    pub fn is_create(&self) -> bool {
        matches!(self, Self::Create { .. })
    }

    pub fn is_destroy(&self) -> bool {
        matches!(self, Self::Destroy { .. })
    }
}

/// A provider that records commands and answers capability queries.
///
/// Snapshots are synthesised rather than real: the fake mints a
/// [`NativeSnapshot`] whose `resource` is derived from the slot, so a test can
/// tell two snapshots apart without a renderer.
#[derive(Debug, Clone)]
pub struct FakeNativeProvider {
    capabilities: NativeCapabilities,
    commands: Vec<NativeCommand>,
    /// When set, `create` fails with this error, so a test can exercise the
    /// path where a platform cannot produce a view at all.
    create_error: Option<NativeError>,
    next_snapshot: u32,
}

impl FakeNativeProvider {
    /// A provider that can do everything: all four modes, snapshots, both input
    /// modes, focusable.
    pub fn new() -> Self {
        Self::with_capabilities(NativeCapabilities {
            modes: &[
                NativeCompositionMode::Overlay,
                NativeCompositionMode::Texture,
                NativeCompositionMode::Surface,
                NativeCompositionMode::Offscreen,
            ],
            snapshot: true,
            input: crate::abi::InputModes::ALL,
            focusable: true,
        })
    }

    pub fn with_capabilities(capabilities: NativeCapabilities) -> Self {
        Self {
            capabilities,
            commands: Vec::new(),
            create_error: None,
            next_snapshot: 1,
        }
    }

    /// A provider that can do nothing at all — the Wayland shape, where there
    /// are no subwindows, or any backend before its bridge is written.
    pub fn incapable() -> Self {
        Self::with_capabilities(NativeCapabilities::NONE)
    }

    /// Make the next (and subsequent) `create` calls fail.
    pub fn fail_create_with(&mut self, error: NativeError) {
        self.create_error = Some(error);
    }

    pub fn commands(&self) -> &[NativeCommand] {
        &self.commands
    }

    /// Drain the recorded commands.
    pub fn take_commands(&mut self) -> Vec<NativeCommand> {
        std::mem::take(&mut self.commands)
    }

    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    pub fn creates(&self) -> usize {
        self.commands.iter().filter(|c| c.is_create()).count()
    }

    pub fn destroys(&self) -> usize {
        self.commands.iter().filter(|c| c.is_destroy()).count()
    }

    /// The `apply` commands, as `(slot, state)` pairs.
    pub fn applied(&self) -> Vec<(NativeSlot, NativeState)> {
        self.commands
            .iter()
            .filter_map(|c| match c {
                NativeCommand::Apply { slot, state } => Some((*slot, *state)),
                _ => None,
            })
            .collect()
    }

    /// Whether any command targeted `slot`.
    pub fn touched(&self, slot: NativeSlot) -> bool {
        self.commands.iter().any(|c| c.slot() == slot)
    }

    fn mint_snapshot(&mut self) -> NativeSnapshot {
        let n = self.next_snapshot;
        self.next_snapshot += 1;
        NativeSnapshot {
            resource: floem_types::ResourceId::new(n, 1),
            pixel_width: 64,
            pixel_height: 32,
        }
    }
}

impl Default for FakeNativeProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeViewProvider for FakeNativeProvider {
    fn capabilities(&self) -> NativeCapabilities {
        self.capabilities
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        if let Some(err) = &self.create_error {
            return Err(err.clone());
        }
        self.commands.push(NativeCommand::Create { slot, kind });
        Ok(())
    }

    fn destroy(&mut self, slot: NativeSlot) {
        self.commands.push(NativeCommand::Destroy { slot });
    }

    fn apply(&mut self, slot: NativeSlot, state: NativeState) {
        self.commands.push(NativeCommand::Apply { slot, state });
    }

    fn focus(&mut self, slot: NativeSlot) {
        self.commands.push(NativeCommand::Focus { slot });
    }

    fn blur(&mut self, slot: NativeSlot) {
        self.commands.push(NativeCommand::Blur { slot });
    }

    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        if !self.capabilities.snapshot {
            return None;
        }
        let snap = self.mint_snapshot();
        self.commands.push(NativeCommand::Snapshot { slot });
        Some(snap)
    }

    fn restore(&mut self, slot: NativeSlot) {
        self.commands.push(NativeCommand::Restore { slot });
    }
}
