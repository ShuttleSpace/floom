//! Application frame runtime: mount widgets, dispatch events, tick frames.

mod event;

pub use event::{
    EventPhase, EventRouter, Hit, HitRegion, HitTester, PointerCapture, RouteStop,
};

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use floem_core::{CreateContext, ElementKind, Widget};
use floem_reactive::RwSignal;
use floem_runtime_core::{
    BindingRegistry, DirtyFlags, DirtySet, ElementArena, ElementTree, ReactiveBinding,
};
use floem_scheduler::{Priority, Scheduler, TaskId};
use floem_types::{ElementId, Point};

/// Minimal input events for the new runtime path.
#[derive(Debug, Clone)]
pub enum Event {
    PointerClick { target: ElementId, position: Point },
    Custom { target: ElementId, name: &'static str },
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

/// Floem 2.0 frame runtime.
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
            arena.insert(floem_core::Element::new(
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
    ) -> floem_runtime_core::ReactiveBindingId {
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

    pub fn schedule(&mut self, priority: Priority, id: TaskId, run: floem_scheduler::TaskFn) {
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
