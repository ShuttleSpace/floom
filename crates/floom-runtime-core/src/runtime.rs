//! The convenience runtime, moved here from the crate that used to hold it.
//!
//! ## Why this moved
//!
//! `floom-runtime` was a 414-line crate whose entire production value was one
//! module: `event.rs`, the hit-tester and event router. `floom-app` imported four
//! symbols from it and nothing else in the workspace imported the crate at all.
//! Everything else here — [`Runtime`], [`Event`], [`FrameStats`], [`RootHandle`]
//! — is a convenience wrapper over facilities this crate already had: the
//! element arena, the dirty set, the element tree, the scheduler.
//!
//! So the split was along a line that did not exist: a separate crate whose
//! only used content was also available in its own dependency. `floom-app`
//! depends on this crate; `floom-devtools` and `floom-hot-reload` depend on this
//! crate. Making the event router reachable through a second crate identity cost
//! a workspace entry and a version constraint and bought nothing.
//!
//! ## What this costs
//!
//! `floom-devtools` and `floom-hot-reload` now compile the scheduler and the
//! event router as well. The scheduler is dependency-free and the router is one
//! module, so this is a hundred-odd lines against several thousand — a fair
//! trade for one fewer crate in the graph.
//!
//! ## What this does not do
//!

//! It does not prune [`Runtime`]. It has no callers inside this workspace, and
//! deleting a public API because the workspace does not use it is a semver break
//! justified by nothing. It stays, and it stays tested.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use floom_core::{CreateContext, ElementKind, Widget};
use floom_signal::RwSignal;
use floom_scheduler::{Priority, Scheduler, TaskFn, TaskId};
use floom_types::{ElementId, Point};

use crate::reactive_bridge::{BindingRegistry, ReactiveBinding, ReactiveBindingId};
use crate::DirtyFlags;
use crate::DirtySet;
use crate::ElementArena;
use crate::ElementTree;

/// Minimal input events for the new runtime path.
#[derive(Debug, Clone)]
pub enum Event {
    PointerClick {
        target: ElementId,
        position: Point,
    },
    Custom {
        target: ElementId,
        name: &'static str,
    },
}

/// Per-frame instrumentation counters.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrameStats {
    pub ticks: u64,
    pub tasks_run: u64,
    pub elements_updated: u64,
    pub last_frame_tasks: u64,
}

/// Handle to a mounted root.
#[derive(Debug, Clone, Copy)]
pub struct RootHandle {
    pub root: ElementId,
}

struct CreateCx<'a> {
    arena: &'a mut ElementArena,
    tree: &'a mut ElementTree,
    parent: Option<ElementId>,
}

impl CreateContext for CreateCx<'_> {
    fn alloc_child(&mut self) -> ElementId {
        let id = self.arena.insert_primitive();
        if let Some(parent) = self.parent {
            let _ = self.tree.append_child(self.arena, parent, id);
        }
        id
    }
}

/// Floom 2.0 frame runtime.
pub struct Runtime {
    arena: Rc<RefCell<ElementArena>>,
    tree: ElementTree,
    dirty: Rc<RefCell<DirtySet>>,
    bindings: BindingRegistry,
    scheduler: Scheduler,
    frame_task: TaskId,
    frame_requested: bool,
    stats: FrameStats,
    click_handlers: Vec<(ElementId, Box<dyn FnMut()>)>,
    update_counts: Vec<u32>,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let mut scheduler = Scheduler::new();
        let frame_task = scheduler.alloc_task_id();
        Self {
            arena: Rc::new(RefCell::new(ElementArena::new())),
            tree: ElementTree::new(),
            dirty: Rc::new(RefCell::new(DirtySet::new())),
            bindings: BindingRegistry::new(),
            scheduler,
            frame_task,
            frame_requested: false,
            stats: FrameStats::default(),
            click_handlers: Vec::new(),
            update_counts: Vec::new(),
        }
    }

    pub fn stats(&self) -> &FrameStats {
        &self.stats
    }

    pub fn update_count(&self, id: ElementId) -> u32 {
        self.update_counts
            .get(id.index() as usize)
            .copied()
            .unwrap_or(0)
    }

    pub fn arena(&self) -> Rc<RefCell<ElementArena>> {
        self.arena.clone()
    }

    pub fn dirty(&self) -> Rc<RefCell<DirtySet>> {
        self.dirty.clone()
    }

    pub fn tree(&self) -> &ElementTree {
        &self.tree
    }

    /// Mount `widget` as a child of a new host element.
    pub fn mount(&mut self, widget: &dyn Widget) -> RootHandle {
        let host = {
            let mut arena = self.arena.borrow_mut();
            arena.insert(floom_core::Element::new(
                ElementId::INVALID,
                ElementKind::Host,
            ))
        };
        let root = {
            let mut arena = self.arena.borrow_mut();
            let mut cx = CreateCx {
                arena: &mut arena,
                tree: &mut self.tree,
                parent: Some(host),
            };
            widget.create(&mut cx)
        };
        RootHandle { root }
    }

    /// Register a click handler for an element.
    pub fn on_click(&mut self, id: ElementId, handler: impl FnMut() + 'static) {
        self.click_handlers.push((id, Box::new(handler)));
    }

    /// Bind a signal to element dirty flags.
    pub fn bind_signal<T: Clone + 'static>(
        &mut self,
        element: ElementId,
        flags: DirtyFlags,
        signal: RwSignal<T>,
    ) -> ReactiveBindingId {
        self.bindings.bind_signal(
            self.arena.clone(),
            self.dirty.clone(),
            ReactiveBinding { element, flags },
            signal,
        )
    }

    pub fn dispatch(&mut self, event: Event) {
        match event {
            Event::PointerClick { target, .. } | Event::Custom { target, .. } => {
                for (id, handler) in &mut self.click_handlers {
                    if *id == target {
                        handler();
                    }
                }
                self.request_frame();
            }
        }
    }

    pub fn request_frame(&mut self) {
        self.frame_requested = true;
        let _ = self.frame_task;
    }

    /// Advance the frame: apply pending dirty elements and drain the scheduler.
    pub fn tick(&mut self, now: Instant) {
        self.stats.ticks += 1;
        self.frame_requested = false;

        let pending = self.dirty.borrow_mut().drain_pending();
        for id in pending {
            self.bump_update_count(id);
            self.stats.elements_updated += 1;
            self.dirty.borrow_mut().clear(id);
        }

        let ran = self.scheduler.tick(now);
        self.stats.tasks_run += ran as u64;
        self.stats.last_frame_tasks = ran as u64;
    }

    pub fn schedule(&mut self, priority: Priority, id: TaskId, run: TaskFn) {
        self.scheduler.schedule(priority, id, run);
    }

    pub fn scheduler_mut(&mut self) -> &mut Scheduler {
        &mut self.scheduler
    }

    pub fn frame_requested(&self) -> bool {
        self.frame_requested
    }

    fn bump_update_count(&mut self, id: ElementId) {
        let index = id.index() as usize;
        if index >= self.update_counts.len() {
            self.update_counts.resize(index + 1, 0);
        }
        self.update_counts[index] += 1;
    }
}
