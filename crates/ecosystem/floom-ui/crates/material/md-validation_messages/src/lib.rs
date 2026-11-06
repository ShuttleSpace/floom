use floom_core::{
    Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext, SemanticsContext,
    UpdateContext, Widget,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_ui_composites::{ValidationMessagesList, ValidationMessagesStyle};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::validation_messages::ValidationMessagesProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_runtime::theme_signal;

pub type SharedMessages = floom_widgets::SharedList;

/// A fresh, empty list.
fn shared_messages() -> SharedMessages {
    floom_widgets::shared_list(Vec::new())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialValidationMessagesStyle {
    pub design_system: &'static str,
    /// The messages' ink.
    pub error_color: Color,
    pub font_size: f32,
    /// The gap between the messages, as the *token* has it. Same caveat.
    pub gap: f32,
    /// The size the messages are actually drawn at: the props'.
    pub painted_font_size: f32,
    /// The gap the messages are actually drawn with: the props'.
    pub painted_gap: f32,
}

/// Material's validation messages.
pub struct MaterialValidationMessages {
    props: ValidationMessagesProps,
    theme: RwSignal<MultiTheme>,
    /// The messages. Shared with whoever pushes into it.
    messages: SharedMessages,
}

impl MaterialValidationMessages {
    pub fn new() -> Self {
        Self {
            props: ValidationMessagesProps::default(),
            theme: theme_signal(),
            messages: shared_messages(),
        }
    }

    pub fn messages_list(&self) -> SharedMessages {
        self.messages.clone()
    }

    /// Set a static list of messages.
    pub fn messages(mut self, messages: Vec<String>) -> Self {
        self.set_messages(messages);
        self
    }

    /// Replace the messages.
    pub fn set_messages(&self, messages: Vec<String>) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = messages;
    }

    /// Append a message.
    pub fn push_message(&self, message: impl Into<String>) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.push(message.into());
    }

    /// Take the messages away.
    pub fn clear_messages(&self) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.clear();
    }

    /// The messages as they are right now.
    pub fn current_messages(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Override the gap between messages.
    pub fn gap(mut self, gap: f32) -> Self {
        self.props.gap = gap;
        self
    }

    /// Override the per-message font size.
    pub fn font_size(mut self, font_size: f32) -> Self {
        self.props.font_size = font_size;
        self
    }

    pub fn props(&self) -> &ValidationMessagesProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> MaterialValidationMessagesStyle {
        let theme = self.theme.get();
        let ComponentTokens::ValidationMessages(tokens) =
            theme.component(ComponentId::ValidationMessages)
        else {
            unreachable!("ComponentId::ValidationMessages must resolve to ValidationMessagesTokens")
        };
        MaterialValidationMessagesStyle {
            design_system: "material",
            error_color: theme.colors().error,
            font_size: tokens.font_size,
            gap: tokens.gap,
            painted_font_size: self.props.font_size,
            painted_gap: self.props.gap,
        }
    }

    /// The list: the messages, in the theme's error colour, with the gap
    /// between them.
    pub fn view(self) -> ValidationMessagesList {
        let style = self.resolved_style();
        ValidationMessagesList::new()
            .style(ValidationMessagesStyle {
                text_color: to_paint(style.error_color),
                font_size: style.painted_font_size,
                gap: style.painted_gap,
            })
            .messages(self.messages)
    }
}

impl Default for MaterialValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}
