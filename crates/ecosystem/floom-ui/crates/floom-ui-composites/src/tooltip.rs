//! `Tooltip` — a hint that appears over a control while the pointer rests on it.
//!
//! Theme-agnostic by design, like [`Badge`]: a 2.0 widget takes explicit values
//! rather than reading a global theme signal, so resolving the palette and the
//! token sizes is the *kit's* job.
//!
//! ## The shape of the thing
//!
//! A tooltip is three decisions and nothing else:
//!
//! * **what it says** — one line of text,
//! * **what it looks like** — a rounded box with the text in it,
//! * **where it appears** — under the control, a gap below it, as wide as the
//!   text and no wider.
//!
//! The third one is the reason this is a widget and not a box: the bubble has
//! to *measure* the text to know how wide it is, and the width is what makes it
//! a bubble rather than a bar.
//!
//! ## Where it lives
//!
//! The bubble is an **overlay**: it is asked of the host, positioned against
//! the control's own box, and laid over whatever is underneath rather than
//! pushing it around. That is the difference between a tooltip and a caption.
//!
//! Without a host to ask, the bubble falls back to sitting in the tree under
//! the control, where it is as wide as its text and moves the layout as it
//! appears. The fallback is the degraded path, and the host decides which one
//! an app gets.

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Child, ChildKey, Color, CreateContext, ElementId, Event, EventContext, LayoutRequest,
    OverlayKind, OverlayRequest, PaintContext, Rect, SemanticsContext, UpdateContext, Widget,
    WidgetAny,
};
use floom_types::{Insets, PointerPhase};

use floom_widgets::primitives::{Revision, SemanticsHint, emit_semantics};
use floom_widgets::{Surface, Text};

use crate::overlay_bookkeeping::OverlayHost;

/// The bubble's appearance.
///
/// The colours are the theme's business; the numbers are the tokens'.
///
/// [`Default`] is a plain dark bubble, which is what all three of the
/// design systems' tooltips are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipStyle {
    /// The bubble's fill.
    pub background: Color,
    /// The text's ink.
    pub foreground: Color,
    /// The bubble's padding, at each side.
    pub padding: f64,
    /// The bubble's corner radius.
    pub radius: f32,
    /// The text's size.
    pub font_size: f32,
    /// The gap between the control and the bubble.
    pub gap: f64,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            background: Color::rgb(0x1C, 0x1B, 0x1F),
            foreground: Color::rgb(0xEF, 0xE6, 0xF2),
            padding: 6.0,
            radius: 4.0,
            font_size: 12.0,
            gap: 0.0,
        }
    }
}

/// A hint that appears under a control while the pointer rests on it.
pub struct Tooltip {
    style: TooltipStyle,
    text: String,
    trigger: Option<Child>,
    surface: OverlayHost,
    /// Whether the pointer is over the control.
    hovered: bool,
    revision: Revision,
}

impl std::fmt::Debug for Tooltip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tooltip")
            .field("text", &self.text)
            .field("hovered", &self.hovered)
            .finish_non_exhaustive()
    }
}

impl Default for Tooltip {
    fn default() -> Self {
        Self::new()
    }
}

impl Tooltip {
    pub fn new() -> Self {
        Self {
            style: TooltipStyle::default(),
            text: String::new(),
            trigger: None,
            surface: OverlayHost::new(false),
            hovered: false,
            revision: Revision::default(),
        }
    }

    /// The control the pointer rests on.
    pub fn trigger(mut self, child: Child) -> Self {
        self.trigger = Some(child);
        self
    }

    /// What the tooltip says.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self
    }

    /// The bubble's appearance.
    pub fn style(mut self, style: TooltipStyle) -> Self {
        self.style = style;
        self
    }

    /// Whether the pointer is over the control.
    pub fn is_hovered(&self) -> bool {
        self.hovered
    }

    /// Whether the bubble is up.
    pub fn is_present(&self) -> bool {
        self.surface.is_shown()
    }

    /// The bubble's slot, for the path where there is no host to ask.
    ///
    /// The gap is a margin on the bubble's own slot, so it is the space
    /// *between* the control and the bubble rather than a space inside either.
    fn bubble_slot(&self) -> Child {
        let style = self.style;
        let text = self.text.clone();
        Child::new(
            ChildKey::index(1),
            LayoutRequest::new().margin(Insets {
                top: style.gap,
                ..Insets::ZERO
            }),
            move || bubble_widget(style, text.clone()),
        )
    }
}

/// The bubble: a rounded box with the text in it.
///
/// One builder for both paths, because the bubble the host lays out and the
/// bubble the tree lays out have to be the same box.
fn bubble_widget(style: TooltipStyle, text: String) -> Box<dyn WidgetAny> {
    Box::new(
        Surface::new()
            .background(style.background)
            .radius(style.radius)
            .padding(Insets {
                left: style.padding,
                right: style.padding,
                top: style.padding,
                bottom: style.padding,
            })
            .push(move || {
                Box::new(
                    Text::new(text.clone())
                        .size(style.font_size)
                        .color(style.foreground),
                )
            }),
    )
}

impl Widget for Tooltip {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // A hover that changed in an event has to reach this node, or the
        // bubble would not know to come up.
        self.revision.sync(cx, id);
        // The bubble is up exactly while the pointer is over the control.
        self.surface.set_present(self.hovered);
        let style = self.style;
        let text = self.text.clone();
        // The bubble is anchored to the control, so it takes the control's own
        // box as the rect it is placed against -- and the rect is *this wide*,
        // because the host lays the content out against the rect it is given
        // and a bubble as wide as the control would be a bar.
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            let measured = floom_text::measure_text(&text, style.font_size, f64::INFINITY);
            let width = measured.width as f64 + style.padding * 2.0;
            let height = measured.height as f64 + style.padding * 2.0;
            let rect = Rect {
                x0: bounds.x0,
                y0: bounds.y1 + style.gap,
                x1: bounds.x0 + width,
                y1: bounds.y1 + style.gap + height,
            };
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                false,
                rect,
                move |_handle| bubble_widget(style, text.clone()),
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        let mut out = Vec::with_capacity(2);
        if let Some(trigger) = &self.trigger {
            out.push(trigger.clone());
        }
        // With a host the bubble is in the host's layer, and this widget's own
        // box is the control's -- which is what the anchor needs. Without one,
        // the bubble sits in the tree, under the control.
        if !self.surface.takes_children_in_tree() && self.hovered {
            out.push(self.bubble_slot());
        }
        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // The control's box is this widget's box: the bubble is out of the flow
        // either way.
        Some(LayoutRequest::new().cross_align(Align::Start))
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                if !self.hovered {
                    self.hovered = true;
                    self.revision.bump();
                }
                true
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                if self.hovered {
                    self.hovered = false;
                    self.revision.bump();
                }
                true
            }
            _ => false,
        }
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl Tooltip {
    /// What this tooltip reports to assistive technology.
    ///
    /// A tooltip with no text has nothing to announce and stays decorative. One
    /// with text reports it while it is up -- and a tooltip that is *not* up is
    /// not there to be read.
    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.text.is_empty() || !self.surface.is_shown() {
            return SemanticsHint::decorative();
        }
        SemanticsHint::labeled(SemanticsRole::StaticText, self.text.clone())
    }
}
