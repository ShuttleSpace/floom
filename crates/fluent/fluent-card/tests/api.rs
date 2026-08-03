use floem_widget_fluent_card::FluentCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = FluentCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "fluent");
}
