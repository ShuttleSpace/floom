//! Retained runtime core: generational ID allocation and element arena.

#![forbid(unsafe_code)]

mod arena;
mod dirty;
mod id;
mod reactive_bridge;
mod tree;

pub use arena::ElementArena;
pub use dirty::{DirtyFlags, DirtySet, invalidate, propagate_layout};
pub use floem_types::ElementId;
pub use id::{IdAllocator, IdSlot};
pub use reactive_bridge::{
    BindingRegistry, ReactiveBinding, ReactiveBindingId, bind_signal, ensure_ui_thread, unbind,
};
pub use tree::{ElementTree, append_child, detach, insert_child, move_child};

