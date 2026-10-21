//! Fluent validation messages, ported to the Floom 2.0 widget API.
//!
//! A validation message list is a column of error strings under a field: the
//! theme's `error` colour, at a small size, with a gap between them.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a column of `Text` leaves, and a shared list the
//! caller pushes into and pulls out of.
//!
//! ## What the port uses
//!
//! The engine's [`ValidationMessagesList`]: a list of error messages, drawn
//! from a shared [`SharedList`]. The engine grew both for this -- it had a
//! shared bool, a shared string and a shared map, but no shared *list*, and a
//! list of messages is none of those: a map keyed by index would lose the
//! order, and a buffer would have to carry a separator that a message could
//! contain.
//!
//! The list re-declares its `Text` children whenever the messages change,
//! because 2.0's tree is built once.
//!
//! ## What this kit answers
//!
//! * the messages' ink: the theme's `error`;
//! * the messages' size and the gap between them: the props, which default to
//!   the tokens.
//!
//! ## The one disagreement inside v1
//!
//! v1's style reported the *tokens*, while v1's paint used the *props*: a
//! caller who wrote `gap(10.0)` changed the pixels and not the style. The port
//! reports both -- v1's fields keep v1's meaning (the token's numbers) and two
//! added fields report what the messages are actually drawn with -- because a
//! style that says nothing about the pixels is a trap. The tokens and the
//! props' defaults are the same numbers, so the two readings only differ after
//! an override.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`messages_signal()` was an `RwSignal<Vec<String>>`.** 2.0's shared state
//!   is a list behind an `Arc<Mutex<_>>`, so the kit hands out
//!   `messages_list()` and the caller pushes and pulls through it.
//! * **`dyn_view`.** v1 re-read the signal on every change and rebuilt the
//!   whole list. 2.0's widget reads the shared list in its `update` and
//!   re-declares its children -- the same thing, said the 2.0 way.
//!
//! ## The one thing v1's style did not have
//!
//! `design_system`. The other two kits' styles name theirs and this one's did
//! not; the port gives all three the same field, so a caller can read which
//! design system a style came from.

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
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

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
pub struct FluentValidationMessagesStyle {
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

/// Fluent's validation messages.
pub struct FluentValidationMessages {
    props: ValidationMessagesProps,
    theme: RwSignal<MultiTheme>,
    /// The messages. Shared with whoever pushes into it.
    messages: SharedMessages,
}

impl FluentValidationMessages {
    pub fn new() -> Self {
        Self {
            props: ValidationMessagesProps::default(),
            // v1's Fluent validation messages read a snapshot of its own
            // design system's theme, not the app-wide signal.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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

impl Default for FluentValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}
