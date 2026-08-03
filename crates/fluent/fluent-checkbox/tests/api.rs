use floem_widget_fluent_checkbox::FluentCheckbox;

#[test]
fn checkbox_exposes_neutral_contract_api() {
    let checkbox = FluentCheckbox::new("Accept").checked(true).disabled(true);

    assert_eq!(checkbox.props().label, "Accept");
    assert!(checkbox.props().checked);
    assert!(checkbox.props().disabled);
    assert_eq!(checkbox.resolved_style().design_system, "fluent");
}
