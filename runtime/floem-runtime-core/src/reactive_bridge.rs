//! Bridge Floem reactive signals to element dirty flags.

use std::cell::RefCell;
use std::rc::Rc;

use floem_legacy_reactive::{Effect, Runtime, RwSignal, Scope, SignalGet};
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
