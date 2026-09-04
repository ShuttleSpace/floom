//! Retained runtime core: generational ID allocation and element arena.

#![forbid(unsafe_code)]

mod arena;
mod dirty;
mod id;

pub use arena::ElementArena;
pub use dirty::{DirtyFlags, DirtySet, invalidate, propagate_layout};
pub use floem_types::ElementId;
pub use id::{IdAllocator, IdSlot};

