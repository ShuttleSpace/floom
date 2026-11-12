//! The catalogue: every component the `floom-ui` satellite provides.
//!
//! The pages are grouped by category, and the categories come in the order
//! the rail shows them, so the order of the pages here is the order a reader
//! meets them.

use crate::pages::{actions, data, feedback, forms, layout, motion, ComponentPage};

/// Every page of the gallery, grouped by category.
pub fn pages() -> Vec<ComponentPage> {
    let mut all = Vec::with_capacity(98);
    all.extend(actions::pages());
    all.extend(forms::pages());
    all.extend(data::pages());
    all.extend(feedback::pages());
    all.extend(layout::pages());
    all.extend(motion::pages());
    all
}

/// How many pages the gallery holds.
pub fn page_count() -> usize {
    pages().len()
}

/// The page of one component, by the name a reader sees.
pub fn page_named(name: &str) -> Option<ComponentPage> {
    pages().into_iter().find(|page| page.name == name)
}
