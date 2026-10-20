//! Cupertino validation messages, ported to the Floom 2.0 widget API.
//!
//! A validation message list is a column of error strings under a field: the
//! theme's `error` colour, at a small size, with a gap between them.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a column of `Text` leaves, and a shared list the
//! caller pushes into and pulls out of.
//!
//! ## The engine gap
//!
//! **The engine has no shared list.** It has a shared bool (`SharedFlag`), a
//! shared string (`SharedBuffer`) and a shared map (`SharedMap`), and a list of
//! messages is none of those: a `SharedMap` keyed by index would lose the
//! order, and a `SharedBuffer` would have to carry a separator that a message
//! could contain.
//!
//! So the kit holds an `Arc<Mutex<Vec<String>>>` of its own, and that type is
//! the gap the engine should close. It is the same shape as the engine's
//! shared types -- an `Arc` over a `Mutex` -- so a caller that holds the
//! handle reads and writes it exactly as it would a `SharedFlag`.
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

use std::sync::{Arc, Mutex};

use floom_core::{
    Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, PaintContext, SemanticsContext,
    UpdateContext, Widget,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::Text;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::validation_messages::ValidationMessagesProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;

/// The shared list of messages.
///
/// The engine has no shared list -- see the module documentation -- so the kit
/// holds one of its own: the same shape as the engine's shared types, an
/// `Arc` over a `Mutex`.
pub type SharedMessages = Arc<Mutex<Vec<String>>>;

/// A fresh, empty list.
fn shared_messages() -> SharedMessages {
    Arc::new(Mutex::new(Vec::new()))
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
            // design system's theme, not the app-wide signal.
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
    pub fn view(self) -> CupertinoValidationMessagesList {
        let style = self.resolved_style();
        CupertinoValidationMessagesList {
            messages: self.messages,
            // The props, not the tokens: v1's paint used the props.
            font_size: style.painted_font_size,
            gap: style.painted_gap,
            error_color: to_paint(style.error_color),
            last: Vec::new(),
        }
    }
}

impl Default for CupertinoValidationMessages {
    fn default() -> Self {
        Self::new()
    }
}

/// The list of messages.
///
/// The engine has no shared list, so the kit holds one -- see the module
/// documentation.
pub struct CupertinoValidationMessagesList {
    messages: SharedMessages,
    font_size: f32,
    gap: f32,
    error_color: PaintColor,
    /// The messages as they were when this widget last looked, so a change is
    /// a change rather than a fresh read every frame.
    last: Vec<String>,
}

impl CupertinoValidationMessagesList {
    /// The messages as they are right now.
    pub fn current_messages(&self) -> Vec<String> {
        self.messages
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl Widget for CupertinoValidationMessagesList {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = self.current_messages();
        if now == self.last {
            return;
        }
        self.last = now;
        // The list changed, so the messages have to be re-declared. The host
        // visits every node each frame, so no notification has to travel.
        cx.mark_children_dirty(id);
    }

    fn children(&self) -> Vec<Child> {
        self.current_messages()
            .into_iter()
            .enumerate()
            .map(|(index, message)| {
                let color = self.error_color;
                let font_size = self.font_size;
                Child::new(
                    ChildKey::index(index),
                    LayoutRequest::new(),
                    move || Box::new(Text::new(message.clone()).size(font_size).color(color)),
                )
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Vertical, self.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        floom_widgets::emit_semantics(floom_widgets::SemanticsHint::decorative(), cx);
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}
