use floem_core_theme::props::button::{ButtonSize, ButtonVariant};
use floem_widget_fluent_button::FluentButton;

#[test]
fn button_exposes_neutral_contract_api() {
    let button = FluentButton::new("Save")
        .variant(ButtonVariant::Secondary)
        .size(ButtonSize::Large)
        .disabled(true)
        .loading(true);

    assert_eq!(button.props().label, "Save");
    assert_eq!(button.props().variant, ButtonVariant::Secondary);
    assert_eq!(button.props().size, ButtonSize::Large);
    assert!(button.props().disabled);
    assert!(button.props().loading);
    assert_eq!(button.resolved_style().design_system, "fluent");
}
