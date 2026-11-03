use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::divider::{DividerOrientation, DividerProps};
use floom_core_theme::props::icon::IconProps;
use floom_core_theme::theme::{CupertinoTheme, FluentTheme, MaterialTheme, Theme};

#[test]
fn icon_props_capture_accessible_icon_contract() {
    let props = IconProps {
        name: "home".to_string(),
        size: Some(20.0),
        label: Some("Home".to_string()),
        decorative: false,
    };

    assert_eq!(props.name, "home");
    assert_eq!(props.size, Some(20.0));
    assert_eq!(props.label.as_deref(), Some("Home"));
    assert!(!props.decorative);
}

#[test]
fn divider_props_capture_orientation_and_inset() {
    let props = DividerProps {
        orientation: DividerOrientation::Vertical,
        inset: 12.0,
    };

    assert_eq!(props.orientation, DividerOrientation::Vertical);
    assert_eq!(props.inset, 12.0);
}

#[test]
fn themes_expose_icon_and_divider_component_tokens() {
    let material = MaterialTheme::new();
    let cupertino = CupertinoTheme::new();
    let fluent = FluentTheme::new();

    let ComponentTokens::Icon(material_icon) = material.component(ComponentId::Icon) else {
        panic!("material icon tokens missing")
    };
    let ComponentTokens::Divider(material_divider) = material.component(ComponentId::Divider)
    else {
        panic!("material divider tokens missing")
    };
    let ComponentTokens::Icon(cupertino_icon) = cupertino.component(ComponentId::Icon) else {
        panic!("cupertino icon tokens missing")
    };
    let ComponentTokens::Divider(fluent_divider) = fluent.component(ComponentId::Divider) else {
        panic!("fluent divider tokens missing")
    };

    assert_eq!(material_icon.size, 24.0);
    assert_eq!(material_divider.thickness, 1.0);
    assert_eq!(cupertino_icon.size, 22.0);
    assert_eq!(fluent_divider.thickness, 1.0);
}
