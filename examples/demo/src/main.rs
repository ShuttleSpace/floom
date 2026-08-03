use floem_elements::button::ButtonVariant;
use floem_elements::prelude::*;

fn main() {
    let _button = Button::new("Primary").variant(ButtonVariant::Primary);
    let _text = Text::new("floem-elements Demo");
    let _text_field = TextField::new().placeholder("Name");
    let _checkbox = Checkbox::new("Accept terms");
    let _switch = Switch::new();
    let _dialog = Dialog::new().title("Confirm");
    let _menu = Menu::new().item("Open", || {});
    let _card = Card::new().elevation(2);
}
