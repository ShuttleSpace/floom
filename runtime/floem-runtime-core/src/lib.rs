//! Retained runtime core: generational ID allocation and (later) element arena.

#![forbid(unsafe_code)]

mod id;

pub use floem_types::ElementId;
pub use id::{IdAllocator, IdSlot};
