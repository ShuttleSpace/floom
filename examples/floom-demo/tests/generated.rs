//! The generated files must describe the satellite, not a hand-kept list.
//!
//! `tools/gen.py` derives the crate's manifest and its per-design-system
//! aliases from the `floom-ui` satellite. These tests are what holds the
//! generator to that: they fail if the component set drifts, if a design
//! system is missing a component, or if a data directory is mistaken for a
//! crate.

use floom_demo::ds;

#[test]
fn the_ds_module_aliases_every_component_in_all_three_design_systems() {
    assert_eq!(ds::material::COMPONENTS.len(), 98);
    assert_eq!(ds::cupertino::COMPONENTS.len(), 98);
    assert_eq!(ds::fluent::COMPONENTS.len(), 98);
}

#[test]
fn the_component_set_is_the_intersection_of_the_three_design_systems() {
    // The two data directories that exist only in Cupertino/Fluent must not
    // appear: they are not crates.
    for names in [
        ds::material::COMPONENTS,
        ds::cupertino::COMPONENTS,
        ds::fluent::COMPONENTS,
    ] {
        assert!(!names.contains(&"IconPaths"));
        assert!(names.contains(&"Button"));
        assert!(names.contains(&"TextField"));
        assert!(names.contains(&"ProgressLinear"));
    }
    // All three design systems expose the same component set.
    let mut material: Vec<&str> = ds::material::COMPONENTS.to_vec();
    material.sort_unstable();
    let mut cupertino: Vec<&str> = ds::cupertino::COMPONENTS.to_vec();
    cupertino.sort_unstable();
    let mut fluent: Vec<&str> = ds::fluent::COMPONENTS.to_vec();
    fluent.sort_unstable();
    assert_eq!(material, cupertino);
    assert_eq!(material, fluent);
}

#[test]
fn the_aliases_are_camel_case_component_names() {
    // The alias is what a page module writes, so it has to be the component's
    // name in CamelCase -- `text-field` becomes `TextField`.
    assert!(ds::material::COMPONENTS.contains(&"TextField"));
    assert!(ds::material::COMPONENTS.contains(&"RadioGroup"));
    assert!(ds::material::COMPONENTS.contains(&"FloatingActionButton"));
    assert!(
        ds::material::COMPONENTS.contains(&"OTPInput")
            || ds::material::COMPONENTS.contains(&"OtpInput")
    );
}
