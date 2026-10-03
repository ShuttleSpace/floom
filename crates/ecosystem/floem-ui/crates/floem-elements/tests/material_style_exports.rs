use floem_elements::button::ButtonStyle;
use floem_elements::card::CardStyle;
use floem_elements::checkbox::CheckboxStyle;
use floem_elements::dialog::DialogStyle;
use floem_elements::menu::MenuStyle;
use floem_elements::prelude::*;
use floem_elements::switch::SwitchStyle;
use floem_elements::text::TextStyle;
use floem_elements::text_field::TextFieldStyle;

#[test]
fn floem_elements_exports_design_system_neutral_style_type_names() {
    let _: ButtonStyle = Button::new("Save").resolved_style();
    let _: TextStyle = Text::new("Body").resolved_style();
    let _: TextFieldStyle = TextField::new().resolved_style();
    let _: CheckboxStyle = Checkbox::new("Accept").resolved_style();
    let _: SwitchStyle = Switch::new().resolved_style();
    let _: DialogStyle = Dialog::new().resolved_style();
    let _: MenuStyle = Menu::new().resolved_style();
    let _: CardStyle = Card::new().resolved_style();
}
