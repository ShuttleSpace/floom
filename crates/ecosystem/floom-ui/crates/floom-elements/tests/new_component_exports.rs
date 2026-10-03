use floom_elements::prelude::*;

#[test]
fn floom_elements_exports_icon_and_divider_contracts() {
    let icon = Icon::new("home");
    let divider = Divider::new().inset(8.0);

    assert_eq!(icon.props().name, "home");
    assert_eq!(divider.props().inset, 8.0);
}
