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
    icon: IconTokens,
    divider: DividerTokens,
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
            icon: IconTokens::default(),
            divider: DividerTokens::default(),
        }
    }

    pub fn cupertino() -> Self {
        let spacing = SpacingTokens::default();
        Self {
            button: ButtonTokens {
                height: 44.0,
                padding_horizontal: spacing.md,
                gap: spacing.sm,
                icon_size: 20.0,
            },
            text: TextTokens::default(),
            text_field: TextFieldTokens {
                height: 44.0,
                padding_horizontal: spacing.md,
                border_radius: 10.0,
            },
            checkbox: CheckboxTokens {
                size: 20.0,
                border_radius: 6.0,
            },
            switch: SwitchTokens {
                width: 51.0,
                height: 31.0,
                thumb_size: 27.0,
            },
            dialog: DialogTokens {
                max_width: 420.0,
                padding: spacing.lg,
                border_radius: 14.0,
            },
            menu: MenuTokens {
                min_width: 160.0,
                item_height: 44.0,
                padding: spacing.sm,
            },
            card: CardTokens {
                elevation: 0,
                border_radius: 12.0,
                padding: spacing.md,
            },
            icon: IconTokens { size: 22.0 },
            divider: DividerTokens::default(),
        }
    }

    pub fn fluent() -> Self {
        let spacing = SpacingTokens::default();
        Self {
            button: ButtonTokens {
                height: 32.0,
                padding_horizontal: spacing.md,
                gap: spacing.sm,
                icon_size: 16.0,
            },
            text: TextTokens::default(),
            text_field: TextFieldTokens {
                height: 32.0,
                padding_horizontal: spacing.sm,
                border_radius: 4.0,
            },
            checkbox: CheckboxTokens {
                size: 20.0,
                border_radius: 4.0,
            },
            switch: SwitchTokens {
                width: 40.0,
                height: 20.0,
                thumb_size: 14.0,
            },
            dialog: DialogTokens {
                max_width: 540.0,
                padding: spacing.lg,
                border_radius: 8.0,
            },
            menu: MenuTokens {
                min_width: 120.0,
                item_height: 32.0,
                padding: spacing.xs,
            },
            card: CardTokens {
                elevation: 1,
                border_radius: 4.0,
                padding: spacing.md,
            },
            icon: IconTokens { size: 20.0 },
            divider: DividerTokens::default(),
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
            ComponentId::Icon => ComponentTokens::Icon(self.icon.clone()),
            ComponentId::Divider => ComponentTokens::Divider(self.divider.clone()),
        }
    }
}
