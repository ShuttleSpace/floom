//! The props tables, checked against the library they are derived from.
//!
//! A props table is only worth reading if it is the library's own truth. The
//! gallery parses these tables out of `floom-core-theme`'s own props sources at
//! run time, so a table cannot drift from the struct it describes -- and these
//! tests are what prove that.

use floom_demo::props_src;

/// The button's props table is the button's props struct.
#[test]
fn the_button_props_table_matches_the_libraries_button_props() {
    let rows = props_src::props_for("button");
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "label", "size", "variant", "disabled", "loading", "on_click"
        ]
    );

    let size = rows
        .iter()
        .find(|row| row.name == "size")
        .expect("a size row");
    assert_eq!(size.type_, "ButtonSize");
    assert_eq!(size.default, "ButtonSize::Medium");
}

/// A module nobody has heard of yields nothing, rather than panicking.
#[test]
fn an_unknown_props_module_yields_no_rows() {
    assert!(props_src::props_for("not_a_component").is_empty());
}

/// Every component's table covers its props struct exactly.
///
/// `field_count` re-counts the fields by a path independent of the row-building
/// path, so a parser that quietly drops a field fails here.
#[test]
fn every_component_has_a_props_table_covering_its_props_struct() {
    for (component, module) in props_src::COMPONENT_PROPS {
        let rows = props_src::props_for(module);
        let expected = props_src::field_count(module);
        assert_eq!(
            rows.len(),
            expected,
            "{component}'s table has {} rows, its props struct has {expected}",
            rows.len()
        );
    }
}

/// Every page in the gallery shows the props table of its own component.
///
/// A component whose props struct is empty -- the three transitions that only
/// take a `show` flag, which lives outside the struct -- shows no rows, and
/// that is right: the struct has no fields to show.
#[test]
fn every_page_shows_the_props_table_of_its_component() {
    for page in floom_demo::registry::pages() {
        match props_src::module_for(page.component) {
            Some(module) => {
                let expected = props_src::field_count(module);
                assert_eq!(
                    page.api.props.len(),
                    expected,
                    "{} shows {} props rows, its props struct has {expected}",
                    page.name,
                    page.api.props.len()
                );
            }
            None => assert!(
                page.api.props.is_empty(),
                "{} has no props module but shows props rows",
                page.name
            ),
        }
    }
}

/// The two components that have no props struct show no props rows.
#[test]
fn a_component_without_a_props_struct_shows_no_props_rows() {
    for name in ["App", "Main"] {
        let page = floom_demo::registry::page_named(name).expect("a page");
        assert!(
            page.api.props.is_empty(),
            "{name} has no props struct but shows {} rows",
            page.api.props.len()
        );
    }
}

/// A documented prop carries the library's own description.
#[test]
fn a_documented_prop_carries_its_own_description() {
    let rows = props_src::props_for("navigation_drawer");
    let open = rows
        .iter()
        .find(|row| row.name == "open")
        .expect("an open row");
    assert_eq!(open.type_, "bool");
    assert_eq!(open.default, "false");
    assert_eq!(
        open.description,
        "Whether the drawer is open on first render."
    );
}

/// A props struct with no `Default` impl says so rather than inventing one.
#[test]
fn a_props_struct_without_a_default_says_so() {
    let rows = props_src::props_for("badge");
    assert!(!rows.is_empty(), "the badge has props");
    for row in &rows {
        assert_eq!(row.default, "—", "{} has no default", row.name);
    }
}

/// A prop whose type carries a comma keeps its whole type.
#[test]
fn a_generic_prop_type_keeps_its_commas() {
    let rows = props_src::props_for("defaults_provider");
    let defaults = rows
        .iter()
        .find(|row| row.name == "defaults")
        .expect("a defaults row");
    assert_eq!(defaults.type_, "HashMap<String, String>");
}
