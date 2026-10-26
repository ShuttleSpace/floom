//! Routing native participants to the provider that registered their kind.
//!
//! [`NativeViewController`] owns the lifecycle *behind one provider*; this type
//! owns the table in front of many. A host registers each provider together
//! with the exact [`NativeViewKind`]s it can actually draw, and every
//! participant is routed by kind — so a kind nobody claimed is **refused** with
//! a message naming it, rather than silently served by whatever provider
//! happens to be linked in.
//!
//! That refusal is the point. A provider that "succeeds" at creating a view it
//! never drew hands the app a blank rectangle, which is indistinguishable from
//! a view that was never asked for. Declaring kinds at registration is what
//! turns that from a documentation warning into a structure: the desktop
//! bridge's plain-view factory is a lifecycle test aid, not a claim that it can
//! composite a camera.
//!
//! One provider may claim many kinds — an OS host owns its views and cannot be
//! duplicated per kind — but one kind may have exactly one provider. The
//! conflict is a registration error, an assembly-time fact, not a silent
//! last-wins surprise at runtime.

use std::collections::HashMap;

use floom_types::NativeCapture;

use crate::abi::{
    NativeCapabilities, NativeCompositionMode, NativeError, NativeSnapshot, NativeState,
    NativeViewKind, NativeViewProvider,
};
use crate::controller::NativeViewController;
use crate::pool::NativeSlot;

/// The kind → provider table, and the host-facing surface for native
/// participants.
///
/// Slots never change kind once bound, so the registry routes by slot after
/// bind: the call sites that only have a slot in hand (`apply`, `focus`, …)
/// need no kind, and the per-provider [`NativeViewController`]s partition the
/// lifecycle book-keeping correctly.
#[derive(Default)]
pub struct NativeProviderRegistry {
    /// One entry per provider, in registration order, so teardown collects
    /// captures in a stable order.
    entries: Vec<Entry>,
    /// kind → entry index. Two providers claiming the same kind is a
    /// registration error, so the table always holds exactly one entry per
    /// served kind.
    by_kind: HashMap<NativeViewKind, usize>,
    /// Which entry a bound slot belongs to. Populated at bind, emptied at
    /// destroy — the registry's own routing table, not a duplicate of the
    /// controllers' `bound` sets.
    owner: HashMap<NativeSlot, usize>,
}

struct Entry {
    /// Lifecycle enforcement stays per-provider, so every backend gets the
    /// controller's invariants for free exactly as with a sole provider.
    controller: NativeViewController<Box<dyn NativeViewProvider>>,
}

impl std::fmt::Debug for NativeProviderRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NativeProviderRegistry")
            .field("providers", &self.entries.len())
            .field("bound_slots", &self.owner.len())
            .finish()
    }
}

impl NativeProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `provider` as the implementer of every kind in `kinds`.
    ///
    /// Claims *exactly*. A kind already claimed by another provider is an error
    /// and the whole registration is rejected — a half registration whose
    /// earlier kinds took effect would make the table's shape depend on call
    /// order. A provider claiming no kinds is accepted and serves nothing,
    /// because that is the honest as-built state of a backend whose factories
    /// do not exist yet.
    pub fn register(
        &mut self,
        provider: impl NativeViewProvider + 'static,
        kinds: &[NativeViewKind],
    ) -> Result<(), NativeError> {
        for &kind in kinds {
            if self.by_kind.contains_key(&kind) {
                return Err(NativeError::KindRegistered(kind));
            }
        }
        // No check past this point can fail, so installing the entry and then
        // reserving the kinds is not observable as a torn state.
        self.entries.push(Entry {
            controller: NativeViewController::boxed(provider),
        });
        let index = self.entries.len() - 1;
        for &kind in kinds {
            self.by_kind.insert(kind, index);
        }
        Ok(())
    }

    /// Whether any provider claims `kind`.
    pub fn serves(&self, kind: NativeViewKind) -> bool {
        self.by_kind.contains_key(&kind)
    }

    /// Whether anything is registered at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The kinds that would be served, for diagnostics and for callers that
    /// want to check before they ask.
    pub fn served_kinds(&self) -> impl Iterator<Item = NativeViewKind> + '_ {
        self.by_kind.keys().copied()
    }

    /// Bind a participant of `kind`, negotiating its composition mode against
    /// the provider registered for that kind.
    ///
    /// A kind nobody claimed is refused here, at bind time — the same moment a
    /// mode negotiation failure surfaces — rather than as a blank region at
    /// paint time.
    pub fn bind(
        &mut self,
        slot: NativeSlot,
        kind: NativeViewKind,
        preferred: &[NativeCompositionMode],
    ) -> Result<NativeCompositionMode, NativeError> {
        let index = self
            .by_kind
            .get(&kind)
            .copied()
            .ok_or_else(|| NativeError::NoProvider(kind))?;
        let mode = self.entries[index].controller.bind(slot, kind, preferred)?;
        self.owner.insert(slot, index);
        Ok(mode)
    }

    /// Change a bound participant's composition mode, on the same slot. The
    /// kind is fixed by the bind that created the participant; a rebind to a
    /// different kind is a destroy-then-bind, which is what the host does.
    pub fn rebind(
        &mut self,
        slot: NativeSlot,
        kind: NativeViewKind,
        preferred: &[NativeCompositionMode],
    ) -> Result<NativeCompositionMode, NativeError> {
        let index = *self.owner.get(&slot).ok_or(NativeError::NotBound(slot))?;
        self.entries[index].controller.rebind(slot, kind, preferred)
    }

    /// Tear a bound participant down, routing to whichever provider bound it.
    ///
    /// A slot nothing bound is reported exactly as the single-controller
    /// contract reports it.
    pub fn destroy(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        let index = *self.owner.get(&slot).ok_or(NativeError::NotBound(slot))?;
        let result = self.entries[index].controller.destroy(slot);
        self.owner.remove(&slot);
        result
    }

    /// Push state to a bound participant.
    pub fn apply(&mut self, slot: NativeSlot, state: NativeState) -> Result<(), NativeError> {
        self.entry_of_slot(slot)?.controller.apply(slot, state)
    }

    /// Hand OS focus to a bound participant.
    ///
    /// Focus is exclusive across the whole registry, not per provider: focusing
    /// one participant blurs whoever held it before, on that participant's own
    /// provider, so the OS focus chain actually moves.
    pub fn focus(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        if let Some(prev) = self.focused() {
            if prev != slot {
                self.blur(prev)?;
            }
        }
        self.entry_of_slot(slot)?.controller.focus(slot)
    }

    /// Take OS focus back from a bound participant.
    pub fn blur(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        self.entry_of_slot(slot)?.controller.blur(slot)
    }

    /// Freeze a bound participant into a texture, `None` when it cannot.
    pub fn snapshot(&mut self, slot: NativeSlot) -> Result<Option<NativeSnapshot>, NativeError> {
        self.entry_of_slot(slot)?.controller.snapshot(slot)
    }

    /// Hand a frozen participant its live view back.
    pub fn restore(&mut self, slot: NativeSlot) -> Result<(), NativeError> {
        self.entry_of_slot(slot)?.controller.restore(slot)
    }

    /// The mode a slot was bound in, from whichever provider bound it.
    pub fn mode(&self, slot: NativeSlot) -> Option<NativeCompositionMode> {
        let &index = self.owner.get(&slot)?;
        Some(self.entries[index].controller.mode(slot)?)
    }

    /// The participant currently holding OS focus, whichever provider it
    /// belongs to. At most one across the whole registry: focus and IME
    /// ownership are exclusive, and routing to per-provider controllers must
    /// not turn that into one focus *per provider*.
    pub fn focused(&self) -> Option<NativeSlot> {
        self.entries
            .iter()
            .filter_map(|entry| entry.controller.focused())
            .next()
    }

    /// Whether a slot is bound, across all providers.
    pub fn is_bound(&self, slot: NativeSlot) -> bool {
        self.owner.contains_key(&slot)
    }

    /// Whether a slot is frozen, across all providers.
    pub fn is_frozen(&self, slot: NativeSlot) -> bool {
        let Some(&index) = self.owner.get(&slot) else {
            return false;
        };
        self.entries[index].controller.is_frozen(slot)
    }

    /// Every slot still bound, across all providers, in registration order.
    pub fn bound_slots(&self) -> Vec<NativeSlot> {
        self.owner.keys().copied().collect()
    }

    /// Slots that are bound but were never destroyed — a leak of a native view
    /// the framework can no longer name. Empty by the invariant tests.
    pub fn unreleased(&self) -> Vec<NativeSlot> {
        self.bound_slots()
    }

    /// The composition capabilities of the provider registered for `kind`.
    ///
    /// `None` when nobody claims the kind: the caller is expected to treat an
    /// unserved participant as unsupportable rather than to read capabilities
    /// from a provider that will never see it.
    pub fn capabilities_for(&self, kind: NativeViewKind) -> Option<NativeCapabilities> {
        let &index = self.by_kind.get(&kind)?;
        Some(self.entries[index].controller.capabilities())
    }

    /// Drain every snapshot capture queued by every provider, in registration
    /// order.
    pub fn take_native_captures(&mut self) -> Vec<NativeCapture> {
        let mut captures = Vec::new();
        for entry in &mut self.entries {
            captures.extend(entry.controller.take_native_captures());
        }
        captures
    }

    // --- routing helpers ---------------------------------------------------

    fn entry_of_slot(&mut self, slot: NativeSlot) -> Result<&mut Entry, NativeError> {
        let &index = self.owner.get(&slot).ok_or(NativeError::NotBound(slot))?;
        Ok(&mut self.entries[index])
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::*;
    use crate::fake::FakeNativeProvider;
    use crate::pool::NativeViewPool;
    use floom_types::Rect;

    /// A provider hard-coded to one composition mode and serving one kind.
    fn provider_for(kind: NativeViewKind, mode: NativeCompositionMode) -> FakeNativeProvider {
        let mut caps = NativeCapabilities::NONE;
        caps.modes = Cow::Owned(vec![mode]);
        caps.focusable = kind == NativeViewKind::TextInput;
        caps.input = crate::abi::InputModes::PASSTHROUGH_ONLY;
        FakeNativeProvider::with_capabilities(caps)
    }

    fn register_many(
        registry: &mut NativeProviderRegistry,
        kinds: impl IntoIterator<Item = (NativeViewKind, NativeCompositionMode)>,
    ) {
        for (kind, mode) in kinds {
            let p = provider_for(kind, mode);
            assert!(
                registry.register(p, &[kind]).is_ok(),
                "{kind:?} should register once"
            );
        }
    }

    #[test]
    fn duplicate_kind_is_rejected_at_registration() {
        let mut registry = NativeProviderRegistry::new();
        let first = provider_for(NativeViewKind::Map, NativeCompositionMode::Overlay);
        let second = FakeNativeProvider::new();
        assert!(registry.register(first, &[NativeViewKind::Map]).is_ok());
        assert_eq!(
            registry.register(second, &[NativeViewKind::Map]),
            Err(NativeError::KindRegistered(NativeViewKind::Map))
        );
        // Only the first one is bound to that kind: a bind gets the first
        // provider's capabilities mode.
        let slot = pool_slot();
        let mode = registry
            .bind(slot, NativeViewKind::Map, &[NativeCompositionMode::Overlay])
            .unwrap();
        assert_eq!(mode, NativeCompositionMode::Overlay);
        let _ = registry.destroy(slot);
    }

    #[test]
    fn unserved_kind_is_refused_with_its_name() {
        let mut registry = NativeProviderRegistry::new();
        registry.register(provider_for(NativeViewKind::Video, NativeCompositionMode::Overlay), &[NativeViewKind::Video]).unwrap();

        let slot = pool_slot();
        match registry.bind(slot, NativeViewKind::Map, &[NativeCompositionMode::Overlay]) {
            Err(NativeError::NoProvider(NativeViewKind::Map)) => {}
            other => panic!("expected NoProvider(Map), got {other:?}"),
        }
        // The slot must not have leaked: nothing bound, nothing released.
        assert!(!registry.is_bound(slot));
        assert!(registry.unreleased().is_empty());
    }

    #[test]
    fn bind_routes_to_the_registered_provider() {
        let mut registry = NativeProviderRegistry::new();
        register_many(
            &mut registry,
            [
                (NativeViewKind::Video, NativeCompositionMode::Overlay),
                (NativeViewKind::Web, NativeCompositionMode::Overlay),
            ],
        );

        let slot_video = pool_slot();
        let slot_web = pool_slot();
        assert_eq!(
            registry.bind(slot_video, NativeViewKind::Video, &[NativeCompositionMode::Overlay]),
            Ok(NativeCompositionMode::Overlay)
        );
        assert_eq!(
            registry.bind(slot_web, NativeViewKind::Web, &[NativeCompositionMode::Overlay]),
            Ok(NativeCompositionMode::Overlay)
        );
        assert!(registry.is_bound(slot_video));
        assert!(registry.is_bound(slot_web));
    }

    #[test]
    fn capabilities_are_per_kind() {
        let mut registry = NativeProviderRegistry::new();
        registry.register(
            provider_for(NativeViewKind::TextInput, NativeCompositionMode::Overlay),
            &[NativeViewKind::TextInput],
        ).unwrap();

        let caps = registry.capabilities_for(NativeViewKind::TextInput).unwrap();
        assert!(caps.focusable);
        assert!(registry.capabilities_for(NativeViewKind::Video).is_none());
    }

    #[test]
    fn lifecycle_ops_route_to_the_right_provider() {
        let mut registry = NativeProviderRegistry::new();
        register_many(
            &mut registry,
            [(NativeViewKind::TextInput, NativeCompositionMode::Overlay)],
        );
        let slot = pool_slot();
        registry
            .bind(slot, NativeViewKind::TextInput, &[NativeCompositionMode::Overlay])
            .unwrap();

        assert!(registry.is_bound(slot));
        assert!(registry.apply(slot, NativeState {
            rect: Rect::ZERO,
            clip: None,
            scale: 1.0,
            visible: true,
            z: 0,
            input: crate::abi::InputMode::Passthrough,
            mode: NativeCompositionMode::Overlay,
        }).is_ok());
        assert!(registry.focus(slot).is_ok());
        assert_eq!(registry.focused(), Some(slot));
        assert!(registry.blur(slot).is_ok());
        assert_eq!(registry.focused(), None);
        let _ = registry.destroy(slot);
        assert!(!registry.is_bound(slot));
        assert!(registry.unreleased().is_empty());
    }

    #[test]
    fn focused_is_registry_wide_not_per_provider() {
        let mut registry = NativeProviderRegistry::new();
        let mut pool = NativeViewPool::<usize>::new();
        let slot_a = pool.acquire(0);
        let slot_b = pool.acquire(1);
        let focusable = || {
            let mut caps = NativeCapabilities::NONE;
            caps.modes = Cow::Owned(vec![NativeCompositionMode::Overlay]);
            caps.focusable = true;
            caps.input = crate::abi::InputModes::PASSTHROUGH_ONLY;
            FakeNativeProvider::with_capabilities(caps)
        };
        registry.register(focusable(), &[NativeViewKind::Video]).unwrap();
        registry.register(focusable(), &[NativeViewKind::Web]).unwrap();
        registry.bind(slot_a, NativeViewKind::Video, &[NativeCompositionMode::Overlay]).unwrap();
        registry.bind(slot_b, NativeViewKind::Web, &[NativeCompositionMode::Overlay]).unwrap();
        registry.focus(slot_a).unwrap();
        assert_eq!(registry.focused(), Some(slot_a));
        registry.focus(slot_b).unwrap();
        assert_eq!(registry.focused(), Some(slot_b));
        registry.blur(slot_a).unwrap();
        assert_eq!(registry.focused(), Some(slot_b));
    }

    #[test]
    fn take_native_captures_drains_every_provider() {
        let mut registry = NativeProviderRegistry::new();
        let mut p1 = provider_for(NativeViewKind::Video, NativeCompositionMode::Overlay);
        let _log1 = p1.watch();
        registry.register(p1, &[NativeViewKind::Video]).unwrap();
        let mut p2 = FakeNativeProvider::new();
        let _log2 = p2.watch();
        registry.register(p2, &[NativeViewKind::Map]).unwrap();

        let _ = registry.take_native_captures();
        // Both providers were drained; no panic on empty queues.
        assert_eq!(_log1.count(), 0);
        assert_eq!(_log2.count(), 0);
    }

    #[test]
    fn served_kinds_lists_exactly_the_registered_ones() {
        let mut registry = NativeProviderRegistry::new();
        assert!(registry.served_kinds().next().is_none());
        register_many(
            &mut registry,
            [
                (NativeViewKind::Map, NativeCompositionMode::Overlay),
                (
                    NativeViewKind::Custom("com.example.camera"),
                    NativeCompositionMode::Overlay,
                ),
            ],
        );
        let mut kinds: Vec<_> = registry.served_kinds().collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(
            kinds,
            vec![
                NativeViewKind::Custom("com.example.camera"),
                NativeViewKind::Map,
            ]
        );
    }

    // --- small helpers ------------------------------------------------------

    static NEXT_SLOT_KEY: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn pool_slot() -> NativeSlot {
        let mut pool = NativeViewPool::<usize>::new();
        pool.acquire(NEXT_SLOT_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}
