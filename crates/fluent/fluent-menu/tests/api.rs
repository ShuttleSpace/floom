use floem_widget_fluent_menu::FluentMenu;

#[test]
fn menu_exposes_neutral_contract_api() {
    let menu = FluentMenu::new().item("Open", || {}).open(true);

    assert_eq!(menu.props().items.len(), 1);
    assert_eq!(menu.props().items[0].label, "Open");
    assert!(menu.props().open);
    assert_eq!(menu.resolved_style().design_system, "fluent");
}
