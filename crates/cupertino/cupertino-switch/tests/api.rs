use floem_widget_cupertino_switch::CupertinoSwitch;

#[test]
fn switch_exposes_neutral_contract_api() {
    let switch = CupertinoSwitch::new().checked(true).disabled(true);

    assert!(switch.props().checked);
    assert!(switch.props().disabled);
    assert_eq!(switch.resolved_style().design_system, "cupertino");
}
