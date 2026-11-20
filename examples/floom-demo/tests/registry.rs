//! The registry must describe the satellite, not a list kept by hand.
//!
//! Every component the `floom-ui` satellite provides gets a page, and the
//! pages are grouped by category in the order the rail shows them. A
//! component added to the satellite without a page -- or a page for a
//! component the satellite does not have -- fails here.

use floom_demo::pages::Category;
use floom_demo::registry;

#[test]
fn the_registry_holds_one_page_per_component() {
    assert_eq!(registry::page_count(), 98);
}

#[test]
fn every_component_of_the_satellite_has_exactly_one_page() {
    // The satellite names a component `text-field`; a page module is
    // `text_field`. The assertion is about the set, so both sides are compared
    // under one spelling rather than pinning either convention.
    let mut keyed: Vec<String> = registry::pages()
        .iter()
        .map(|p| p.component.replace('-', "_"))
        .collect();
    keyed.sort_unstable();
    let mut expected: Vec<String> = floom_demo::ds::material::COMPONENT_KEYS
        .iter()
        .map(|k| k.replace('-', "_"))
        .collect();
    expected.sort_unstable();
    assert_eq!(
        keyed, expected,
        "the registry and the satellite disagree about which components exist"
    );
}

#[test]
fn every_page_has_a_unique_name() {
    let mut names: Vec<&str> = registry::pages().iter().map(|p| p.name).collect();
    names.sort_unstable();
    let total = names.len();
    names.dedup();
    assert_eq!(names.len(), total, "two pages share a name");
}

#[test]
fn pages_are_grouped_by_category_in_category_order() {
    let pages = registry::pages();
    let mut seen: Vec<Category> = Vec::new();
    for page in &pages {
        if seen.last() != Some(&page.category) {
            seen.push(page.category);
        }
    }
    assert_eq!(seen, Category::ALL.to_vec());
}

#[test]
fn every_category_has_a_label_and_a_slug() {
    for category in Category::ALL {
        assert!(!category.label().is_empty());
        assert!(!category.slug().is_empty());
    }
    let slugs: Vec<&str> = Category::ALL.iter().map(|c| c.slug()).collect();
    let mut unique = slugs.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(slugs.len(), unique.len(), "two categories share a slug");
}
