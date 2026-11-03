
use std::sync::{Arc, Mutex};

use floom_accessibility::SemanticsRole;
use floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, SemanticsContext,
    UpdateContext, Widget, WidgetAny,
};
use floom_types::Color;

use floom_widgets::{emit_semantics, Pressable, Revision, SemanticsHint, Stack, Surface, Text};

/// The counter's mutable state, shared between the widget and its callers.
///
/// `Arc<Mutex<..>>` for the same reason `SharedBuffer` and `SharedTree` are: a
/// 2.0 slot is a replayed `Fn() -> Box<dyn WidgetAny> + Send + Sync`, and
/// `RwSignal` is `Rc`-based, so a caller that wants to read the value has to hold
/// something that crosses that boundary.
pub type SharedCounter = Arc<Mutex<CounterState>>;

/// Which way a step goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// Toward `max`: `value + step`.
    Up,
    /// Toward `min`: `value - step`.
    Down,
}

/// Everything a counter remembers between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CounterState {
    /// The value shown, already clamped into `[min, max]`.
    pub value: i64,
    /// Inclusive lower bound, or unbounded below.
    pub min: Option<i64>,
    /// Inclusive upper bound, or unbounded above.
    pub max: Option<i64>,
    /// How much one press moves the value.
    pub step: i64,
}

impl Default for CounterState {
    fn default() -> Self {
        Self {
            value: 0,
            min: None,
            max: None,
            step: 1,
        }
    }
}

/// Clamps `value` into `[min, max]`, applying `min` first.
///
/// The order is the behaviour: when the bounds are inverted, `max` wins.
pub fn clamp_value(value: i64, min: Option<i64>, max: Option<i64>) -> i64 {
    let mut result = value;
    if let Some(min) = min {
        result = result.max(min);
    }
    if let Some(max) = max {
        result = result.min(max);
    }
    result
}

/// The value one press in `direction` produces, from `state`.
///
/// The step is applied first and the result clamped afterwards, which is what
/// makes the last step onto a bound land exactly on it rather than overshooting.
///
/// `step` is used as given, with no sign check — so a negative `step` makes `Up`
/// decrease the value. v1 behaved the same way, and the test is named for the
/// behaviour rather than for the intent.
pub fn step_value(state: &CounterState, direction: Step) -> i64 {
    let raw = match direction {
        Step::Up => state.value.saturating_add(state.step),
        Step::Down => state.value.saturating_sub(state.step),
    };
    clamp_value(raw, state.min, state.max)
}

/// The counter's resolved appearance. Every field is supplied by the caller.
///
/// `Copy` because the slot builders below are replayed `Fn`s, and a closure may
/// only copy what it captures — a `Clone`-only style would have to be cloned into
/// each one, which is the mistake this crate has made twice already.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CounterStyle {
    /// Width and height of each step button.
    pub button_size: f32,
    /// Total width of the control.
    pub width: f32,
    /// Total height of the control.
    pub height: f32,
    /// Corner radius, of the band and of the buttons.
    pub radius: f32,
    /// Gap between the three parts.
    pub gap: f32,
    /// Border width of the band.
    pub border: f32,
    /// Border colour of the band.
    pub border_color: Color,
    /// Fill of the two buttons.
    pub button_background: Color,
    /// Colour of the two glyphs.
    pub button_foreground: Color,
    /// Font size of the value readout, and of the two glyphs.
    pub value_font_size: f32,
    /// Colour of the value readout.
    pub value_color: Color,
}

impl Default for CounterStyle {
    fn default() -> Self {
        // Sizes are `CounterTokens::default()` — the one table all three kits read.
        // Colours are deliberately neutral rather than any kit's palette.
        Self {
            button_size: 32.0,
            width: 140.0,
            height: 40.0,
            radius: 8.0,
            gap: 4.0,
            border: 1.0,
            border_color: Color::rgb(120, 120, 120),
            button_background: Color::rgb(230, 230, 230),
            button_foreground: Color::rgb(20, 20, 20),
            value_font_size: 16.0,
            value_color: Color::rgb(20, 20, 20),
        }
    }
}

/// The `on_change` callback type. `Send + Sync` because a slot closure is.
type ChangeCallback = Arc<dyn Fn(i64) + Send + Sync + 'static>;

/// A `- / value / +` stepper.
pub struct Counter {
    state: SharedCounter,
    style: CounterStyle,
    on_change: Option<ChangeCallback>,
    /// Bumped only by the builders, which take `self` by value. See the module
    /// note on why the value has no revision.
    revision: Revision,
}

impl std::fmt::Debug for Counter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Counter")
            .field("state", &self.state_snapshot())
            .field("style", &self.style)
            .finish_non_exhaustive()
    }
}

impl Counter {
    /// A counter showing `value`.
    ///
    /// The bounds are *not* clamped here, because `new` has none yet; setting a
    /// bound clamps. v1 clamped at construction because its props arrived as one
    /// struct, and two paths for one rule is two rules.
    pub fn new(value: i64) -> Self {
        Counter {
            state: Arc::new(Mutex::new(CounterState {
                value,
                ..CounterState::default()
            })),
            style: CounterStyle::default(),
            on_change: None,
            revision: Revision::default(),
        }
    }

    /// Adopt an existing shared state, so a caller that already has one — or that
    /// wants to watch it — does not have to hand over ownership.
    pub fn with_shared_state(state: SharedCounter) -> Self {
        Counter {
            state,
            style: CounterStyle::default(),
            on_change: None,
            revision: Revision::default(),
        }
    }

    /// The shared state, for a caller that wants to read or replace it.
    pub fn state(&self) -> SharedCounter {
        self.state.clone()
    }

    pub fn style(mut self, style: CounterStyle) -> Self {
        self.style = style;
        self.revision.bump();
        self
    }

    pub fn resolved_style(&self) -> CounterStyle {
        self.style.clone()
    }

    /// Called with the new value after every press or write — **including a press
    /// that did not move the value**. See the module note.
    pub fn on_change(mut self, callback: impl Fn(i64) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self.revision.bump();
        self
    }

    /// Sets the lower bound and re-clamps, so a value that is now out of range
    /// moves immediately rather than on the next press.
    pub fn min(self, min: i64) -> Self {
        self.with_bound(|state| state.min = Some(min))
    }

    /// Sets the upper bound and re-clamps.
    pub fn max(self, max: i64) -> Self {
        self.with_bound(|state| state.max = Some(max))
    }

    /// Sets the step. The step does not move the value, so nothing re-clamps.
    pub fn step_size(mut self, step: i64) -> Self {
        if let Ok(mut state) = self.state.lock() {
            state.step = step;
        }
        self.revision.bump();
        self
    }

    fn with_bound(mut self, edit: impl FnOnce(&mut CounterState)) -> Self {
        if let Ok(mut state) = self.state.lock() {
            edit(&mut state);
            state.value = clamp_value(state.value, state.min, state.max);
        }
        self.revision.bump();
        self
    }

    fn state_snapshot(&self) -> CounterState {
        match self.state.lock() {
            Ok(state) => *state,
            Err(_) => CounterState::default(),
        }
    }

    /// The value as it stands.
    pub fn value(&self) -> i64 {
        self.state_snapshot().value
    }

    /// Inclusive lower bound. Named `minimum` because the builder is `min`, and a
    /// getter and a setter cannot both be `min`.
    pub fn minimum(&self) -> Option<i64> {
        self.state_snapshot().min
    }

    /// Inclusive upper bound. Named for the same reason as `minimum`.
    pub fn maximum(&self) -> Option<i64> {
        self.state_snapshot().max
    }

    /// How much one press moves the value.
    pub fn step(&self) -> i64 {
        self.state_snapshot().step
    }

    /// Writes a value, clamped, and notifies — even if it clamped to what was
    /// already there.
    pub fn set_value(&self, value: i64) -> i64 {
        let next = match self.state.lock() {
            Ok(mut state) => {
                state.value = clamp_value(value, state.min, state.max);
                state.value
            }
            Err(_) => return 0,
        };
        self.notify(next);
        next
    }

    /// One press of `+`.
    pub fn step_up(&self) -> i64 {
        self.press(Step::Up)
    }

    /// One press of `-`.
    pub fn step_down(&self) -> i64 {
        self.press(Step::Down)
    }

    fn press(&self, direction: Step) -> i64 {
        let next = self.write_step(direction);
        self.notify(next);
        next
    }

    /// Applies one step under a single lock, so a second press cannot read a
    /// half-updated state.
    ///
    /// The write is unconditional, but **it does not need to be**, and that is
    /// worth saying because a comment claiming otherwise would be untestable. An
    /// early return when `next == state.value` was tried as a mutation and turned
    /// no test red: at a bound the value already equals `next`, so skipping the
    /// assignment is a no-op. The observable half of "a press at a bound is still
    /// a press" is the *notification*, which [`Self::press`] sends unconditionally
    /// and which `a_press_that_cannot_move_the_value_still_fires_on_change` pins.
    fn write_step(&self, direction: Step) -> i64 {
        match self.state.lock() {
            Ok(mut state) => {
                let next = step_value(&state, direction);
                state.value = next;
                next
            }
            Err(_) => 0,
        }
    }

    fn notify(&self, value: i64) {
        if let Some(on_change) = &self.on_change {
            on_change(value);
        }
    }

    /// What this counter reports to assistive technology.
    ///
    /// A group: the value is announced by the two buttons' labels plus the
    /// readout's own text, so no separate live region is emitted.
    pub fn semantics_hint(&self) -> SemanticsHint {
        SemanticsHint::labeled(SemanticsRole::Group, "Counter")
    }
}

impl Widget for Counter {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn children(&self) -> Vec<Child> {
        let style = self.style;
        let state = self.state.clone();
        let on_change = self.on_change.clone();

        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || {
                let band = Surface::new()
                    .request(
                        LayoutRequest::new()
                            .width(style.width)
                            .height(style.height)
                            .main_align(Align::Center)
                            .cross_align(Align::Center),
                    )
                    .radius(style.radius)
                    .border(style.border, style.border_color)
                    .push({
                        // Cloned here, in this `Fn`'s body, so the closures nested
                        // inside it own their copies — the outer builder is
                        // replayed, and a closure may not move a captured `Arc`
                        // out on its second run.
                        let state = state.clone();
                        let on_change = on_change.clone();
                        move || {
                            // Read inside, not captured: a value captured when
                            // the band was built is a value from whenever that
                            // was.
                            let value = match state.lock() {
                                Ok(state) => state.value,
                                Err(_) => 0,
                            };
                            let mut row = Stack::new(Axis::Horizontal)
                                .gap(style.gap)
                                .align(Align::Center, Align::Center);

                            row = row.push({
                                let state = state.clone();
                                let on_change = on_change.clone();
                                move || {
                                    step_button(
                                        "-",
                                        Step::Down,
                                        style,
                                        state.clone(),
                                        on_change.clone(),
                                    )
                                }
                            });
                            row = row.push(move || {
                                Box::new(
                                    Text::new(value.to_string())
                                        .size(style.value_font_size)
                                        .color(style.value_color),
                                )
                            });
                            row = row.push({
                                let state = state.clone();
                                let on_change = on_change.clone();
                                move || {
                                    step_button(
                                        "+",
                                        Step::Up,
                                        style,
                                        state.clone(),
                                        on_change.clone(),
                                    )
                                }
                            });

                            Box::new(row)
                        }
                    });
                Box::new(band)
            },
        )]
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

/// One step button, built as a free function so both call sites in
/// `Counter::children` are one line and neither can drift from the other.
fn step_button(
    glyph: &'static str,
    direction: Step,
    style: CounterStyle,
    state: SharedCounter,
    on_change: Option<ChangeCallback>,
) -> Box<dyn WidgetAny> {
    let accessible = match direction {
        Step::Up => "Increase",
        Step::Down => "Decrease",
    };
    Box::new(
        Surface::new()
            .request(
                LayoutRequest::new()
                    .width(style.button_size)
                    .height(style.button_size)
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(style.button_background)
            .radius(style.radius)
            .push({
                let state = state.clone();
                let on_change = on_change.clone();
                move || {
                    let state = state.clone();
                    let on_change = on_change.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().fill(),
                            move || {
                                Box::new(
                                    Text::new(glyph)
                                        .size(style.value_font_size)
                                        .color(style.button_foreground)
                                        .label(accessible.to_string()),
                                )
                            },
                        ))
                        .role(SemanticsRole::Button)
                        .label(accessible.to_string())
                        .on_click(move || {
                            let next = match state.lock() {
                                Ok(mut state) => {
                                    let next = step_value(&state, direction);
                                    state.value = next;
                                    next
                                }
                                Err(_) => return,
                            };
                            if let Some(on_change) = &on_change {
                                on_change(next);
                            }
                        }),
                    )
                }
            }),
    )
}
