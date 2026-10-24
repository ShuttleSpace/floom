//! `Chip` — a pill: a rounded box with a label in the middle, and a click.
//!
//! Theme-agnostic by design, like [`Badge`]: a 2.0 widget takes explicit
//! values rather than reading a global theme signal, so resolving the palette
//! and the token sizes is the *kit's* job.
//!
//! ## Why this is a widget and not a box with a child
//!
//! v1's chip was a `Container` holding a `Label`, and the two things that made
//! it a chip were on the container: the label was **centred** in it
//! (`items_center`, `justify_center`) and the label's **ink** was the
//! container's own `color`. 2.0's `Surface` paints a fill and only the child
//! knows it is text, and a box's content sits at the padding's edge rather than
//! in the middle. So the pill and its centred label are one widget here, the
//! way they were one box in v1.
//!
//! ## Why the fill is painted here and not delegated
//!
//! Only one of the three kits changes a chip's fill while the pointer is over
//! it, and the change is a *fill* rather than a layer over one. A fill that
//! depends on the pointer cannot be delegated to a child box: the child is
//! built once, and the pointer state lives in the widget that answers the
//! pointer. So the chip holds the interaction state and paints the fill, the
//! way [`Radio`](crate::radio::Radio) paints its halo.

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    PaintContext, Point, SemanticsContext, UpdateContext, Widget,
};
use floom_types::{FontWeight, PointerButton, PointerPhase};

use floom_widgets::primitives::stroke_rounded_outline;
use floom_widgets::{emit_semantics, InteractionState, Revision, SemanticsHint};

/// A pill with a label in the middle.
pub struct Chip {
    /// The text in the middle.
    pub label: String,
    /// The pill's fill.
    pub background: Color,
    /// The fill while the pointer is over the chip.
    ///
    /// `None` means the resting fill does not change, which is what two of the
    /// three kits wanted.
    pub hover: Option<Color>,
    /// The label's ink.
    pub foreground: Color,
    /// The pill's height. The label is centred in it.
    pub height: f32,
    /// The space between the pill's edge and the label, at each side.
    pub padding_horizontal: f32,
    /// The corner radius. Half the height gives a pill.
    pub radius: f32,
    /// The label's size.
    pub font_size: f32,
    /// The label's weight.
    pub font_weight: FontWeight,
    /// The outline's thickness. `0.0` draws none.
    pub border_width: f32,
    /// The outline's colour.
    pub border_color: Color,
    /// Whether the chip answers the pointer.
    ///
    /// The *colours* of a disabled chip are the kit's business; this is only
    /// the gate on the click and the state reported to assistive technology.
    pub disabled: bool,
    on_click: Option<Box<dyn Fn()>>,
    state: InteractionState,
    revision: Revision,
}

impl Default for Chip {
    fn default() -> Self {
        Self {
            label: String::new(),
            background: Color::rgb(0xE7, 0xE0, 0xEC),
            hover: None,
            foreground: Color::rgb(0x1C, 0x1B, 0x1F),
            height: 32.0,
            padding_horizontal: 12.0,
            radius: 16.0,
            font_size: 14.0,
            font_weight: FontWeight::Normal,
            border_width: 0.0,
            border_color: Color::rgb(0x79, 0x79, 0x7E),
            disabled: false,
            on_click: None,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }
}

impl std::fmt::Debug for Chip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chip")
            .field("label", &self.label)
            .field("disabled", &self.disabled)
            .field("hovered", &self.state.hovered)
            .finish_non_exhaustive()
    }
}

impl Chip {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self.revision.bump();
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = color;
        self.revision.bump();
        self
    }

    /// The fill while the pointer is over the chip.
    pub fn hover(mut self, color: Option<Color>) -> Self {
        self.hover = color;
        self.revision.bump();
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = color;
        self.revision.bump();
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self.revision.bump();
        self
    }

    pub fn padding_horizontal(mut self, padding: f32) -> Self {
        self.padding_horizontal = padding;
        self.revision.bump();
        self
    }

    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self.revision.bump();
        self
    }

    pub fn font_weight(mut self, weight: FontWeight) -> Self {
        self.font_weight = weight;
        self.revision.bump();
        self
    }

    /// The outline drawn just inside the chip's edge.
    ///
    /// A width of `0.0` draws none, which is what a kit whose chip has no
    /// outline wants.
    pub fn border(mut self, width: f32, color: Color) -> Self {
        self.border_width = width;
        self.border_color = color;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    /// Called when the chip is clicked.
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    /// The interaction state, for a caller mirroring it.
    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    /// Whether the chip holds the keyboard focus.
    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// What this chip reports to assistive technology.
    ///
    /// A chip with no label has nothing to announce and stays decorative. One
    /// with a label reports it, as a button when it can be clicked and as
    /// static text when it cannot.
    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.label.is_empty() {
            return SemanticsHint::decorative();
        }
        let role = if self.on_click.is_some() && !self.disabled {
            SemanticsRole::Button
        } else {
            SemanticsRole::StaticText
        };
        let hint = SemanticsHint::labeled(role, self.label.clone());
        if self.disabled {
            hint.with_state(SemanticsState::DISABLED)
        } else {
            hint
        }
    }

    /// The label's measured size, at the chip's own size and weight.
    fn measured(&self) -> (f32, f32) {
        let measured = floom_text::measure_weighted_text(
            &self.label,
            self.font_size,
            self.font_weight,
            f64::INFINITY,
        );
        (measured.width as f32, measured.height as f32)
    }

    /// The extent this chip wants, given its own label.
    ///
    /// The width is the label plus the padding at each side; the height is the
    /// chip's own, so a chip is a chip whatever its label says.
    fn intrinsic(&self) -> (f32, f32) {
        let (text_width, _) = self.measured();
        (text_width + self.padding_horizontal * 2.0, self.height)
    }

    /// Whether the key is the one that activates the chip.
    fn is_activation_key(&self, key: &KeyEvent) -> bool {
        if self.disabled || !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        matches!(key.key, Key::Named(NamedKey::Enter) | Key::Char(' '))
    }

    /// The click, if the chip is allowed to answer it.
    fn activate(&mut self) -> bool {
        if self.disabled {
            return false;
        }
        if let Some(callback) = &self.on_click {
            callback();
        }
        self.revision.bump();
        true
    }
}

impl Widget for Chip {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Chip::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        let (width, height) = self.intrinsic();
        Some(floom_core::LayoutRequest::new().width(width).height(height))
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) if self.is_activation_key(&key) => self.activate(),
                _ => false,
            };
        };
        if self.disabled {
            return false;
        }
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                let changed = !self.state.hovered;
                self.state.hovered = true;
                if changed {
                    self.revision.bump();
                }
                true
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                let changed = self.state.hovered || self.state.pressed;
                self.state.hovered = false;
                self.state.pressed = false;
                if changed {
                    self.revision.bump();
                }
                true
            }
            PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
                self.state.hovered = true;
                self.state.pressed = true;
                cx.request_focus(id);
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                let activate = self.state.pressed;
                self.state.pressed = false;
                if activate {
                    self.activate()
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Fill, then outline, then label.
    ///
    /// The outline is a stroke just inside the edge, so it is drawn after the
    /// fill it sits on: a border drawn first would be half-covered by the fill
    /// it is supposed to ring.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let fill = match self.hover {
            Some(hover) if self.state.hovered => hover,
            _ => self.background,
        };
        if self.radius > 0.0 {
            cx.fill_rounded_rect(bounds, self.radius, fill);
        } else {
            cx.fill_rect(bounds, fill);
        }
        if self.border_width > 0.0 {
            stroke_rounded_outline(
                cx,
                bounds,
                f64::from(self.radius),
                f64::from(self.border_width),
                self.border_color,
            );
        }
        if self.label.is_empty() {
            return;
        }
        // The label is centred in the chip, the way v1's `items_center` and
        // `justify_center` put it in the middle of the container.
        let (text_width, text_height) = self.measured();
        let x = bounds.x0 + (bounds.width() - f64::from(text_width)) / 2.0;
        let y = bounds.y0 + (bounds.height() - f64::from(text_height)) / 2.0;
        cx.fill_text(
            Point::new(x, y),
            &self.label,
            self.font_size,
            self.font_weight,
            self.foreground,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}
