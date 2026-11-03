//! Cupertino validation messages, ported to the Floom 2.0 widget API.
//!

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

/// The shared list of messages.
///
/// The engine has no shared list -- see the module documentation -- so the kit
/// holds one of its own: the same shape as the engine's shared types, an
/// `Arc` over a `Mutex`.
/// The messages, shared with whoever writes them.
pub type SharedMessages = floom_widgets::SharedList;

/// A fresh, empty list.
fn shared_messages() -> SharedMessages {
    floom_widgets::shared_list(Vec::new())
}

/// The validation messages' appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoValidationMessagesStyle {
    pub design_system: &'static str,
    /// The messages' ink.
    pub error_color: Color,
    /// The messages' size, as the *token* has it. v1's style reported this
    /// number, and v1's paint used the props' -- see the module documentation.
    pub font_size: f32,
    /// The gap between the messages, as the *token* has it. Same caveat.
    pub gap: f32,
    /// The size the messages are actually drawn at: the props'.
    pub painted_font_size: f32,
    /// The gap the messages are actually drawn with: the props'.
    pub painted_gap: f32,
}

/// Cupertino's validation messages.
pub struct CupertinoValidationMessages {
    props: ValidationMessagesProps,
    theme: RwSignal<MultiTheme>,
    /// The messages. Shared with whoever pushes into it.
    messages: SharedMessages,
}

impl CupertinoValidationMessages {
    pub fn new() -> Self {
        Self {
            props: ValidationMessagesProps::default(),
            // v1's Cupertino validation messages read a snapshot of its own
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            messages: shared_messages(),
        }
    }

    /// The shared list of messages, so the consumer can push and pull errors
    /// (mirrors the `open_signal()` pattern from `Dialog`/`Menu`).
    ///
    /// v1's `messages_signal()` was an `RwSignal<Vec<String>>`; 2.0's shared
    /// state is a list behind an `Arc<Mutex<_>>`, because a value that changes
    /// after mount cannot be held in a signal.
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

    pub fn resolved_style(&self) -> CupertinoValidationMessagesStyle {
        let theme = self.theme.get();
        let ComponentTokens::ValidationMessages(tokens) =
            theme.component(ComponentId::ValidationMessages)
        else {
            unreachable!("ComponentId::ValidationMessages must resolve to ValidationMessagesTokens")
        };
        CupertinoValidationMessagesStyle {
            design_system: "cupertino",
            error_color: theme.colors().error,
            // v1's style: the token's numbers.
            font_size: tokens.font_size,
            gap: tokens.gap,
            // What v1's paint used: the props'.
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
                // The props, not the tokens: v1's paint used the props.
                font_size: style.painted_font_size,
                gap: style.painted_gap,
            })
            .messages(self.messages)
    }
}

impl Default for CupertinoValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}
