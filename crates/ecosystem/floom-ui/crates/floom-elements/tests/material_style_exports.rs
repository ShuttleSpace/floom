use floom_elements::button::ButtonStyle;
use floom_elements::card::CardStyle;
use floom_elements::checkbox::CheckboxStyle;
use floom_elements::dialog::DialogStyle;
use floom_elements::menu::MenuStyle;
use floom_elements::prelude::*;
use floom_elements::switch::SwitchStyle;
use floom_elements::text::TextStyle;
use floom_elements::text_field::TextFieldStyle;

#[test]
fn floom_elements_exports_design_system_neutral_style_type_names() {
    let _: ButtonStyle = Button::new("Save").resolved_style();
    let _: TextStyle = Text::new("Body").resolved_style();
    let _: TextFieldStyle = TextField::new().resolved_style();
    let _: CheckboxStyle = Checkbox::new("Accept").resolved_style();
    let _: SwitchStyle = Switch::new().resolved_style();
    let _: DialogStyle = Dialog::new().resolved_style();
    let _: MenuStyle = Menu::new().resolved_style();
    let _: CardStyle = Card::new().resolved_style();
}
