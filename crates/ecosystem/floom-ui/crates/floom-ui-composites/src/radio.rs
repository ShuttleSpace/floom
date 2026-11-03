
use crate::{
    focus::FocusRing,
    state::{LayerShape, StateLayer},
};
use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    Color, CreateContext, ElementId, Event, EventContext, Key, KeyEvent, KeyPhase, NamedKey,
    PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget,
};
use floom_types::{FontWeight, Path, PointerButton, PointerPhase, Stroke};

use floom_widgets::{emit_semantics, resolved_color, InteractionState, Revision, SemanticsHint};

/// Gap between the ring's right edge and the start of the label.
const INDICATOR_LABEL_GAP: f32 = 6.0;

/// How a radio looks.
///
/// Colours are *resolved* rather than carried as an alpha over a theme colour,
/// the same rule [`crate::slider::SliderStyle`] and
/// [`crate::select::SelectStyle`] follow: a kit whose disabled radio is
/// `on_surface`@38 in one design system and `outline`@40 in another has two
/// different source colours, which no `alpha: u8` could carry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioStyle {
    // --- geometry ---
    /// The ring's diameter. The dot inside it is always half of this: all three
    /// v1 kits wrote `size * 0.5` and so did the engine's 25% inset, so it is
    /// derived rather than a field.
    pub size: f32,
    /// The invisible hit area around the ring, and the height the radio lays
    /// itself out to when that exceeds `size`.
    ///
    /// Material's is a flat 40; Cupertino's and Fluent's are `size + 8` so the
    /// ring keeps four points of padding.
    pub touch_target: f32,
    /// How thick the ring's rim is. `0.0` draws no rim and the ring is just its
    /// interior. `size / 2.0` fills the whole ring, which is a disc.
    pub border_width: f32,
    /// The inside of the ring, painted under the dot. `None` is transparent,
    /// which is what all three kits want: a radio shows its parent's background
    /// through the middle. The default fills it, because that is what the engine
    /// painted.
    pub interior_color: Option<Color>,

    // --- resolved colours ---
    /// The rim while selected. The kits use `primary`; the engine painted a
    /// `#9AA5B1` disc here.
    pub selected_color: Color,
    /// The rim while not selected. All three kits use `outline`.
    pub unselected_color: Color,
    /// The dot. All three kits use `primary`.
    pub selected_dot_color: Color,
    /// The rim while not selected and hovered, `None` for no variant.
    pub unselected_hovered_color: Option<Color>,
    /// The rim while not selected and pressed, `None` for no variant.
    pub unselected_pressed_color: Option<Color>,
    /// One colour for the whole disabled radio -- rim, dot and label.
    ///
    /// All three kits dim to `on_surface`@38. `None` does **not** mean "no dim":
    /// it falls through to the interaction resolution, whose first rule is a
    /// half-alpha tint, which is what the engine did. That is why the engine's
    /// default is `None` and still dims an unselected radio while leaving a
    /// selected one at full strength.
    pub disabled_color: Option<Color>,

    // --- label ---
    /// The gap between the ring's right edge and the label's left edge.
    ///
    /// All three v1 kits wrote `8.0`; the engine's shared
    /// `INDICATOR_LABEL_GAP` is `6.0`, so without this field the port would
    /// move every kit's label two points left with nothing to show for it.
    pub label_gap: f32,
    /// The label's font size. All three kits read it from `RadioTokens`.
    pub label_font_size: f32,
    /// The label's ink. All three kits use `on_surface`.
    pub label_color: Color,

    /// The state layer behind the ring.
    pub halo: StateLayer,

    /// The ring drawn around the whole control while focused.
    ///
    /// Cupertino's and Fluent's v1 radios both drew `outline(2.0)` in the theme's
    /// `primary` around the ring *and the label*, which is this. Material's drew
    /// none, because its halo already answers focus -- see [`RadioHalo::on_focus`]
    /// -- so Material leaves this `None` and the two mechanisms do not both fire.
    ///
    /// Around the control's whole bounds, not the indicator's: v1 put the
    /// `focus` closure on the container holding both, which is what a keyboard
    /// user needs to see.
    pub focus_ring: Option<FocusRing>,
}

/// The palette and geometry the engine hard-coded before a radio had a style.
///
/// Every value here is what `Radio::paint` used to write inline, so the default
/// is not a design decision: it is a snapshot of the old behaviour, kept so that
/// moving the widget did not also repaint every existing caller.
///
/// Two of the numbers read as odd until the rest is in place. `border_width` is
/// `size / 2.0`, which is what makes the "ring" the filled `#9AA5B1` disc the
/// engine drew, and `interior_color` is white, which is what that disc was
/// underneath it — both of them are restatements of one shape rather than
/// independent choices.
impl Default for RadioStyle {
    fn default() -> Self {
        RadioStyle {
            size: 16.0,
            touch_target: 16.0,
            border_width: 8.0,
            interior_color: Some(Color::WHITE),
            selected_color: Color::rgb(0x9A, 0xA5, 0xB1),
            unselected_color: Color::WHITE,
            selected_dot_color: Color::rgb(0x2B, 0x6C, 0xB0),
            // `floom_widgets::controls::resolved_color` takes `(base, pressed,
            // hovered)`, and the old inline call passed `#E2E8F0` second and
            // `#F5F7FA` third -- so hover was the *lighter* of the two, and
            // these two are in that order.
            unselected_hovered_color: Some(Color::rgb(0xF5, 0xF7, 0xFA)),
            unselected_pressed_color: Some(Color::rgb(0xE2, 0xE8, 0xF0)),
            disabled_color: None,
            label_gap: INDICATOR_LABEL_GAP,
            label_font_size: 14.0,
            label_color: Color::BLACK,
            halo: StateLayer::default(),
            // The engine drew no focus affordance of any kind, and the halo
            // defaults to answering none of the states.
            focus_ring: None,
        }
    }
}

/// One radio option. Grouping (mutual exclusion) is owned by the caller.
pub struct Radio {
    label: String,
    selected: bool,
    on_select: Option<Box<dyn Fn()>>,
    state: InteractionState,
    revision: Revision,
    /// Visual style. Named `style_` because the builder below is
    /// [`Radio::style`], the same split `Slider` uses.
    style_: RadioStyle,
}

impl Radio {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            selected: false,
            on_select: None,
            state: InteractionState::default(),
            revision: Revision::default(),
            style_: RadioStyle::default(),
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn on_select(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_select = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn is_selected(&self) -> bool {
        self.selected
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

    pub fn select(&mut self) {
        if self.selected {
            return;
        }
        self.selected = true;
        self.revision.bump();
        if let Some(callback) = &self.on_select {
            callback();
        }
    }

    /// Replaces the whole appearance. A kit resolves its `RadioTokens` into a
    /// [`RadioStyle`] once and hands it over here, rather than the widget
    /// holding a partial set of knobs a kit has to guess the rest of.
    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style_ = style;
        self.revision.bump();
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.selected {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::Radio, self.label.clone()).with_state(state)
    }

    /// The circular indicator bounds: [`RadioStyle::size`] across, centred
    /// vertically in the box the host gave us.
    ///
    /// The diameter is the style's own number and is *not* capped at the box's
    /// height. The old paint took `min(bounds.height(), 16.0)`, which could only
    /// ever return 16 — a hard-coded constant wearing a cap's clothes — so a kit
    /// whose ring is 20 or 24 across had no way to be drawn at all.
    pub fn dot_bounds(&self, bounds: Rect) -> Rect {
        let diameter = f64::from(self.style_.size);
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - diameter) / 2.0,
            x1: bounds.x0 + diameter,
            y1: bounds.y0 + (bounds.height() + diameter) / 2.0,
        }
    }

    /// A `diameter`-wide circle centred on `centre`.
    ///
    /// Drawn as a rounded rect at half the diameter: `fill_rounded_rect` is the
    /// one shape the paint context promises every backend, and at that radius it
    /// *is* the circle.
    fn circle_at(centre: Point, diameter: f64) -> Rect {
        let half = diameter / 2.0;
        Rect {
            x0: centre.x - half,
            y0: centre.y - half,
            x1: centre.x + half,
            y1: centre.y + half,
        }
    }

    /// Whether the halo's gates open for the state the radio is in.
    ///
    /// Every gate is a separate flag because the three kits disagree about which
    /// states draw one: a hover-only halo and a focus-answering halo are the same
    /// circle asked to appear at different moments, and there is no single
    /// boolean for "Material's" that is also true of Cupertino's.
    fn halo_shows(&self) -> bool {
        self.style_.halo.shows(
            self.state.hovered,
            self.state.pressed,
            self.state.focused,
            self.state.disabled,
        )
    }

    /// A colour the interaction variants do not reach, with the style's one
    /// disabled override still in front of it.
    ///
    /// [`RadioStyle::disabled_color`] is the only rule that covers a selected
    /// radio's layers and the label. With `None` there is nothing to apply, which
    /// is why a selected radio stays at full strength while disabled: the old
    /// `paint` branched on `selected` before it consulted the state at all, and
    /// a half-alpha rim and dot are not what it drew.
    fn unresolved(&self, color: Color) -> Color {
        match self.style_.disabled_color {
            Some(disabled) if self.state.disabled => disabled,
            _ => color,
        }
    }

    /// One visible layer of the **unselected** ring.
    ///
    /// The interaction variants replace *every* layer rather than the rim alone,
    /// which is what makes one pair of fields enough: a style that draws a rim
    /// sees its rim change, and one with `border_width == 0.0` — where there is
    /// no rim to see — sees its interior change instead. `None` falls back to
    /// this layer's own base colour.
    ///
    /// The call is `floom_widgets::resolved_color` rather than a second copy of
    /// its rules: that function already ranks disabled ahead of pressed ahead of
    /// hovered, and re-deriving the ranking here is how two widgets end up
    /// disagreeing about what a disabled control looks like. It is the same
    /// argument list the old inline `paint` passed, base white, pressed
    /// `#E2E8F0`, hovered `#F5F7FA`, which is where the two interaction fields
    /// above come from.
    fn unselected_layer(&self, base: Color) -> Color {
        match self.style_.disabled_color {
            Some(disabled) if self.state.disabled => disabled,
            _ => resolved_color(
                base,
                self.style_.unselected_pressed_color.unwrap_or(base),
                self.style_.unselected_hovered_color.unwrap_or(base),
                self.state,
            ),
        }
    }

    /// The rim's colour, whichever of the two rings is being drawn.
    fn rim_color(&self) -> Color {
        if self.selected {
            self.unresolved(self.style_.selected_color)
        } else {
            self.unselected_layer(self.style_.unselected_color)
        }
    }

    /// The interior's colour, or `None` when the ring is see-through.
    ///
    /// `interior_color` is an `Option` rather than a colour because a transparent
    /// middle is not a colour a theme has to name: all three kits leave it
    /// unpainted so the radio shows whatever its parent is painted with.
    fn interior_color(&self) -> Option<Color> {
        let base = self.style_.interior_color?;
        Some(if self.selected {
            self.unresolved(base)
        } else {
            self.unselected_layer(base)
        })
    }

    /// The dot's colour.
    fn dot_color(&self) -> Color {
        self.unresolved(self.style_.selected_dot_color)
    }

    /// The label's ink.
    ///
    /// `disabled_color` covers it when the style names one; otherwise it is the
    /// style's own ink, undimmed. The old `paint` never tinted its label, so
    /// halving it here would be a repaint of every existing caller rather than a
    /// move.
    fn label_color(&self) -> Color {
        self.unresolved(self.style_.label_color)
    }

    /// True when a key event should activate the focused radio.
    fn is_activation_key(&self, key: &KeyEvent) -> bool {
        if self.state.disabled || !self.state.focused || key.phase == KeyPhase::Up {
            return false;
        }
        matches!(key.key, Key::Named(NamedKey::Enter) | Key::Char(' '))
    }
}

impl std::fmt::Debug for Radio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Radio")
            .field("label", &self.label)
            .field("selected", &self.selected)
            .finish_non_exhaustive()
    }
}

impl Widget for Radio {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Radio::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: ring + gap + label extent (same geometry as
    /// [`Checkbox`](floom_widgets::Checkbox)).
    ///
    /// Both halves of this read the style. The height is
    /// [`RadioStyle::touch_target`] rather than a constant, because a kit whose
    /// ring carries four points of padding around a 20px ring reserves 28px
    /// there and would otherwise be laid out 16px tall with its ring clipped. The
    /// label is measured at [`RadioStyle::label_font_size`] rather than at the
    /// engine's 14, for the same reason: a kit's label is not the engine's label
    /// and measuring it at the wrong size reserves the wrong width.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        let side = self.style_.touch_target.max(self.style_.size);
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
        let Event::Pointer(pointer) = cx.event() else {
            return match cx.event() {
                Event::Key(key) if self.is_activation_key(&key) => {
                    self.select();
                    true
                }
                _ => false,
            };
        };
        if self.state.disabled {
            return false;
        }
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                self.state.hovered = true;
                true
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                self.state.hovered = false;
                self.state.pressed = false;
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
                    self.select();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Halo, then interior, then rim, then dot, then label.
    ///
    /// The order is load-bearing twice over. The halo goes first because it sits
    /// *behind* the ring, and the interior before the rim because a rim drawn
    /// first would be half-covered by the middle it is supposed to ring. The dot
    /// is last of the three ring layers because it is the only part of the radio
    /// that says which option this is.
    ///
    /// The rim is an annulus whose *outer* edge is the ring, so the stroked path
    /// sits half a `border_width` inside it rather than on it. That is what makes
    /// `border_width: size / 2.0` — the default — come out as the filled disc the
    /// engine drew: the stroke runs from radius 0 out to radius `size / 2`, with
    /// nothing left in the middle for the interior to show through.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style_;
        let ring = self.dot_bounds(bounds);
        let centre = ring.center();

        // First, so the halo and the rim both sit inside it. v1 put the focus
        // closure on the container that held the indicator *and the label*, so
        // the ring goes around the whole control's bounds.
        if let Some(focus_ring) = style.focus_ring {
            if self.state.focused {
                focus_ring.paint(cx, bounds, 0.0);
            }
        }

        let halo = self.style_.halo;
        if self.halo_shows() {
            // A radio's layer is always a circle; the `Box` arm is the
            // checkbox's, and a radio that wrote one would be a kit asking for a
            // shape this control does not draw.
            debug_assert!(
                matches!(halo.shape, LayerShape::Circle),
                "a radio's state layer is a circle; `Box` is the checkbox's"
            );
            let diameter = f64::from(halo.size);
            cx.fill_rounded_rect(
                Self::circle_at(centre, diameter),
                (diameter / 2.0) as f32,
                halo.ink(self.state.pressed),
            );
        }

        if let Some(interior) = self.interior_color() {
            cx.fill_rounded_rect(ring, (f64::from(style.size) / 2.0) as f32, interior);
        }

        let rim = self.rim_color();
        if style.border_width > 0.0 {
            let outer = f64::from(style.size) / 2.0;
            // A rim wider than the ring would stroke its own centreline outside
            // the box; capping the width at the diameter keeps the annulus
            // inside `ring` whatever a kit writes.
            let width = f64::from(style.border_width).min(outer * 2.0);
            cx.stroke_path(
                &Path::circle(centre, (outer - width / 2.0).max(0.0)),
                &Stroke::new(width, rim),
            );
        }

        if self.selected {
            let diameter = f64::from(style.size) * 0.5;
            cx.fill_rounded_rect(
                Self::circle_at(centre, diameter),
                (diameter / 2.0) as f32,
                self.dot_color(),
            );
        }

        cx.fill_text(
            Point::new(ring.x1 + f64::from(self.style_.label_gap), bounds.y0),
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
