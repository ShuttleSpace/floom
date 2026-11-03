//! Fluent validation messages, ported to the Floom 2.0 widget API.

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

pub type SharedMessages = floom_widgets::SharedList;

fn shared_messages() -> SharedMessages {
    floom_widgets::shared_list(Vec::new())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentValidationMessagesStyle {
    pub design_system: &'static str,
    pub error_color: Color,
    pub font_size: f32,
    pub gap: f32,
    pub painted_font_size: f32,
    pub painted_gap: f32,
}

pub struct FluentValidationMessages {
    props: ValidationMessagesProps,
    theme: RwSignal<MultiTheme>,
    messages: SharedMessages,
}

impl FluentValidationMessages {
    pub fn new() -> Self {
        Self {
            props: ValidationMessagesProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            messages: shared_messages(),
        }
    }

    pub fn messages_list(&self) -> SharedMessages {
        self.messages.clone()
    }

    pub fn messages(mut self, messages: Vec<String>) -> Self {
        self.set_messages(messages);
        self
    }

    pub fn set_messages(&self, messages: Vec<String>) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = messages;
    }

    pub fn push_message(&self, message: impl Into<String>) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.push(message.into());
    }

    pub fn clear_messages(&self) {
        let mut guard = self
            .messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.clear();
    }

    pub fn current_messages(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.props.gap = gap;
        self
    }

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

    pub fn resolved_style(&self) -> FluentValidationMessagesStyle {
        let theme = self.theme.get();
        let ComponentTokens::ValidationMessages(tokens) =
            theme.component(ComponentId::ValidationMessages)
        else {
            unreachable!("ComponentId::ValidationMessages must resolve to ValidationMessagesTokens")
        };
        FluentValidationMessagesStyle {
            design_system: "fluent",
            error_color: theme.colors().error,
            font_size: tokens.font_size,
            gap: tokens.gap,
            painted_font_size: self.props.font_size,
            painted_gap: self.props.gap,
        }
    }

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

impl Default for FluentValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}
