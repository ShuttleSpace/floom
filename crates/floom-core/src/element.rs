//! Persistent element identity shell (storage lives in runtime-core later).

use floom_types::ElementId;

/// Discriminator for element roles at the contract layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ElementKind {
    #[default]
    Host,
    Primitive,
    Composition,
}

/// Minimal element identity record exposed to widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Element {
    pub id: ElementId,
    pub kind: ElementKind,
    pub parent: Option<ElementId>,
}

impl Element {
    pub fn new(id: ElementId, kind: ElementKind) -> Self {
        Self {
            id,
            kind,
            parent: None,
        }
    }
}
