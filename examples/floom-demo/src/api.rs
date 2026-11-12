//! The API tables a component page carries.
//!
//! A page states what a component takes and what it gives back: the props it
//! reads, the events it fires, and the methods it exposes. The props are
//! derived from the library's own sources (see [`crate::props_sources`]), so
//! only the events and the methods are written by hand.

/// One row of an API table.
///
/// The columns are the same for a prop, an event and a method: what it is
/// called, what it carries, what it defaults to, and what it is for. For an
/// event the "default" column holds the payload it carries; for a method it
/// holds what it returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiRow {
    /// The prop's, event's or method's name.
    pub name: String,
    /// The type it takes or gives.
    pub type_: String,
    /// Its default value, or the payload an event carries.
    pub default: String,
    /// What it is for.
    pub description: String,
}

impl ApiRow {
    /// A row with all four columns filled in.
    pub fn new(name: &str, type_: &str, default: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            type_: type_.to_string(),
            default: default.to_string(),
            description: description.to_string(),
        }
    }
}

/// The API of one component: its props, its events and its methods.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiDoc {
    /// The props the component reads.
    pub props: Vec<ApiRow>,
    /// The events the component fires.
    pub events: Vec<ApiRow>,
    /// The methods the component exposes.
    pub methods: Vec<ApiRow>,
}

impl ApiDoc {
    /// A document with nothing in it yet.
    pub fn empty() -> Self {
        Self::default()
    }
}
