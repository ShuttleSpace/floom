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

use std::sync::{Arc, Mutex};

use crate::abi::{
    ClipSupport, InputModes, NativeCapabilities, NativeCompositionMode, NativeError,
    NativeSnapshot, NativeState, NativeViewKind, NativeViewProvider,
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

/// A handle to a fake provider's command stream.
///
/// Exists because the interesting tests hand the provider to something else —
/// the host owns it — and then need to ask what it was told. A provider that
/// can only be read back through `&self` would be unobservable from exactly the
/// tests that matter most.
#[derive(Debug, Clone, Default)]
pub struct NativeCommandLog(Arc<Mutex<Vec<NativeCommand>>>);

impl NativeCommandLog {
    pub fn commands(&self) -> Vec<NativeCommand> {
        self.0.lock().expect("command log poisoned").clone()
    }

    pub fn count(&self) -> usize {
        self.0.lock().expect("command log poisoned").len()
    }

    pub fn creates(&self) -> usize {
        self.count_where(|c| c.is_create())
    }

    pub fn destroys(&self) -> usize {
        self.count_where(|c| c.is_destroy())
    }

    pub fn focuses(&self) -> usize {
        self.count_where(|c| matches!(c, NativeCommand::Focus { .. }))
    }

    pub fn blurs(&self) -> usize {
        self.count_where(|c| matches!(c, NativeCommand::Blur { .. }))
    }

    fn count_where(&self, predicate: impl Fn(&NativeCommand) -> bool) -> usize {
        self.0
            .lock()
            .expect("command log poisoned")
            .iter()
            .filter(|c| predicate(c))
            .count()
    }

    /// The `apply` commands, in order.
    pub fn applied(&self) -> Vec<(NativeSlot, NativeState)> {
        self.0
            .lock()
            .expect("command log poisoned")
            .iter()
            .filter_map(|c| match c {
                NativeCommand::Apply { slot, state } => Some((*slot, *state)),
                _ => None,
            })
            .collect()
    }

    /// Whether any recorded command satisfies `predicate`.
    pub fn any(&self, predicate: impl Fn(&NativeCommand) -> bool) -> bool {
        self.0
            .lock()
            .expect("command log poisoned")
            .iter()
            .any(predicate)
    }

    pub fn clear(&self) {
        self.0.lock().expect("command log poisoned").clear();
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
    /// A second, shared copy of `commands` for a test that cannot read the
    /// provider back — see [`NativeCommandLog`].
    log: Option<NativeCommandLog>,
    /// When set, `create` fails with this error, so a test can exercise the
    /// path where a platform cannot produce a view at all.
    create_error: Option<NativeError>,
    next_snapshot: u32,
}

impl FakeNativeProvider {
    /// A provider that can do everything: all four modes, snapshots, both input
    /// modes, focusable, and cropping its own content.
    ///
    /// Cropping is declared as [`ClipSupport::Provider`] because that is the
    /// interesting case for a test: the host sends a crop and the provider is
    /// expected to honour it, so a test that forgets to send one fails.
    pub fn new() -> Self {
        Self::with_capabilities(NativeCapabilities {
            modes: std::borrow::Cow::Borrowed(&[
                NativeCompositionMode::Overlay,
                NativeCompositionMode::Texture,
                NativeCompositionMode::Surface,
                NativeCompositionMode::Offscreen,
            ]),
            snapshot: true,
            input: InputModes::ALL,
            focusable: true,
            clip: ClipSupport::Provider,
        })
    }

    pub fn with_capabilities(capabilities: NativeCapabilities) -> Self {
        Self {
            capabilities,
            commands: Vec::new(),
            log: None,
            create_error: None,
            next_snapshot: 1,
        }
    }

    /// A provider that can do nothing at all — the Wayland shape, where there
    /// are no subwindows, or any backend before its bridge is written.
    pub fn incapable() -> Self {
        Self::with_capabilities(NativeCapabilities::NONE)
    }

    /// Change what this provider says about cropping.
    ///
    /// [`ClipSupport::Unsupported`] is the interesting one: a KMS plane, or any
    /// backend whose view cannot be cropped by anyone, which forces the host to
    /// find a mode it composites itself.
    pub fn with_clip(mut self, clip: ClipSupport) -> Self {
        self.capabilities.clip = clip;
        self
    }

    /// A provider that can composite but cannot crop — the shape a plane-based
    /// backend has, and the one that decides whether a host degrades correctly.
    pub fn cannot_crop() -> Self {
        Self::new().with_clip(ClipSupport::Unsupported)
    }

    /// Make the next (and subsequent) `create` calls fail.
    pub fn fail_create_with(&mut self, error: NativeError) {
        self.create_error = Some(error);
    }

    /// Start mirroring every command into a handle the test keeps.
    ///
    /// Call this *before* handing the provider to the host, which takes it by
    /// value: afterwards the only way to ask what happened is the log.
    pub fn watch(&mut self) -> NativeCommandLog {
        let log = NativeCommandLog::default();
        self.log = Some(log.clone());
        log
    }

    fn record(&mut self, command: NativeCommand) {
        if let Some(log) = &self.log {
            log.0
                .lock()
                .expect("command log poisoned")
                .push(command.clone());
        }
        self.commands.push(command);
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

    pub fn focuses(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| matches!(c, NativeCommand::Focus { .. }))
            .count()
    }

    pub fn blurs(&self) -> usize {
        self.commands
            .iter()
            .filter(|c| matches!(c, NativeCommand::Blur { .. }))
            .count()
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
            resource: floom_types::ResourceId::new(n, 1),
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
        // Cloned rather than moved: the capabilities are a `Cow`, so a
        // borrowed list clones as a borrow and an owned one copies its
        // allocation. A fake provider's capabilities never change, so this
        // is a pointer bump in the common case.
        self.capabilities.clone()
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        if let Some(err) = &self.create_error {
            return Err(err.clone());
        }
        self.record(NativeCommand::Create { slot, kind });
        Ok(())
    }

    fn destroy(&mut self, slot: NativeSlot) {
        self.record(NativeCommand::Destroy { slot });
    }

    fn apply(&mut self, slot: NativeSlot, state: NativeState) {
        self.record(NativeCommand::Apply { slot, state });
    }

    fn focus(&mut self, slot: NativeSlot) {
        self.record(NativeCommand::Focus { slot });
    }

    fn blur(&mut self, slot: NativeSlot) {
        self.record(NativeCommand::Blur { slot });
    }

    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        if !self.capabilities.snapshot {
            return None;
        }
        let snap = self.mint_snapshot();
        self.record(NativeCommand::Snapshot { slot });
        Some(snap)
    }

    fn restore(&mut self, slot: NativeSlot) {
        self.record(NativeCommand::Restore { slot });
    }
}
