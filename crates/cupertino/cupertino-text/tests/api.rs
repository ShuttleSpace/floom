use floem_widget_cupertino_text::CupertinoText;

#[test]
fn text_exposes_neutral_contract_api() {
    let text = CupertinoText::new("Body").max_lines(2).selectable(true);

    assert_eq!(text.props().content, "Body");
    assert_eq!(text.props().max_lines, Some(2));
    assert!(text.props().is_selectable);
    assert_eq!(text.resolved_style().design_system, "cupertino");
}
