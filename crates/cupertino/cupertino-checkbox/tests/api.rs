use floem_widget_cupertino_checkbox::CupertinoCheckbox;

#[test]
fn checkbox_exposes_neutral_contract_api() {
    let checkbox = CupertinoCheckbox::new("Accept")
        .checked(true)
        .disabled(true);

    assert_eq!(checkbox.props().label, "Accept");
    assert!(checkbox.props().checked);
    assert!(checkbox.props().disabled);
    assert_eq!(checkbox.resolved_style().design_system, "cupertino");
}
