use floem_widget_cupertino_card::CupertinoCard;

#[test]
fn card_exposes_neutral_contract_api() {
    let card = CupertinoCard::new().elevation(3);

    assert_eq!(card.props().elevation, 3);
    assert_eq!(card.resolved_style().design_system, "cupertino");
}
