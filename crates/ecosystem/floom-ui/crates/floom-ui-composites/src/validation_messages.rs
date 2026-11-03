
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, LayoutRequest, PaintContext,
    SemanticsContext, UpdateContext, Widget, WidgetAny,
};
use floom_widgets::primitives::{emit_semantics, SemanticsHint};
use floom_widgets::{read_list, SharedList, Text};

/// The list's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ValidationMessagesStyle {
    /// The messages' ink.
    pub text_color: Color,
    /// The messages' size.
    pub font_size: f32,
    /// The gap between the messages.
    pub gap: f32,
}

impl Default for ValidationMessagesStyle {
    fn default() -> Self {
        Self {
            text_color: Color::rgb(0xB3, 0x26, 0x1E),
            font_size: 12.0,
            gap: 4.0,
        }
    }
}

/// A list of error messages, drawn from a shared list.
#[derive(Clone)]
pub struct ValidationMessagesList {
    style: ValidationMessagesStyle,
    messages: SharedList,
    /// The messages as they were when this widget last looked, so a change is
    /// a change rather than a fresh read every frame.
    last: Vec<String>,
}

impl std::fmt::Debug for ValidationMessagesList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidationMessagesList")
            .field("messages", &self.current_messages().len())
            .finish_non_exhaustive()
    }
}

impl Default for ValidationMessagesList {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationMessagesList {
    pub fn new() -> Self {
        Self {
            style: ValidationMessagesStyle::default(),
            messages: floom_widgets::shared_list(Vec::new()),
            last: Vec::new(),
        }
    }

    /// The list's appearance.
    pub fn style(mut self, style: ValidationMessagesStyle) -> Self {
        self.style = style;
        self
    }

    /// The shared list the messages travel in.
    pub fn messages(mut self, messages: SharedList) -> Self {
        self.messages = messages;
        self
    }

    /// The messages as they are right now.
    pub fn current_messages(&self) -> Vec<String> {
        read_list(&self.messages)
    }
}

impl Widget for ValidationMessagesList {
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
                let color = self.style.text_color;
                let font_size = self.style.font_size;
                Child::new(ChildKey::index(index), LayoutRequest::new(), move || {
                    Box::new(Text::new(message.clone()).size(font_size).color(color))
                })
            })
            .collect()
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(floom_core::Axis::Vertical, self.style.gap))
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(SemanticsHint::decorative(), cx);
    }
}

impl WidgetAny for ValidationMessagesList {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The list is a decoration; the messages inside it are text.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
