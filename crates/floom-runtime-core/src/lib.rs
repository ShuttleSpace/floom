//! Retained runtime core: generational ID allocation and element arena.

#![forbid(unsafe_code)]

mod arena;
mod dirty;
mod event;
mod id;
mod reactive_bridge;
mod runtime;
mod tree;

pub use arena::ElementArena;
pub use event::{EventPhase, EventRouter, Hit, HitRegion, HitTester, PointerCapture, RouteStop};
pub use dirty::{DirtyFlags, DirtySet, invalidate, propagate_layout};
pub use floom_types::ElementId;
pub use id::{IdAllocator, IdSlot};
pub use runtime::{Event, FrameStats, RootHandle, Runtime};
pub use reactive_bridge::{
    BindingRegistry, BindingReport, ReactiveBinding, ReactiveBindingId, SignalSource, bind_signal,
    ensure_ui_thread, unbind,
};
pub use tree::{ElementTree, append_child, detach, insert_child, move_child};
