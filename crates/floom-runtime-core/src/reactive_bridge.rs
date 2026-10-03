//! Bridge Floem reactive signals to element dirty flags.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use floem_legacy_reactive::{Effect, ReactiveId, Runtime, RwSignal, Scope, SignalGet};
use floem_types::ElementId;

use crate::arena::ElementArena;
use crate::dirty::{DirtyFlags, DirtySet};

/// Opaque handle for a reactive → element binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReactiveBindingId(u32);

/// Description of a binding from reactive reads to an element property.
pub struct ReactiveBinding {
    pub element: ElementId,
    pub flags: DirtyFlags,
}

/// Where a binding's updates come from: the reactive signal, what it is called
/// in the app, and which element property it drives. This is the metadata the
/// "why did this update?" diagnostic needs — a dirty flag only says that
/// something changed, not what changed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalSource {
    /// The reactive runtime's own stable id for the signal.
    pub signal: ReactiveId,
    /// The app-level name ("heading_text", "cart_total").
    pub label: String,
    /// The element property this signal drives ("label", "background").
    pub property: String,
}

/// One binding as the update-path report carries it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindingReport {
    pub id: ReactiveBindingId,
    pub element: ElementId,
    pub flags: DirtyFlags,
    pub runs: u32,
    /// `None` for bindings registered without source metadata.
    pub source: Option<SignalSource>,
}

struct LiveBinding {
    scope: Scope,
    element: ElementId,
    flags: DirtyFlags,
    /// Number of times the binding effect ran (including the initial run).
    runs: Rc<RefCell<u32>>,
}

/// Registry of signal→element invalidation bindings.
#[derive(Default)]
pub struct BindingRegistry {
    bindings: Vec<Option<LiveBinding>>,
    sources: HashMap<ReactiveBindingId, SignalSource>,
}

impl BindingRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Bind `signal` so that updates mark `binding.element` with `binding.flags`.
    ///
    /// Sibling elements are not invalidated. Call [`ensure_ui_thread`] first.
    pub fn bind_signal<T: Clone + 'static>(
        &mut self,
        arena: Rc<RefCell<ElementArena>>,
        dirty: Rc<RefCell<DirtySet>>,
        binding: ReactiveBinding,
        signal: RwSignal<T>,
    ) -> ReactiveBindingId {
        let runs = Rc::new(RefCell::new(0u32));
        let runs_effect = runs.clone();
        let element = binding.element;
        let flags = binding.flags;

        let scope = Scope::new();
        scope.enter(|| {
            let arena = arena.clone();
            let dirty = dirty.clone();
            Effect::new(move |_| {
                let _value = signal.get();
                *runs_effect.borrow_mut() += 1;
                let arena_ref = arena.borrow();
                dirty.borrow_mut().invalidate(&arena_ref, element, flags);
            });
        });

        self.alloc_slot(LiveBinding {
            scope,
            element,
            flags,
            runs,
        })
    }

    pub fn unbind(&mut self, id: ReactiveBindingId) -> bool {
        let Some(slot) = self.bindings.get_mut(id.0 as usize) else {
            return false;
        };
        let Some(live) = slot.take() else {
            return false;
        };
        live.scope.dispose();
        true
    }

    pub fn run_count(&self, id: ReactiveBindingId) -> Option<u32> {
        self.bindings
            .get(id.0 as usize)?
            .as_ref()
            .map(|b| *b.runs.borrow())
    }

    pub fn binding_element(&self, id: ReactiveBindingId) -> Option<ElementId> {
        self.bindings
            .get(id.0 as usize)?
            .as_ref()
            .map(|b| b.element)
    }

    pub fn binding_flags(&self, id: ReactiveBindingId) -> Option<DirtyFlags> {
        self.bindings.get(id.0 as usize)?.as_ref().map(|b| b.flags)
    }

    /// Bind a signal and record *where its updates come from*.
    ///
    /// Same behaviour as [`Self::bind_signal`]; the source is diagnostics
    /// metadata and does not affect invalidation.
    pub fn bind_signal_with_source<T: Clone + 'static>(
        &mut self,
        arena: Rc<RefCell<ElementArena>>,
        dirty: Rc<RefCell<DirtySet>>,
        binding: ReactiveBinding,
        signal: RwSignal<T>,
        source: SignalSource,
    ) -> ReactiveBindingId {
        let id = self.bind_signal(arena, dirty, binding, signal);
        self.sources.insert(id, source);
        id
    }

    /// The source recorded for a binding, if any.
    pub fn source_of(&self, id: ReactiveBindingId) -> Option<&SignalSource> {
        self.sources.get(&id)
    }

    /// Every binding targeting `element`, with its run count and source — the
    /// "why did this update?" report for one element. Order is registration
    /// order, so a report is reproducible.
    pub fn reports_for_element(&self, element: ElementId) -> Vec<BindingReport> {
        self.bindings
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                let live = slot.as_ref()?;
                if live.element != element {
                    return None;
                }
                let id = ReactiveBindingId(index as u32);
                Some(BindingReport {
                    id,
                    element: live.element,
                    flags: live.flags,
                    runs: *live.runs.borrow(),
                    source: self.sources.get(&id).cloned(),
                })
            })
            .collect()
    }

    fn alloc_slot(&mut self, live: LiveBinding) -> ReactiveBindingId {
        if let Some((index, slot)) = self
            .bindings
            .iter_mut()
            .enumerate()
            .find(|(_, s)| s.is_none())
        {
            *slot = Some(live);
            return ReactiveBindingId(index as u32);
        }
        let index = self.bindings.len() as u32;
        self.bindings.push(Some(live));
        ReactiveBindingId(index)
    }
}

/// Plan-shaped helpers.
pub fn bind_signal<T: Clone + 'static>(
    registry: &mut BindingRegistry,
    arena: Rc<RefCell<ElementArena>>,
    dirty: Rc<RefCell<DirtySet>>,
    binding: ReactiveBinding,
    signal: RwSignal<T>,
) -> ReactiveBindingId {
    registry.bind_signal(arena, dirty, binding, signal)
}

pub fn unbind(registry: &mut BindingRegistry, id: ReactiveBindingId) -> bool {
    registry.unbind(id)
}

/// Ensure the reactive runtime treats this thread as the UI thread.
pub fn ensure_ui_thread() {
    Runtime::init_on_ui_thread();
}

#[cfg(test)]
#[path = "reactive_bridge_tests.rs"]
mod reactive_bridge_tests;
