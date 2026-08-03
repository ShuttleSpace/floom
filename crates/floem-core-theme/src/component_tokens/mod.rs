pub mod material;

mod button;
mod card;
mod checkbox;
mod dialog;
mod divider;
mod icon;
mod menu;
mod switch;
mod text;
mod text_field;

pub use button::ButtonTokens;
pub use card::CardTokens;
pub use checkbox::CheckboxTokens;
pub use dialog::DialogTokens;
pub use divider::DividerTokens;
pub use icon::IconTokens;
pub use menu::MenuTokens;
pub use switch::SwitchTokens;
pub use text::TextTokens;
pub use text_field::TextFieldTokens;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentId {
    Button,
    Text,
    TextField,
    Checkbox,
    Switch,
    Dialog,
    Menu,
    Card,
    Icon,
    Divider,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentTokens {
    Button(ButtonTokens),
    Text(TextTokens),
    TextField(TextFieldTokens),
    Checkbox(CheckboxTokens),
    Switch(SwitchTokens),
    Dialog(DialogTokens),
    Menu(MenuTokens),
    Card(CardTokens),
    Icon(IconTokens),
    Divider(DividerTokens),
}
