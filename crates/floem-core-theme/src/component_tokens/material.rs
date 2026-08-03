use super::*;
use crate::tokens::spacing::SpacingTokens;

#[derive(Debug, Clone)]
pub struct MaterialComponentTokens {
    button: ButtonTokens,
    text: TextTokens,
    text_field: TextFieldTokens,
    checkbox: CheckboxTokens,
    switch: SwitchTokens,
    dialog: DialogTokens,
    menu: MenuTokens,
    card: CardTokens,
}

impl MaterialComponentTokens {
    pub fn new() -> Self {
        let spacing = SpacingTokens::default();
        Self {
            button: ButtonTokens::from_spacing(&spacing),
            text: TextTokens::default(),
            text_field: TextFieldTokens::from_spacing(&spacing),
            checkbox: CheckboxTokens::default(),
            switch: SwitchTokens::default(),
            dialog: DialogTokens::from_spacing(&spacing),
            menu: MenuTokens::from_spacing(&spacing),
            card: CardTokens::from_spacing(&spacing),
        }
    }
    pub fn get(&self, id: ComponentId) -> ComponentTokens {
        match id {
            ComponentId::Button => ComponentTokens::Button(self.button.clone()),
            ComponentId::Text => ComponentTokens::Text(self.text.clone()),
            ComponentId::TextField => ComponentTokens::TextField(self.text_field.clone()),
            ComponentId::Checkbox => ComponentTokens::Checkbox(self.checkbox.clone()),
            ComponentId::Switch => ComponentTokens::Switch(self.switch.clone()),
            ComponentId::Dialog => ComponentTokens::Dialog(self.dialog.clone()),
            ComponentId::Menu => ComponentTokens::Menu(self.menu.clone()),
            ComponentId::Card => ComponentTokens::Card(self.card.clone()),
        }
    }
}
