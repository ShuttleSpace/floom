//! Lifecycle enforcement for native participants (task 16.1).
//!
//! [`NativeViewProvider`] is intentionally dumb: a platform backend maps
//! framework calls onto OS calls and should not have to remember anything. All
//! of the invariants that make a native view safe to hold across frames live
//! here instead, once:
//!
//! - every `create` is eventually matched by a `destroy`;
//! - a *recycled* slot — same [`NativeViewId`], higher generation — is re-bound
//!   before anything is applied to it, because the handle the provider is
//!   holding belongs to the previous generation;
//! - a rect never crosses to a provider with a negative extent;
//! - a participant is never frozen twice, and is never restored when it was not
//!   frozen.
//!
//! The host (16.2) talks to this, never to a provider directly, so a new
//! platform backend cannot quietly opt out of any of them.

use std::collections::{HashMap, HashSet};

use crate::abi::{
    NativeCapabilities, NativeCompositionMode, NativeError, NativeSnapshot, NativeState,
    NativeViewKind, NativeViewProvider,
};
use crate::pool::NativeSlot;

/// Owns the lifecycle of native participants behind one provider.
#[derive(Debug)]
pub struct NativeViewController<P: NativeViewProvider> {
    provider: P,
    /// Slots the provider has been told to create and not yet destroyed.
    bound: HashSet<NativeSlot>,
    /// Slots currently frozen into a snapshot texture.
    frozen: HashSet<NativeSlot>,
    /// The mode each slot was bound in.
    modes: HashMap<NativeSlot, NativeCompositionMode>,
    /// The slot currently holding OS focus, if any. At most one: focus and IME
    /// ownership are exclusive (decision D4).
    focused: Option<NativeSlot>,
}

impl<P: NativeViewProvider> NativeViewController<P> {
    pub fn new(provider: P) -> Self {
        Self {
            provider,
            bound: HashSet::new(),
            frozen: HashSet::new(),
            modes: HashMap::new(),
            focused: None,
        }
    }

    pub fn capabilities(&self) -> NativeCapabilities {
        self.provider.capabilities()
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn provider_mut(&mut self) -> &mut P {
        &mut self.provider
    }

    pub fn into_provider(self) -> P {
        self.provider
    }

    /// Bind a participant, negotiating its composition mode.
    ///
    /// `preferred` is the participant's order of preference, strongest first.
    /// The chosen mode is returned and remembered, so the host records what it
    /// got rather than assuming — on Wayland (no subwindows) `Overlay` is not
    /// on offer and the answer is what decides whether a fallback happened.
    pub fn bind(
        &mut self,
        slot: NativeSlot,
        kind: NativeViewKind,
        preferred: &[NativeCompositionMode],
    ) -> Result<NativeCompositionMode, NativeError> {
        if self.bound.contains(&slot) {
            return Err(NativeError::AlreadyBound(slot));
        }
        let mode = self
            .provider
            .capabilities()
            .negotiate(preferred)
            .ok_or(NativeError::NoSupportedMode)?;
        self.provider.create(slot, kind)?;
        self.bound.insert(slot);
        self.modes.insert(slot, mode);
        Ok(mode)
    }

    /// Tear a participant down.
    ///
    /// A frozen participant is restored first: destroying the OS view without
    /// handing back the snapshot strands the texture (16.4).
    pub fn destroy(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        if !self.bound.remove(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        if self.frozen.remove(&slot) {
            self.provider.restore(slot);
        }
        if self.focused == Some(slot) {
            self.focused = None;
        }
        self.modes.remove(&slot);
        self.provider.destroy(slot);
        Ok(())
    }

    /// Push the desired state for one participant.
    ///
    /// Idempotent: applying the same state twice is expected to be free, which
    /// is what lets a missed frame self-heal.
    pub fn apply(&mut self, slot: NativeSlot, state: NativeState) -> Result<(), NativeError> {
        if !self.bound.contains(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        state.validate()?;
        let caps = self.provider.capabilities();
        if !caps.supports(state.mode) {
            return Err(NativeError::UnsupportedMode(state.mode));
        }
        if !caps.input.supports(state.input) {
            return Err(NativeError::Unsupported(match state.input {
                crate::abi::InputMode::Capture => "capture input",
                crate::abi::InputMode::Passthrough => "passthrough input",
            }));
        }
        self.provider.apply(slot, state);
        Ok(())
    }

    /// Hand OS focus to a participant, taking it away from whoever had it.
    pub fn focus(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        if !self.bound.contains(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        if !self.provider.capabilities().focusable {
            return Err(NativeError::Unsupported("focus"));
        }
        if let Some(previous) = self.focused.take() {
            self.provider.blur(previous);
        }
        self.focused = Some(slot);
        self.provider.focus(slot);
        Ok(())
    }

    pub fn blur(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        if !self.bound.contains(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        if self.focused.take() == Some(slot) {
            self.provider.blur(slot);
        }
        Ok(())
    }

    /// Freeze a participant into a texture.
    ///
    /// `Ok(None)` means the provider cannot snapshot; `Err` means the caller
    /// asked for something illegal, most often a second freeze.
    pub fn snapshot(&mut self, slot: NativeSlot) -> Result<Option<NativeSnapshot>, NativeError> {
        if !self.bound.contains(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        if self.frozen.contains(&slot) {
            return Err(NativeError::AlreadyFrozen(slot));
        }
        if !self.provider.capabilities().snapshot {
            return Ok(None);
        }
        let snap = self.provider.snapshot(slot);
        if snap.is_some() {
            self.frozen.insert(slot);
        }
        Ok(snap)
    }

    pub fn restore(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        if !self.bound.contains(&slot) {
            return Err(NativeError::NotBound(slot));
        }
        if !self.frozen.remove(&slot) {
            return Err(NativeError::NotFrozen(slot));
        }
        self.provider.restore(slot);
        Ok(())
    }

    // --- queries the host and tests use ---

    pub fn is_bound(&self, slot: NativeSlot) -> bool {
        self.bound.contains(&slot)
    }

    pub fn is_frozen(&self, slot: NativeSlot) -> bool {
        self.frozen.contains(&slot)
    }

    pub fn mode(&self, slot: NativeSlot) -> Option<NativeCompositionMode> {
        self.modes.get(&slot).copied()
    }

    pub fn focused(&self) -> Option<NativeSlot> {
        self.focused
    }

    pub fn bound_count(&self) -> usize {
        self.bound.len()
    }

    /// Slots created and not yet destroyed.
    ///
    /// Non-zero at teardown is a leak of a native view the framework can no
    /// longer name — the invariant tests assert this is empty.
    pub fn unreleased(&self) -> impl Iterator<Item = NativeSlot> + '_ {
        self.bound.iter().copied()
    }
}

/// Let a boxed provider stand in for the provider itself, so
/// `NativeViewController<Box<dyn NativeViewProvider>>` type-checks.
impl<P: NativeViewProvider + ?Sized> NativeViewProvider for Box<P> {
    fn capabilities(&self) -> NativeCapabilities {
        (**self).capabilities()
    }

    fn create(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        (**self).create(slot, kind)
    }

    fn destroy(&mut self, slot: NativeSlot) {
        (**self).destroy(slot)
    }

    fn apply(&mut self, slot: NativeSlot, state: NativeState) {
        (**self).apply(slot, state)
    }

    fn focus(&mut self, slot: NativeSlot) {
        (**self).focus(slot)
    }

    fn blur(&mut self, slot: NativeSlot) {
        (**self).blur(slot)
    }

    fn snapshot(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        (**self).snapshot(slot)
    }

    fn restore(&mut self, slot: NativeSlot) {
        (**self).restore(slot)
    }
}

impl NativeViewController<Box<dyn NativeViewProvider>> {
    /// Erase the provider type, which is what the host wants: one participant
    /// controller per window, chosen at runtime.
    pub fn boxed<P: NativeViewProvider + 'static>(provider: P) -> Self {
        NativeViewController::new(Box::new(provider) as Box<dyn NativeViewProvider>)
    }
}
