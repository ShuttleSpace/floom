//! Retained runtime core: generational ID allocation and element arena.

#![forbid(unsafe_code)]

mod arena;
mod id;

pub use arena::ElementArena;
pub use floem_types::ElementId;
pub use id::{IdAllocator, IdSlot};

