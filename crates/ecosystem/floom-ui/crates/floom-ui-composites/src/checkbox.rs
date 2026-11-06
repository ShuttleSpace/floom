use crate::{
    focus::FocusRing,
    state::{LayerShape, StateLayer},
};
use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget,
};
use floom_types::{FontWeight, PointerButton, PointerPhase};

use floom_widgets::{
    emit_semantics, paint_icon, stroke_rounded_outline, InteractionState, Revision, SemanticsHint,
    VectorIcon,
};

/// How a checkbox looks.
///
/// Colours are resolved per state rather than carried as an alpha over a theme
/// colour, for the same reason [`crate::radio::RadioStyle`]'s are: the unchecked
/// container is not a dimmed `primary`, it is fully transparent, and no
/// `alpha: u8` can say "nothing at all".
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckboxStyle {
    // --- geometry, from `CheckboxTokens` ---
    /// The indicator's side.
    pub size: f32,
    /// The indicator's corner radius.
    pub border_radius: f32,
    /// The rim's thickness. All three kits wrote `2.0`; the engine drew none.
    pub border_width: f32,
    /// Space between the indicator and the label. All three kits wrote `8.0`;
    /// the engine's is `6.0`.
    pub label_gap: f32,
    /// The label's ink size. All three kits read
    /// `typography().body.medium.font_size`.
    pub label_font_size: f32,

    // --- the indicator, resolved per state ---
    /// The indicator's fill while checked.
    pub container_color: Color,
    /// The indicator's fill while unchecked. Fully transparent in all three kits:
    /// a checkbox shows its parent's background, and a filled unchecked box reads
    /// as a third state.
    pub container_color_unchecked: Color,
    /// The indicator's fill while disabled. All three kits: `on_surface`@12.
    pub container_color_disabled: Color,
    /// The rim while checked. All three kits: `primary`.
    pub border_color: Color,
    /// The rim while unchecked. All three kits: `outline`.
    pub border_color_unchecked: Color,
    /// The rim while disabled. All three kits: `on_surface`@38.
    pub border_color_disabled: Color,
    /// The checkmark while checked. All three kits: `on_primary`.
    pub mark_color: Color,
    /// The checkmark while unchecked: transparent, so it is not drawn.
    pub mark_color_unchecked: Color,
    /// The checkmark while disabled. All three kits: `on_surface`@38.
    pub mark_color_disabled: Color,
    /// The checkmark's edge as a fraction of the indicator. All three kits
    /// wrote `0.7`.
    pub mark_scale: f32,

    // --- the label ---
    pub label_color: Color,
    pub label_color_disabled: Color,

    /// The layer behind the indicator. The checkbox's is a
    /// [`crate::state::LayerShape::Box`], not the radio's circle.
    pub layer: StateLayer,
    /// The ring around the whole control while focused, which two of the three
    /// kits draw and Material answers with its layer instead.
    pub focus_ring: Option<FocusRing>,
}

/// The palette and geometry the engine hard-coded before a checkbox had a style.
///
/// Every value here is what `Checkbox::paint` used to write inline, so the
/// default is not a design decision: it is a snapshot of the old behaviour, kept
/// so that moving the widget did not also repaint every existing caller.
///
/// Three of the entries read as odd until the rest is in place, and all three
/// are the same fact — the engine drew a bare fill and nothing else.
/// `border_width` is `0.0` and the three border colours are white, so the rim
/// has no thickness to show them at; `container_color_unchecked` is white where
/// every kit is transparent; and `layer` and `focus_ring` are both inert, which
/// is why an unstyled checkbox has no pointer feedback and no focus affordance
/// beyond what its own surface gives it.
///
/// The *disabled* column is the one place the default is not a literal. The old
/// paint resolved its fill through `floom_widgets::resolved_color`, whose first
/// rule halves the base's alpha, so a disabled checkbox painted
/// `#2B6CB0@127`; these four colours are undimmed, which is a deliberate
/// difference from the old pixels rather than an oversight. A style that wants
/// the old half-alpha rule has to name the dimmed colour itself.
impl Default for CheckboxStyle {
    fn default() -> Self {
        CheckboxStyle {
            size: 16.0,
            border_radius: 0.0,
            border_width: 0.0,
            label_gap: 6.0,
            label_font_size: 14.0,

            container_color: Color::rgb(0x2B, 0x6C, 0xB0),
            container_color_unchecked: Color::WHITE,
            container_color_disabled: Color::WHITE,
            border_color: Color::rgb(0x2B, 0x6C, 0xB0),
            border_color_unchecked: Color::WHITE,
            border_color_disabled: Color::WHITE,
            mark_color: Color::WHITE,
            mark_color_unchecked: Color::TRANSPARENT,
            mark_color_disabled: Color::WHITE,
            mark_scale: 1.0,

            label_color: Color::BLACK,
            label_color_disabled: Color::BLACK,

            layer: StateLayer::default(),
            focus_ring: None,
        }
    }
}

/// One boolean, and the indicator that says which value it is holding.
pub struct Checkbox {
    label: String,
    checked: bool,
    on_toggle: Option<Box<dyn Fn(bool)>>,
    state: InteractionState,
    revision: Revision,
    /// Visual style. Named `style_` because the builder below is
    /// [`Checkbox::style`], the same split `Radio` and `Slider` use.
    style_: CheckboxStyle,
    /// The checkmark to draw, when the caller supplied one.
    ///
    /// `None` is the engine's own character glyph, and it is the default because
    /// the engine's glyph is what an unstyled checkbox has always drawn — a kit
    /// passes the icon its own icon crate hands out and gets a vector mark.
    mark: Option<VectorIcon>,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            on_toggle: None,
            state: InteractionState::default(),
            revision: Revision::default(),
            style_: CheckboxStyle::default(),
            mark: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_toggle(mut self, callback: impl Fn(bool) + 'static) -> Self {
        self.on_toggle = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked
    }

    pub fn interaction(&self) -> InteractionState {
        self.state
    }

    pub fn set_focused(&mut self, focused: bool) -> &mut Self {
        if self.state.focused != focused {
            self.state.focused = focused;
            self.revision.bump();
        }
        self
    }

    /// Flips the value and notifies the change callback.
    pub fn toggle(&mut self) {
        self.checked = !self.checked;
        self.revision.bump();
        if let Some(callback) = &self.on_toggle {
            callback(self.checked);
        }
    }

    /// The indicator bounds: [`CheckboxStyle::size`] across, centred vertically
    /// in the box the host gave us.
    ///
    /// The side is the style's own number and is *not* capped at the box's
    /// height. The old paint took `min(bounds.height(), 16.0)`, which could only
    /// ever return 16 — a hard-coded constant wearing a cap's clothes — so a kit
    /// whose indicator is 20 or 24 across had no way to be drawn at all.
    pub fn box_bounds(&self, bounds: Rect) -> Rect {
        let side = f64::from(self.style_.size);
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - side) / 2.0,
            x1: bounds.x0 + side,
            y1: bounds.y0 + (bounds.height() + side) / 2.0,
        }
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.checked {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::CheckBox, self.label.clone()).with_state(state)
    }

    /// Replaces the whole appearance. A kit resolves its `CheckboxTokens` into a
    /// [`CheckboxStyle`] once and hands it over here, rather than the widget
    /// holding a partial set of knobs a kit has to guess the rest of.
    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style_ = style;
        self.revision.bump();
        self
    }

    /// The checkmark to draw instead of the default character glyph.
    ///
    /// A builder rather than a style field, for the same reason the cursor is not
    /// in a style: a mark is content the control draws, not a design system's
    /// opinion about it. `floom_widgets::VectorIcon` is already exported, and
    /// `VectorIcon::from_path_data` takes the path a kit's icon crate already
    /// hands out (`cupertino_icon_path("check")` and friends).
    pub fn mark(mut self, icon: VectorIcon) -> Self {
        self.mark = Some(icon);
        self.revision.bump();
        self
    }

    /// The indicator's fill for the state the checkbox is in.
    ///
    /// Disabled comes first and covers *both* checked and unchecked, which is
    /// what all three kits do — the `on_surface`@12 is written over the pair
    /// rather than beside it, so a disabled checked box is the dim colour and
    /// not the checked colour dimmed.
    fn container_color(&self) -> Color {
        if self.state.disabled {
            self.style_.container_color_disabled
        } else if self.checked {
            self.style_.container_color
        } else {
            self.style_.container_color_unchecked
        }
    }

    /// The rim's colour, for the same three-way resolution as the fill.
    fn border_color(&self) -> Color {
        if self.state.disabled {
            self.style_.border_color_disabled
        } else if self.checked {
            self.style_.border_color
        } else {
            self.style_.border_color_unchecked
        }
    }

    /// The checkmark's colour.
    ///
    /// `mark_color_unchecked` exists so a kit can name the mark's colour
    /// without having to suppress the draw; the control still only draws a mark
    /// while checked, because an unchecked mark is a third state rather than a
    /// faint one.
    fn mark_color(&self) -> Color {
        if self.state.disabled {
            self.style_.mark_color_disabled
        } else if self.checked {
            self.style_.mark_color
        } else {
            self.style_.mark_color_unchecked
        }
    }

    /// The label's ink.
    fn label_color(&self) -> Color {
        if self.state.disabled {
            self.style_.label_color_disabled
        } else {
            self.style_.label_color
        }
    }

    /// Whether the state layer's gates open for the state the checkbox is in.
    ///
    /// Four independent flags rather than one boolean, because the three kits
    /// disagree on every one: Material opens on hover, press *or focus* and
    /// keeps drawing while disabled; Cupertino and Fluent open on hover alone and
    /// suppress it entirely when disabled. Shared with `Radio` through
    /// [`StateLayer::shows`] rather than re-derived, which is the same argument
    /// the gate's own doc makes.
    fn layer_shows(&self) -> bool {
        self.style_.layer.shows(
            self.state.hovered,
            self.state.pressed,
            self.state.focused,
            self.state.disabled,
        )
    }

    /// A `side`-wide box centred on `centre`.
    fn centred(centre: Point, side: f64) -> Rect {
        let half = side / 2.0;
        Rect {
            x0: centre.x - half,
            y0: centre.y - half,
            x1: centre.x + half,
            y1: centre.y + half,
        }
    }

    /// True when a key event should activate the focused checkbox.
    fn is_activation_key(&self, key: &KeyEvent) -> bool {
        if self.state.disabled || !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        matches!(key.key, Key::Named(NamedKey::Enter) | Key::Char(' '))
    }
}

impl std::fmt::Debug for Checkbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Checkbox")
            .field("label", &self.label)
            .field("checked", &self.checked)
            .finish_non_exhaustive()
    }
}

impl Widget for Checkbox {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Checkbox::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: indicator + gap + label extent (same geometry as
    /// [`Radio`](crate::radio::Radio)).
    ///
    /// Both halves read the style. The indicator is
    /// [`CheckboxStyle::size`] rather than the engine's constant, because a kit
    /// whose `CheckboxTokens::size` is 20 would otherwise be laid out 16 and
    /// clipped; and the label is measured at [`CheckboxStyle::label_font_size`]
    /// rather than at the engine's 14, for the same reason — a kit's label is not
    /// the engine's label, and measuring it at the wrong size reserves the wrong
    /// width.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        let side = self.style_.size;
        let label_width = floom_widgets::text_width_at(&self.label, self.style_.label_font_size);
        let label_height = floom_text::measure_text(
            &self.label,
            self.style_.label_font_size,
            f64::INFINITY,
        )
        .height as f32;
        Some(
            floom_core::LayoutRequest::new()
                .width(side + self.style_.label_gap + label_width)
                .height(side.max(label_height)),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id) {
            Press::Activate => {
                self.toggle();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event() {
                    if self.is_activation_key(&key) {
                        self.toggle();
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Focus ring, then layer, then fill, then rim, then mark, then label.
    ///
    /// The order is load-bearing three times over. The ring goes first because
    /// it is drawn *outside* the control's box and everything else would sit on
    /// top of it; the layer goes next because it sits behind the indicator; and
    /// the rim after the fill because a stroke is drawn over what it outlines —
    /// which is also the order v1 used, `background(bg).border(2.0)`.
    ///
    /// The rim's outer edge is the indicator's own edge, so the stroked path sits
    /// half a `border_width` inside it rather than on it.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style_;
        let box_rect = self.box_bounds(bounds);
        let centre = box_rect.center();

        // First, so the indicator and the label both sit inside it. v1 put the
        // focus closure on the container that held the indicator *and the
        // label*, so the ring goes around the whole control's bounds.
        if let Some(focus_ring) = style.focus_ring {
            if self.state.focused {
                focus_ring.paint(cx, bounds, style.border_radius);
            }
        }

        let layer = self.style_.layer;
        if self.layer_shows() {
            let side = f64::from(layer.size);
            // Both arms are drawn rather than one being asserted against. A
            // checkbox's layer is a `Box` in all three kits, but `LayerShape`'s
            // `Default` is `Circle`, and a style that only sets `on_hover` would
            // trip an assertion in a path no test reaches.
            let radius = match layer.shape {
                LayerShape::Circle => (side / 2.0) as f32,
                // v1 inset the control's own radius by `padding` before growing
                // the box, which is the same as growing the box and adding
                // `padding` to the radius: both give `size + 2 * padding` across
                // with four points of corner.
                LayerShape::Box { padding } => f64::from(style.border_radius + padding) as f32,
            };
            cx.fill_rounded_rect(
                Self::centred(centre, side),
                radius,
                layer.ink(self.state.pressed),
            );
        }

        cx.fill_rounded_rect(box_rect, style.border_radius, self.container_color());

        if style.border_width > 0.0 {
            stroke_rounded_outline(
                cx,
                box_rect,
                f64::from(style.border_radius),
                f64::from(style.border_width),
                self.border_color(),
            );
        }

        if self.checked {
            let ink = self.mark_color();
            match &self.mark {
                Some(icon) => {
                    let side = f64::from(style.size * style.mark_scale);
                    paint_icon(cx, icon, Self::centred(centre, side), ink);
                }
                None => {
                    // The engine's own glyph, at the engine's own size and the
                    // engine's own two-and-one nudge from the box's corner. The
                    // size is the *box's* height rather than `size *
                    // mark_scale`: this path is the one that predates the style,
                    // and it is what the default table promises.
                    let mut buf = [0u8; 4];
                    let check = '\u{2714}'.encode_utf8(&mut buf);
                    let origin = Point::new(box_rect.x0 + 2.0, box_rect.y0 + 1.0);
                    cx.fill_text(
                        origin,
                        check,
                        box_rect.height() as f32,
                        FontWeight::Normal,
                        ink,
                    );
                }
            }
        }

        cx.fill_text(
            Point::new(box_rect.x1 + f64::from(style.label_gap), bounds.y0),
            &self.label,
            style.label_font_size,
            FontWeight::Normal,
            self.label_color(),
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }

    fn cursor(&self) -> floom_types::CursorStyle {
        // A disabled control gets the platform arrow: a pointer over a
        // thing that cannot be clicked is a promise the control cannot keep.
        if self.state.disabled {
            floom_types::CursorStyle::Default
        } else {
            floom_types::CursorStyle::Pointer
        }
    }
}

// ---------------------------------------------------------------------------
// The engine's pointer tracking, carried across verbatim.
//
// `floom_widgets`' `track_pointer` is `pub(crate)` — the widgets crate has no
// reason to publish a three-way answer its own callers all understand — so these
// are the same table written out again. `Radio` and `TextField` did the same
// thing, each with the note of what its own call site needed; this call site
// needs all three outcomes, so it is the original rather than either reduction.
// ---------------------------------------------------------------------------

/// Outcome of feeding a pointer event into a control.
///
/// `Consumed` and `Activate` both stop the event from bubbling; only
/// `Activate` triggers the control's action, and only on the release that
/// completes a press begun inside the control.
enum Press {
    Ignored,
    Consumed,
    Activate,
}

/// Applies a pointer event to `state`, reporting how far it got.
fn track_pointer(state: &mut InteractionState, cx: &mut dyn EventContext, id: ElementId) -> Press {
    let Event::Pointer(pointer) = cx.event() else {
        return Press::Ignored;
    };
    if state.disabled {
        return Press::Ignored;
    }
    match pointer.phase {
        PointerPhase::Enter | PointerPhase::Move => {
            state.hovered = true;
            Press::Consumed
        }
        PointerPhase::Leave | PointerPhase::Cancel => {
            state.hovered = false;
            state.pressed = false;
            Press::Consumed
        }
        PointerPhase::Down if pointer.button == Some(PointerButton::Primary) => {
            state.hovered = true;
            state.pressed = true;
            cx.request_focus(id);
            Press::Consumed
        }
        PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
            let activate = state.pressed;
            state.pressed = false;
            if activate {
                Press::Activate
            } else {
                Press::Ignored
            }
        }
        _ => Press::Ignored,
    }
}
