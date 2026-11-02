//! Tier-1 interactive controls.
//!
//! Every control carries an [`InteractionState`] (hover / press / focus /
//! disable) and reports it through [`SemanticsHint::state`], so assistive
//! technology sees the same state the control paints. Mutation goes through
//! builders that bump the internal revision, keeping a single invalidation
//! model across all widgets.
//!
//! [`Button`] and friends draw their own content; [`Pressable`] is the one that
//! does not — it makes an arbitrary subtree act as a single clickable surface,
//! which is what a clickable subtree is for.

use std::sync::Arc;

use floom_accessibility::{SemanticsRole, SemanticsState};
use floom_core::{
    AnimationSpec, Brush, Child, Color, CreateContext, Easing, ElementId, Event, EventContext, Key,
    KeyEvent, KeyPhase, NamedKey, PaintContext, PaintStyle, Point, Rect, RepeatSpec,
    SemanticsContext, Transition, UpdateContext, Widget,
};
use floom_types::{FontWeight, PointerButton, PointerPhase};

use crate::primitives::{Revision, SemanticsHint};
use crate::vector_icon::VectorIcon;

/// Font size every tier-1 control paints its label with. Shared between
/// `paint` and `layout_request` so the intrinsic size always matches what is
/// drawn.
const LABEL_FONT_SIZE: f32 = 14.0;
/// Horizontal padding between a Button's edge and its label (each side).
const BUTTON_PADDING: f32 = 8.0;
/// Vertical padding around a Button's label (each side).
const BUTTON_VERTICAL_PADDING: f32 = 4.0;
/// Gap between a Switch's track and its label.
///
/// The checkbox that used to share this constant now measures its own gap from
/// `CheckboxStyle::label_gap`, because all three kits wrote `8.0` and the
/// engine's `6.0` had to survive as the default.
const INDICATOR_LABEL_GAP: f32 = 6.0;
/// Switch track width; its height is half of this.
const SWITCH_TRACK_WIDTH: f32 = 40.0;
/// Progress bar height: the bar paints its whole box, so its intrinsic size
/// is a convention rather than a measurement.
const PROGRESS_HEIGHT: f32 = 8.0;
/// Default Progress bar length when no explicit width is given.
const PROGRESS_WIDTH: f32 = 160.0;

/// Intrinsic width of a label at the controls' shared font size.
///
/// Empty labels measure to zero, which keeps "no label" controls auto-sized
/// instead of requesting a phantom extent.
fn label_width(label: &str) -> f32 {
    if label.is_empty() {
        0.0
    } else {
        floom_text::measure_text(label, LABEL_FONT_SIZE, f64::INFINITY).width as f32
    }
}

/// Intrinsic height of a label at the controls' shared font size.
fn label_height(label: &str) -> f32 {
    if label.is_empty() {
        0.0
    } else {
        floom_text::measure_text(label, LABEL_FONT_SIZE, f64::INFINITY).height as f32
    }
}

/// Intrinsic width of a string at an arbitrary size.
///
/// `label_width` measures at the controls' shared size; a text field takes its size
/// from a token, so it needs the same measurement at *its* size. Both go through
/// `floom_text::measure_text`, so there is still one estimator in the crate.
///
/// Public because widgets above this crate size their own labels: `Select`
/// measures its options at `SelectStyle::text_size`, a size the engine does not
/// choose. Re-deriving the empty-string rule at each call site is how an
/// auto-sized control ends up requesting a phantom extent.
pub fn text_width_at(label: &str, size: f32) -> f32 {
    if label.is_empty() {
        0.0
    } else {
        floom_text::measure_text(label, size, f64::INFINITY).width as f32
    }
}

/// The height one line occupies at an arbitrary size.
pub(crate) fn text_height_at(size: f32) -> f32 {
    floom_text::measure_text(" ", size, f64::INFINITY).height as f32
}

/// Where the baseline sits within one line, at an arbitrary size.
///
/// Derived from a measured reference string rather than a guessed ratio, so a caret
/// and a first line agree at every size — and `text_ascent_at` exists rather than a
/// formula so that a future real measurement replaces one place.
pub(crate) fn text_ascent_at(size: f32) -> f32 {
    floom_text::measure_text("Hg", size, f64::INFINITY).height as f32 * 0.8
}

/// Shared hover/press/focus/disable state for interactive controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InteractionState {
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub disabled: bool,
}

impl InteractionState {
    /// The subset that reaches assistive technology.
    pub fn to_semantics(self) -> SemanticsState {
        self.to_semantics_with(true)
    }

    /// [`InteractionState::to_semantics`], for a control that is not focusable.
    ///
    /// A non-focusable control is still enabled and still clickable, so it keeps
    /// `ENABLED`; it just does not advertise `FOCUSABLE`, because no key press
    /// can reach it. Advertising it anyway would put a dead stop in a screen
    /// reader's tab order.
    pub fn to_semantics_with(self, focusable: bool) -> SemanticsState {
        let mut state = SemanticsState::empty();
        if self.disabled {
            state |= SemanticsState::DISABLED;
        } else {
            state |= SemanticsState::ENABLED;
            if focusable {
                state |= SemanticsState::FOCUSABLE;
            }
        }
        if self.focused {
            state |= SemanticsState::FOCUSED;
        }
        state
    }

    pub fn set_hovered(&mut self, hovered: bool) -> bool {
        let changed = self.hovered != hovered;
        self.hovered = hovered;
        changed
    }
}

// --- colour helpers -------------------------------------------------------

/// Linear blend from `a` to `b`.
fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t).round() as u8;
    Color::rgba(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), ch(a.a, b.a))
}

/// Dims a colour toward transparency to render the disabled appearance.
///
/// Public because it is a convention rather than a private detail: halving the
/// alpha is what "disabled" means to every control in this crate, so a widget
/// that resolves its own palette — a `Radio` in `floom-ui-composites`, whose
/// style leaves the disabled colour as `None` — has to dim the same way. Copying
/// the rule is how two widgets end up disagreeing about what a disabled control
/// looks like, which is the exact failure `RadioStyle::disabled_color` exists to
/// let a kit opt out of.
pub fn disabled_tint(c: Color) -> Color {
    Color::rgba(c.r, c.g, c.b, c.a / 2)
}

/// Resolves the base colour through the interaction state.
///
/// The ranking is the other half of the convention: disabled outranks pressed,
/// which outranks hovered, and a disabled control dims its *base* colour
/// however the pointer is behaving. `PointerPhase::Down` sets `hovered` and
/// `pressed` together, so a resolution that ranked hover higher would let a
/// press paint its hover colour and never its own.
///
/// Public for the reason [`disabled_tint`] is: `floom-ui-composites`' `Radio`
/// resolves two optional interaction variants through this rather than
/// re-deriving the order, so that a style which names no variant falls back to
/// exactly what an unstyled control in this crate paints.
pub fn resolved_color(
    base: Color,
    pressed: Color,
    hovered: Color,
    state: InteractionState,
) -> Color {
    if state.disabled {
        return disabled_tint(base);
    }
    if state.pressed {
        return pressed;
    }
    if state.hovered {
        return hovered;
    }
    base
}

/// Darker/lighter variants of a base control colour.
fn emphasis(c: Color) -> (Color, Color) {
    (mix(c, Color::BLACK, 0.18), mix(c, Color::WHITE, 0.18))
}

// --- shared pointer tracking ---------------------------------------------

/// Outcome of feeding a pointer event into a control.
///
/// `Consumed` and `Activate` both stop the event from bubbling; only
/// `Activate` triggers the control's action, and only on the release that
/// completes a press begun inside the control.
pub(crate) enum Press {
    Ignored,
    Consumed,
    Activate,
}

/// Applies a pointer event to `state`, reporting how far it got.
///
/// `focusable` is the control's own answer to "should a press move the keyboard
/// here". A press is still consumed when it is `false` — the control answers the
/// pointer, it just does not take the keyboard — which is what a tool-chrome
/// button (a devtools tab, say) wants: clicking it must not steal focus from the
/// app underneath.
pub(crate) fn track_pointer(
    state: &mut InteractionState,
    cx: &mut dyn EventContext,
    id: ElementId,
    focusable: bool,
) -> Press {
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
            if focusable {
                cx.request_focus(id);
            }
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

/// True when a key event should activate the focused control.
fn is_activation_key(key: &KeyEvent, state: InteractionState) -> bool {
    if state.disabled || !state.focused || key.phase == KeyPhase::Up {
        return false;
    }
    matches!(key.key, Key::Named(NamedKey::Enter) | Key::Char(' '))
}

/// Paints `icon` inside `bounds`, tinted `color`.
///
/// A composite cannot hand a [`VectorIcon`] its own paint context, so it needs
/// this: the icon's path is in its view box's coordinates, and the mapping onto
/// an arbitrary box is [`VectorIcon::transform_for`] — the same one the icon
/// uses for itself, so a chevron drawn by a Select and one drawn on its own are
/// identical. An empty path (an icon name with no glyph behind it) paints
/// nothing rather than a degenerate path.
/// Fills `icon` scaled into `bounds`, skipping the two cases that have no paint.
///
/// Public for the same reason [`text_width_at`] is: a widget above this crate
/// draws a supplied icon — `Select`'s chevron is a kit's glyph, chosen by the
/// kit — and the guards are the whole value here. An icon with no path, or one
/// that cannot be scaled into the box, has to paint nothing; re-deriving that
/// per call site is how an absent glyph turns into a fill of the origin.
pub fn paint_icon(cx: &mut dyn PaintContext, icon: &VectorIcon, bounds: Rect, color: Color) {
    if icon.path().is_empty() {
        return;
    }
    let Some(transform) = icon.transform_for(bounds) else {
        return;
    };
    cx.fill_path(&icon.path().transform(transform), color);
}

/// Draws `text` left-aligned and vertically centred inside `bounds`.
fn draw_centered_text(
    cx: &mut dyn PaintContext,
    bounds: Rect,
    text: &str,
    size: f32,
    color: Color,
) {
    if text.is_empty() || size <= 0.0 {
        return;
    }
    let origin = Point::new(
        bounds.x0 + f64::from(BUTTON_PADDING),
        bounds.y0 + (bounds.height() - f64::from(size)) / 2.0,
    );
    cx.fill_text(origin, text, size, FontWeight::Normal, color);
}

// ---------------------------------------------------------------------------
// Button
// ---------------------------------------------------------------------------

/// A UI callback.
///
/// Deliberately *not* `Send + Sync`: input dispatch happens on one thread, and
/// keeping the bound off lets callbacks capture `Rc`/`Cell` handles. Widgets
/// that carry callbacks can still live inside composites — construct the
/// widget inside the builder closure (builders are replayable `Fn`s) rather
/// than capturing a pre-built one.
pub type Callback = Box<dyn Fn()>;

/// A clickable action surface.
pub struct Button {
    label: String,
    on_click: Option<Callback>,
    base: Color,
    text_color: Color,
    radius: f32,
    /// The fill to use while the pointer is over the button, when a kit names one.
    ///
    /// `None` (the default) derives the hover fill from the base colour, which is
    /// right for a filled button. A kit whose base fill is *transparent* -- a ghost
    /// or a text button -- has no colour to derive from, so it names the hover
    /// fill itself rather than getting an invisible one.
    hover: Option<Color>,
    /// The fill to use while the button is pressed. `None` derives it the same way
    /// `hover` does.
    press: Option<Color>,
    state: InteractionState,
    /// Whether a press moves the keyboard here.
    ///
    /// `true` for a normal button: pressing it focuses it, so Enter reaches the
    /// same control the pointer just touched. Tool chrome — the devtools tab bar
    /// is the one this exists for — turns it off, because a click on the chrome
    /// must not take the keyboard away from the app underneath.
    focusable: bool,
    revision: Revision,
    /// The implicit colour transition, when set: hovering / pressing fades the
    /// fill between its old and new colour instead of snapping. `None` (the
    /// default) keeps the pre-transition instant switch.
    transition: Option<Transition>,
    /// The fill colour the current transition *started from* — the value the
    /// resolved colour had when the last transition was seeded. Interpolating
    /// from here (rather than always from `base`) is what makes a mid-flight
    /// reversal start from where the colour is, not where it was born.
    transition_from: Color,
    /// Bumped every time the resolved fill colour changes while a transition is
    /// armed. Feeds [`Button::animation`]'s declaration key, so the host sees a
    /// *new* declaration and replays the tween instead of leaving the finished
    /// one parked.
    transition_rev: u64,
}

impl std::fmt::Debug for Button {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Button")
            .field("label", &self.label)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_click: None,
            base: Color::rgb(0x2B, 0x6C, 0xB0),
            text_color: Color::WHITE,
            radius: 4.0,
            hover: None,
            press: None,
            state: InteractionState::default(),
            focusable: true,
            revision: Revision::default(),
            transition: None,
            transition_from: Color::rgb(0x2B, 0x6C, 0xB0),
            transition_rev: 0,
        }
    }

    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    pub fn color(mut self, base: Color) -> Self {
        self.base = base;
        self.revision.bump();
        self
    }

    /// The colour of the label.
    ///
    /// A button *is* its label -- it paints a run of text and nothing else -- so
    /// the label's colour is a property of the button rather than of a child it
    /// holds. Without this the label is always white, which is only right for a
    /// button whose fill happens to be dark.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = color;
        self.revision.bump();
        self
    }

    /// The corner radius of the button's fill.
    ///
    /// `0.0` paints a square-cornered button; anything above rounds the corners
    /// by that many pixels. A pill is a radius at least half the button's height.
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self.revision.bump();
        self
    }

    /// The fill to use while the pointer is over the button.
    ///
    /// Without one, the hover fill is derived from the base colour (darker on
    /// press, lighter on hover). A kit whose base fill is transparent names the
    /// hover fill here instead, so the button still answers the pointer.
    ///
    /// The named colour *replaces* the base fill, it is not blended with it --
    /// which is what a design system that specifies a hover fill means.
    pub fn hover_color(mut self, color: Color) -> Self {
        self.hover = Some(color);
        self.revision.bump();
        self
    }

    /// The fill to use while the button is pressed. See [`Button::hover_color`].
    pub fn press_color(mut self, color: Color) -> Self {
        self.press = Some(color);
        self.revision.bump();
        self
    }

    /// Arms the implicit fill-colour transition: hovering or pressing this
    /// button fades the fill between its old and new colour over `transition`
    /// instead of switching instantly.
    ///
    /// The button has no clock of its own — it declares the transition's shape
    /// (see [`Button::animation`]) and reads the host's sampled progress back
    /// in [`Button::paint`], interpolating the fill colour between where it
    /// was and where it should be. Reversing the hover mid-flight starts the
    /// fade again from the colour at that moment, not from the resting colour.
    pub fn transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
    }

    /// Whether a press moves the keyboard here. Defaults to `true`.
    ///
    /// Turn it off for chrome: a devtools tab still answers the pointer and
    /// still runs its callback, but clicking it leaves focus where the user had
    /// it, so the app underneath keeps receiving keys. Such a button is also
    /// reported to assistive tech as not focusable, so a screen reader does not
    /// offer a stop the keyboard cannot reach.
    pub fn focusable(mut self, focusable: bool) -> Self {
        self.focusable = focusable;
        self.revision.bump();
        self
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    /// The fill colour the current interaction state resolves to, absent any
    /// transition — the value the fill is *heading toward*.
    fn resolved_fill(&self) -> Color {
        let (pressed, hovered) = self.emphasis_colors();
        resolved_color(self.base, pressed, hovered, self.state)
    }

    /// The pressed and hovered fills: the ones a kit named, or the base colour's
    /// own emphasis when it named none.
    fn emphasis_colors(&self) -> (Color, Color) {
        let (darker, lighter) = emphasis(self.base);
        (self.press.unwrap_or(darker), self.hover.unwrap_or(lighter))
    }

    /// Seeds the implicit transition when the pointer changed the state such
    /// that the resolved fill colour moved. `before` is the interaction state
    /// prior to the event that just ran.
    ///
    /// Called from [`Button::event`] only, and only when a transition is
    /// armed. When the target colour genuinely changed, the old target becomes
    /// the new `transition_from` (so a mid-flight reversal starts from the
    /// colour at that moment) and the declaration key bumps, which is what the
    /// host reads as "restart the tween".
    fn arm_transition(&mut self, before: InteractionState) {
        if self.transition.is_none() {
            return;
        }
        let (pressed, hovered) = self.emphasis_colors();
        let old_target = resolved_color(self.base, pressed, hovered, before);
        let new_target = resolved_color(self.base, pressed, hovered, self.state);
        if old_target == new_target {
            return;
        }
        self.transition_from = old_target;
        self.transition_rev = self.transition_rev.wrapping_add(1);
        self.revision.bump();
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

    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(SemanticsRole::Button, self.label.clone())
            .with_state(self.state.to_semantics_with(self.focusable))
    }
}

impl Widget for Button {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Button::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the label's measured extent plus the padding paint
    /// applies. The host treats this as default sizing — an explicit mount
    /// style still pins the button, while unpinned ones shrink-wrap.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        Some(
            floom_core::LayoutRequest::new()
                .width(label_width(&self.label) + BUTTON_PADDING * 2.0)
                .height(label_height(&self.label) + BUTTON_VERTICAL_PADDING * 2.0),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let before = self.state;
        let press = track_pointer(&mut self.state, cx, id, self.focusable);
        // Armed once for *every* path rather than inside one branch: releasing a
        // press is `Press::Activate`, and it clears `pressed` — which moves the
        // resolved colour just as leaving does — so arming only on `Consumed`
        // makes letting go snap while everything else fades. `arm_transition`
        // itself compares the old and new target colours, so a path that moved
        // no colour costs nothing.
        self.arm_transition(before);
        match press {
            Press::Activate => {
                if let Some(callback) = &self.on_click {
                    callback();
                }
                self.revision.bump();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event()
                    && is_activation_key(&key, self.state)
                {
                    if let Some(callback) = &self.on_click {
                        callback();
                    }
                    self.revision.bump();
                    return true;
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let target = self.resolved_fill();
        // With a transition armed, the fill fades toward `target` from where
        // the last transition started. The host's sampled progress is the
        // eased `0..=1` fraction; `None` means "not animating", which here is
        // the settled state — the target itself, not a reason to draw nothing.
        let fill = match self.transition {
            Some(_) => {
                let t = cx.animation_value().unwrap_or(1.0).clamp(0.0, 1.0);
                mix(self.transition_from, target, t as f32)
            }
            None => target,
        };
        // The button's own fill is always the full-bounds rect, so callers and
        // golden frames that sample it keep seeing the button colour.
        if self.radius > 0.0 {
            cx.fill_rounded_rect(bounds, self.radius, fill);
        } else {
            cx.fill_rect(bounds, fill);
        }
        // Gate #7 focus indicator: a focused button gets a 2px ring drawn as a
        // band along the inner edge. It is kept off the button's fill rect so it
        // never replaces the colour the fill transition paints.
        if self.state.focused {
            const RING: Color = Color::rgb(0x4C, 0x9A, 0xFF);
            let t = 2.0;
            let i = 1.0;
            cx.fill_rect(
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y0 + i,
                    x1: bounds.x1 - i,
                    y1: bounds.y0 + t,
                },
                RING,
            );
            cx.fill_rect(
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y1 - t,
                    x1: bounds.x1 - i,
                    y1: bounds.y1 - i,
                },
                RING,
            );
            cx.fill_rect(
                Rect {
                    x0: bounds.x0 + i,
                    y0: bounds.y0 + i,
                    x1: bounds.x0 + t,
                    y1: bounds.y1 - i,
                },
                RING,
            );
            cx.fill_rect(
                Rect {
                    x0: bounds.x1 - t,
                    y0: bounds.y0 + i,
                    x1: bounds.x1 - i,
                    y1: bounds.y1 - i,
                },
                RING,
            );
        }
        let color = if self.state.disabled {
            disabled_tint(self.text_color)
        } else {
            self.text_color
        };
        draw_centered_text(cx, bounds, &self.label, LABEL_FONT_SIZE, color);
    }

    fn animation(&self) -> Option<AnimationSpec> {
        // The implicit fill-colour transition, declared rather than driven: the
        // host runs a `0 → 1` tween and hands the eased progress back through
        // `PaintContext::animation_value`, which `paint` interpolates the fill
        // with. The `key` is what makes "my target moved" restart the tween:
        // each colour change bumps it, so the host sees a *new* declaration
        // instead of an unchanged one it would leave parked.
        let transition = self.transition?;
        Some(
            AnimationSpec::new(0.0, 1.0, transition.duration_secs)
                .with_easing(transition.easing)
                .with_key(self.transition_rev),
        )
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }

    /// The resting fill and the label — the two colours a button is.
    ///
    /// The fill reported is the *base* one, not whatever the interaction state
    /// resolves to right now: a readout that changed every time the pointer
    /// crossed the button would describe the frame rather than the widget. The
    /// hover and press fills are not reported at all, because `PaintStyle` has
    /// one background and choosing one of three would be picking.
    fn paint_style(&self) -> Option<PaintStyle> {
        Some(PaintStyle {
            background: Some(self.base),
            foreground: Some(self.text_color),
            font_size: Some(LABEL_FONT_SIZE),
            radius: (self.radius > 0.0).then_some(self.radius),
            ..PaintStyle::default()
        })
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
// Pressable
// ---------------------------------------------------------------------------

/// A single-child wrapper that makes its child act as one clickable surface.
///
/// This is the missing half of Decorators::on_event: let any view take
/// take `.on_event(listener::Click, ..)`, which is why 360 call sites across 146
/// floom-ui files use it. The only other clickable widget is [], and
/// `Button` *is* its label — it paints a run of text and nothing else — so it
/// cannot carry a row of widgets. An expansion-panel header is a title plus a
/// chevron; a list row is an icon, two labels and a trailing icon. Those need
/// the interaction without the drawing, which is what this is.
///
/// Deliberately thin: it paints nothing and owns no style. Wrap it in a
/// [`crate::primitives::Container`] (or a [`Ripple`](crate::ripple::Ripple),
/// which does not consume and so composes with it) for the visuals.
///
/// ## Why the callback is `Fn()`, not `Fn() + Send + Sync`
///
/// Same reasoning as [`crate::controls::Callback`]: dispatch happens on one
/// thread, and leaving the bound off lets a callback capture an `Rc` handle to
/// the state it drives. A widget that has to live *inside* a `Child` builder
/// (which is `Send + Sync`) builds itself inside the closure from `Send + Sync`
/// captured data, and hands this an `Fn` over that.
pub struct Pressable {
    child: Child,
    on_click: Option<Callback>,
    role: SemanticsRole,
    label: Option<String>,
    state: InteractionState,
    /// Reported as [`SemanticsState::EXPANDED`] / [`COLLAPSED`].
    ///
    /// A value here or a reader in [`Self::expanded_from`], never both: a
    /// disclosure header has to survive its own toggle to keep keyboard focus,
    /// and a widget that is *not* rebuilt cannot learn a value it captured when
    /// it was built. That is what the reader is for.
    expanded: Option<bool>,
    expanded_reader: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    /// What [`Self::expanded_reader`] last reported, so a change is a change
    /// rather than a fresh read every frame.
    reported: Option<bool>,
    /// The same seam for `SELECTED`, kept separate because a widget can be both an
    /// expanded disclosure *and* a selected row, and one flag cannot be both.
    selected_reader: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    reported_selected: Option<bool>,
    revision: Revision,
}

impl std::fmt::Debug for Pressable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pressable")
            .field("role", &self.role)
            .field("state", &self.state)
            .field("expanded", &self.expanded)
            .finish_non_exhaustive()
    }
}

impl Pressable {
    /// Wraps `child`, which becomes the whole clickable surface.
    pub fn new(child: Child) -> Self {
        Self {
            child,
            on_click: None,
            role: SemanticsRole::Button,
            label: None,
            state: InteractionState::default(),
            expanded: None,
            expanded_reader: None,
            selected_reader: None,
            reported: None,
            reported_selected: None,
            revision: Revision::default(),
        }
    }

    /// Runs on a completed click — a press *and* a release over the same spot —
    /// or on an activation key while focused.
    pub fn on_click(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_click = Some(Box::new(callback));
        self.revision.bump();
        self
    }

    /// The role this surface reports. [`SemanticsRole::Button`] by default;
    /// `MenuItem`, `Tab` and friends are one call away.
    pub fn role(mut self, role: SemanticsRole) -> Self {
        self.role = role;
        self.revision.bump();
        self
    }

    /// The accessible name. Without one this surface publishes its role and no
    /// name, which leaves a screen reader with something it cannot name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self.revision.bump();
        self
    }

    /// Whether this surface is currently expanded, reported to assistive
    /// technology as `EXPANDED` or `COLLAPSED`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = Some(expanded);
        self.revision.bump();
        self
    }

    /// Reads the expanded state on every update instead of baking it in.
    ///
    /// For the widget that has to keep its identity across a toggle: a
    /// disclosure header is rebuilt by nothing, so a value captured at build
    /// time goes stale the moment the panel opens, and its reported state would
    /// be the opposite of what the user sees. Same seam as `PresenceLeaf`'s
    /// `when_signal`.
    pub fn expanded_from(mut self, read: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        self.expanded_reader = Some(Arc::new(read));
        self.revision.bump();
        self
    }

    /// Reads the selected state on every update, reported as
    /// [`SemanticsState::SELECTED`].
    ///
    /// The same seam as [`Self::expanded_from`], and for the same reason: a row in
    /// a selectable group is not rebuilt when the selection moves, so a value
    /// captured at build time goes stale the instant another row is chosen — and
    /// the stale value is the *opposite* of what the user sees. `SELECTED` is a
    /// flag rather than a pair, so an unselected row reports nothing at all.
    pub fn selected_from(mut self, read: impl Fn() -> bool + Send + Sync + 'static) -> Self {
        self.selected_reader = Some(Arc::new(read));
        self.revision.bump();
        self
    }

    /// Whether this surface is currently selected, if anything says so.
    pub fn is_selected(&self) -> Option<bool> {
        self.selected_reader.as_ref().map(|read| read())
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.state.disabled = disabled;
        self.revision.bump();
        self
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

    /// The expanded state as last read, for a test or a caller mirroring it.
    pub fn is_expanded(&self) -> Option<bool> {
        match &self.expanded_reader {
            Some(read) => Some(read()),
            None => self.expanded,
        }
    }

    pub fn child(&self) -> &Child {
        &self.child
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut hint = match &self.label {
            Some(label) => SemanticsHint::labeled(self.role, label.clone()),
            None => SemanticsHint {
                role: self.role,
                label: None,
                state: SemanticsState::empty(),
            },
        };
        hint.state = hint.state.union(self.state.to_semantics());
        if let Some(expanded) = self.is_expanded() {
            hint.state = hint.state.union(if expanded {
                SemanticsState::EXPANDED
            } else {
                SemanticsState::COLLAPSED
            });
        }
        // A flag, not a pair: an unselected row says nothing, which is what
        // `SELECTED` means. A row that never declared a selection also says
        // nothing, so a plain button is unaffected.
        if self.is_selected() == Some(true) {
            hint.state = hint.state.union(SemanticsState::SELECTED);
        }
        hint
    }
}

impl Widget for Pressable {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Pressable::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        // A reader is consulted here rather than in `paint` because this widget
        // paints nothing: what a moved flag has to invalidate is the *semantics*
        // tree, and that is reached through the dirty flag.
        let read_now = self.expanded_reader.as_ref().map(|read| read());
        if let Some(now) = read_now
            && self.reported != Some(now)
        {
            self.reported = Some(now);
            self.revision.bump();
        }
        let selected_now = self.selected_reader.as_ref().map(|read| read());
        if let Some(now) = selected_now
            && self.reported_selected != Some(now)
        {
            self.reported_selected = Some(now);
            self.revision.bump();
        }
        self.revision.sync(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        vec![self.child.clone()]
    }

    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        None
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let before = self.state;
        let consumed = match track_pointer(&mut self.state, cx, id, true) {
            Press::Activate => {
                if let Some(callback) = &self.on_click {
                    callback();
                }
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                // A focused surface still answers the keyboard, exactly as
                // `Button` does — the press/release pair has no keyboard
                // equivalent, so activation keys are matched directly.
                if let Event::Key(key) = cx.event() {
                    if is_activation_key(&key, self.state) {
                        if let Some(callback) = &self.on_click {
                            callback();
                        }
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        };
        if self.state != before {
            self.revision.bump();
        }
        consumed
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}

// ---------------------------------------------------------------------------
// Switch
// ---------------------------------------------------------------------------

pub type BoolCallback = Box<dyn Fn(bool)>;

/// The colours a [`Switch`] paints with.
///
/// A palette rather than four setters, because the four base colours are never
/// chosen independently in practice: a design system either owns a switch's
/// colours or does not own its switch at all. The defaults are the values this
/// widget has always used, so [`Default`] is exactly the pre-palette behaviour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchPalette {
    /// Track fill when on.
    pub track_on: Color,
    /// Track fill when off.
    pub track_off: Color,
    /// Knob fill when on.
    pub thumb_on: Color,
    /// Knob fill when off.
    pub track_knob_off: Color,
    /// Track fill when disabled. `None` halves the base colour's alpha, which
    /// is what this widget did before a palette existed.
    pub track_disabled: Option<Color>,
    /// Knob fill when disabled. `None` halves the base colour's alpha.
    pub thumb_disabled: Option<Color>,
}

impl Default for SwitchPalette {
    fn default() -> Self {
        Self {
            track_on: Color::rgb(0x38, 0xA1, 0x69),
            track_off: Color::rgb(0xCB, 0xD5, 0xE0),
            thumb_on: Color::WHITE,
            track_knob_off: Color::WHITE,
            track_disabled: None,
            thumb_disabled: None,
        }
    }
}

/// An on/off switch.
///
/// ## Geometry is derived, and that is a choice
///
/// By default this widget computes its own shape: a 40-wide track, half as tall
/// as it is wide, with a knob exactly as tall as the track and sitting flush
/// against one end. Nothing here is read from a theme, so the widget is usable
/// with no design system at all — which is the point of it living in the
/// primitive layer rather than in a kit.
///
/// `The `md-switch, cupertino-switch, and fluent-switch` (`md-switch`, `cupertino-switch`, `fluent-switch`) each
/// supplied three *independent* numbers instead — 52 × 32 with a 24 knob — and
/// none of those three satisfy the derivation above (`52 * 0.5 != 32`, and a 24
/// knob inside a 32-tall track does not fill it). So a kit that passes tokens
/// and a caller that passes nothing would otherwise draw different shapes under
/// the same name, and neither shape is derivable from the other.
///
/// Hence the three overrides below. Each defaults to the derivation, so every
/// existing caller is unaffected, and a kit can state its own numbers.
///
/// `knob_inset` deserves a note: the 2px gap between the knob and the track
/// end is a layout decision expressed as decoration — the knob's position
/// depends on that padding, so it is a geometric parameter here, not a paint
/// one. Setting it too large would put the knob flush against the track's end.
pub struct Switch {
    label: String,
    on: bool,
    on_toggle: Option<BoolCallback>,
    track_width: f64,
    /// Explicit track height. `None` derives half the track's width.
    track_height: Option<f64>,
    /// Explicit knob diameter. `None` uses the track's height.
    knob_diameter: Option<f64>,
    /// Gap between the knob and the track end it sits against.
    knob_inset: f64,
    palette: SwitchPalette,
    /// Whether hover and press shift the track's colour.
    ///
    /// On by default, because feedback on an interactive control is worth more
    /// than palette fidelity. Turning this off avoids passing four near-identical
    /// colours to achieve the same result.
    tints: bool,
    state: InteractionState,
    revision: Revision,
}

impl Switch {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on: false,
            on_toggle: None,
            track_width: 40.0,
            track_height: None,
            knob_diameter: None,
            knob_inset: 0.0,
            palette: SwitchPalette::default(),
            tints: true,
            state: InteractionState::default(),
            revision: Revision::default(),
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.on = on;
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

    /// An explicit track size, overriding the derived height.
    ///
    /// Without this the height is half the width. A caller passing both numbers
    /// gets exactly the box it asked for rather than a shape related to it.
    pub fn track_size(mut self, width: f64, height: f64) -> Self {
        self.track_width = width;
        self.track_height = Some(height);
        self.revision.bump();
        self
    }

    /// An explicit knob diameter, overriding "as tall as the track".
    ///
    /// A knob shorter than its track is the normal case once a kit sets the two
    /// independently, so deriving one from the other is not an option.
    pub fn knob_size(mut self, diameter: f64) -> Self {
        self.knob_diameter = Some(diameter);
        self.revision.bump();
        self
    }

    /// The gap between the knob and the track end it rests against.
    ///
    /// Defaults to flush.
    pub fn knob_inset(mut self, inset: f64) -> Self {
        self.knob_inset = inset;
        self.revision.bump();
        self
    }

    /// The colours to paint with.
    pub fn palette(mut self, palette: SwitchPalette) -> Self {
        self.palette = palette;
        self.revision.bump();
        self
    }

    /// Whether hover and press shift the track's colour. Defaults to `true`.
    pub fn tints(mut self, tints: bool) -> Self {
        self.tints = tints;
        self.revision.bump();
        self
    }

    /// The palette in use.
    pub fn switch_palette(&self) -> SwitchPalette {
        self.palette
    }

    /// The interaction state, mutably.
    ///
    /// Read-only access exists via [`Switch::interaction`], but a test or an
    /// embedder driving hover and press from outside the event loop needs to
    /// set them, and reaching in through a setter per flag would grow with the
    /// flags. An embedded switch is driven by the host, not by [`Switch`]'s own
    /// event handling, so this is the supported path rather than a test hook.
    pub fn state_mut(&mut self) -> &mut InteractionState {
        &mut self.state
    }

    pub fn is_on(&self) -> bool {
        self.on
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

    pub fn toggle(&mut self) {
        self.on = !self.on;
        self.revision.bump();
        if let Some(callback) = &self.on_toggle {
            callback(self.on);
        }
    }

    /// The track rectangle, laid flush to the left edge.
    pub fn track_bounds(&self, bounds: Rect) -> Rect {
        let width = self.track_width.min(bounds.width());
        let height = match self.track_height {
            Some(h) => h.min(bounds.height()),
            None => (width * 0.5).min(bounds.height()),
        };
        Rect {
            x0: bounds.x0,
            y0: bounds.y0 + (bounds.height() - height) / 2.0,
            x1: bounds.x0 + width,
            y1: bounds.y0 + (bounds.height() + height) / 2.0,
        }
    }

    /// The knob rectangle for the current state.
    pub fn knob_bounds(&self, bounds: Rect) -> Rect {
        let track = self.track_bounds(bounds);
        let diameter = match self.knob_diameter {
            Some(d) => d.min(track.height()),
            None => track.height(),
        };
        let x = if self.on {
            track.x1 - self.knob_inset - diameter
        } else {
            track.x0 + self.knob_inset
        };
        // Vertically centred on the track rather than stretched to it, so an
        // explicit `knob_size` yields a square knob. Spanning the track's full
        // height would make the diameter mean two different things -- width
        // here, height there -- and a 24 knob in a 32 track would render as a
        // 24x32 rectangle.
        let y = track.y0 + (track.height() - diameter) / 2.0;
        Rect {
            x0: x,
            y0: y,
            x1: x + diameter,
            y1: y + diameter,
        }
    }

    /// The track's fill colour, resolving disabled, then interaction tints.
    ///
    /// Disabled is checked before the tints because it wins outright: a kit
    /// that supplies an explicit disabled colour has said what it wants, and a
    /// hover tint on top of it would be inventing a fifth state nobody asked
    /// for. This widget's own default keeps the pre-palette behaviour, where a
    /// disabled switch was the base colour at half alpha.
    fn track_fill(&self) -> Color {
        let (base, pressed, hovered) = if self.on {
            (
                self.palette.track_on,
                Color::rgb(0x2A, 0x7D, 0x51),
                self.palette.track_on,
            )
        } else {
            (
                self.palette.track_off,
                Color::rgb(0xA0, 0xAE, 0xC0),
                Color::rgb(0xB8, 0xC2, 0xCC),
            )
        };
        if self.state.disabled {
            return self
                .palette
                .track_disabled
                .unwrap_or_else(|| disabled_tint(base));
        }
        if !self.tints {
            return base;
        }
        resolved_color(base, pressed, hovered, self.state)
    }

    /// The knob's fill. Never tinted: the knob is the affordance, and shifting
    /// it on hover would move the thing the pointer is aimed at.
    fn thumb_fill(&self) -> Color {
        let base = if self.on {
            self.palette.thumb_on
        } else {
            self.palette.track_knob_off
        };
        if self.state.disabled {
            return self
                .palette
                .thumb_disabled
                .unwrap_or_else(|| disabled_tint(base));
        }
        base
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let mut state = self.state.to_semantics();
        if self.on {
            state |= SemanticsState::CHECKED;
        }
        SemanticsHint::labeled(SemanticsRole::Switch, self.label.clone()).with_state(state)
    }
}

impl std::fmt::Debug for Switch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Switch")
            .field("label", &self.label)
            .field("on", &self.on)
            .finish_non_exhaustive()
    }
}

impl Widget for Switch {
    fn set_focused(&mut self, focused: bool) {
        // Forward to the inherent builder, which bumps the revision.
        Switch::set_focused(self, focused);
    }

    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: track (width × width/2) + gap + label extent; the
    /// height also fits the label so the track never clips it.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        if self.label.is_empty() {
            return None;
        }
        let track_w = if self.track_width > 0.0 {
            self.track_width as f32
        } else {
            SWITCH_TRACK_WIDTH
        };
        let track_h = self.track_height.map_or(track_w * 0.5, |h| h as f32);
        Some(
            floom_core::LayoutRequest::new()
                .width(track_w + INDICATOR_LABEL_GAP + label_width(&self.label))
                .height(track_h.max(label_height(&self.label))),
        )
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        match track_pointer(&mut self.state, cx, id, true) {
            Press::Activate => {
                self.toggle();
                true
            }
            Press::Consumed => true,
            Press::Ignored => {
                if let Event::Key(key) = cx.event()
                    && is_activation_key(&key, self.state)
                {
                    self.toggle();
                    return true;
                }
                false
            }
        }
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let track = self.track_bounds(bounds);
        let knob = self.knob_bounds(bounds);
        cx.fill_rounded_rect(track, track.height() as f32 / 2.0, self.track_fill());
        cx.fill_rounded_rect(knob, knob.height() as f32 / 2.0, self.thumb_fill());
        let label_origin = Point::new(track.x1 + 6.0, bounds.y0);
        cx.fill_text(
            label_origin,
            &self.label,
            LABEL_FONT_SIZE,
            FontWeight::Normal,
            Color::BLACK,
        );
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
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
// Progress
// ---------------------------------------------------------------------------

/// How much of an indeterminate bar's width the moving segment covers.
const INDETERMINATE_SEGMENT: f64 = 0.3;

/// One end-to-end pass of the indeterminate shuttle.
///
/// A convention rather than a spec value: the bar is decoration whose only job
/// is to say "still working", and this is long enough to read as motion and
/// short enough to stay out of the way.
const INDETERMINATE_CYCLE_SECS: f64 = 1.2;

/// The sweep an indeterminate bar runs on until it is given a value.
///
/// A shuttle — out and back — rather than a one-way sweep that restarts. A
/// restart jumps the segment from the far edge back to the near one every
/// cycle, which reads as a glitch rather than as progress; easing the ends
/// keeps the turnarounds from looking like a bounce.
const INDETERMINATE_SWEEP: AnimationSpec = AnimationSpec::repeating(
    0.0,
    1.0,
    INDETERMINATE_CYCLE_SECS,
    RepeatSpec::INFINITE_REVERSE,
)
.with_easing(Easing::EASE_IN_OUT);

/// Determinate or indeterminate progress indicator.
#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    /// `None` renders the indeterminate appearance.
    value: Option<f64>,
    disabled: bool,
    /// What the moving part is filled with, when it is not a flat colour.
    ///
    /// The brush carries its own coordinates, so it stays anchored to the
    /// *track* while the fill grows along it — the ramp does not compress into
    /// whatever fraction is currently filled, which is what would happen if
    /// the gradient were respecified per value. That is the whole reason this
    /// is a brush and not a second colour.
    gradient: Option<Brush>,
    revision: Revision,
}

impl Progress {
    pub fn determinate(value: f64) -> Self {
        Self {
            value: Some(value.clamp(0.0, 1.0)),
            disabled: false,
            gradient: None,
            revision: Revision::default(),
        }
    }

    pub fn indeterminate() -> Self {
        Self {
            value: None,
            disabled: false,
            gradient: None,
            revision: Revision::default(),
        }
    }

    /// Fills the moving part with a gradient instead of its flat colour. See
    /// the field's docs for what the brush anchors to.
    pub fn gradient(mut self, brush: Brush) -> Self {
        self.gradient = Some(brush);
        self.revision.bump();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.revision.bump();
        self
    }

    /// The in-place counterpart to [`Progress::disabled`]: this widget carries a
    /// [`Brush`], which owns its stops, so it is not `Copy` and the by-value
    /// builder cannot be used through a `&mut` reference. Anything reachable
    /// from `WidgetHost::widget_mut` needs this form — the by-value one silently
    /// edits a temporary wherever `Copy` still happens to be derived, which is
    /// exactly how this trap hides.
    pub fn set_disabled(&mut self, disabled: bool) -> &mut Self {
        self.disabled = disabled;
        self.revision.bump();
        self
    }

    pub fn value(&self) -> Option<f64> {
        self.value
    }

    /// The brush the moving part is filled with, when it is not a flat colour.
    pub fn gradient_brush(&self) -> Option<&Brush> {
        self.gradient.as_ref()
    }

    pub fn set_value(&mut self, value: Option<f64>) -> &mut Self {
        self.value = value.map(|v| v.clamp(0.0, 1.0));
        self.revision.bump();
        self
    }

    pub fn semantics_hint(&self) -> SemanticsHint {
        let label = match self.value {
            Some(v) => format!("{}%", (v * 100.0).round() as i32),
            None => "busy".to_string(),
        };
        let mut hint = SemanticsHint::labeled(SemanticsRole::ProgressBar, label);
        if self.disabled {
            hint = hint.with_state(SemanticsState::DISABLED);
        }
        hint
    }
}

impl Default for Progress {
    fn default() -> Self {
        Self::indeterminate()
    }
}

impl Widget for Progress {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    /// Intrinsic size: the bar fills its box, so its extent is a convention —
    /// a thin bar of default length. A pinned style still wins.
    fn layout_request(&self) -> Option<floom_core::LayoutRequest> {
        Some(
            floom_core::LayoutRequest::new()
                .width(PROGRESS_WIDTH)
                .height(PROGRESS_HEIGHT),
        )
    }

    /// An indeterminate bar keeps a shuttle running on itself; a determinate
    /// one declares nothing, because a bar that has a value is not animating.
    ///
    /// The declaration is what makes the bar move: this widget cannot start
    /// anything itself (it holds no host handle), so it states the motion it
    /// wants and the host owns running it.
    fn animation(&self) -> Option<AnimationSpec> {
        if self.value.is_some() {
            return None;
        }
        Some(INDETERMINATE_SWEEP)
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let base = if self.disabled {
            disabled_tint(Color::rgb(0x2B, 0x6C, 0xB0))
        } else {
            Color::rgb(0x2B, 0x6C, 0xB0)
        };
        let ink = self.gradient.clone().unwrap_or(Brush::Solid(base));
        // A solid ink takes the same fast path every renderer does: one colour
        // costs nothing per pixel, and the ops stay what they were before this
        // widget had a gradient at all.
        let paint_part = |cx: &mut dyn PaintContext, part: Rect| match ink.solid() {
            Some(color) => cx.fill_rect(part, color),
            None => cx.fill_rect_brush(part, &ink),
        };
        match self.value {
            Some(value) => {
                cx.fill_rect(bounds, Color::rgb(0xE2, 0xE8, 0xF0));
                let filled = Rect {
                    x1: bounds.x0 + bounds.width() * value.clamp(0.0, 1.0),
                    ..bounds
                };
                paint_part(cx, filled);
            }
            None => {
                // Indeterminate: a shuttle over a dim track. The phase is the
                // value the host sampled for this element, so this widget draws
                // the motion itself rather than having the host move the whole
                // bar for it. With nothing animating — a host that has no
                // animation, or a frame drawn before the first tick — the
                // segment parks at the start rather than vanishing, because
                // "not moving yet" is not the same as "nothing here".
                cx.fill_rect(bounds, Color::rgb(0xE2, 0xE8, 0xF0));
                let phase = cx.animation_value().unwrap_or(0.0).clamp(0.0, 1.0);
                let segment = (bounds.width() * INDETERMINATE_SEGMENT).min(bounds.width());
                let travel = (bounds.width() - segment).max(0.0);
                let x0 = bounds.x0 + travel * phase;
                paint_part(
                    cx,
                    Rect {
                        x0,
                        x1: x0 + segment,
                        ..bounds
                    },
                );
            }
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        crate::primitives::emit_semantics(self.semantics_hint(), cx);
    }
}
