use floem_widget_cupertino_text_field::CupertinoTextField;

#[test]
fn text_field_exposes_neutral_contract_api() {
    let field = CupertinoTextField::new()
        .placeholder("Name")
        .initial_value("Ada")
        .password(true)
        .max_length(8)
        .disabled(true);

    assert_eq!(field.props().placeholder, "Name");
    assert_eq!(field.props().value, "Ada");
    assert!(field.props().is_password);
    assert_eq!(field.props().max_length, Some(8));
    assert!(field.props().disabled);
    assert_eq!(field.resolved_style().design_system, "cupertino");
}
