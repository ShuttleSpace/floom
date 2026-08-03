use floem_widget_cupertino_dialog::CupertinoDialog;

#[test]
fn dialog_exposes_neutral_contract_api() {
    let dialog = CupertinoDialog::new()
        .title("Confirm")
        .content("Continue?")
        .open(true);

    assert_eq!(dialog.props().title, "Confirm");
    assert_eq!(dialog.props().content, "Continue?");
    assert!(dialog.props().open);
    assert_eq!(dialog.resolved_style().design_system, "cupertino");
}
