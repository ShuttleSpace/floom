use floem_widget_fluent_text::FluentText;

#[test]
fn text_exposes_neutral_contract_api() {
    let text = FluentText::new("Body").max_lines(2).selectable(true);

    assert_eq!(text.props().content, "Body");
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);
    assert_eq!(text.resolved_style().design_system, "fluent");
}
