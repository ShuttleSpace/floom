//! Cupertino speed dial, ported to the Floom 2.0 widget API.
//!
//! A speed dial is a floating action button and a fan of smaller ones: the
//! button opens the fan, and a button in the fan does the work.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a `Stack` holding the fan and the button, with
//! the fan mounted by the engine's `Conditional` while a shared flag says the
//! dial is open. Each button is a `Pressable` around the `Surface` that
//! paints it -- interaction without the drawing, the way the ported floating
//! action button does it.
//!
//! ## What this kit answers
//!
//! * the button's size, the action buttons' size, the gap and the radius:
//!   the tokens;
//! * the button's fill: `primary`, its ink: `on_primary`; the action
//!   buttons' fill: `secondary_container`, their ink: `on_secondary_container`;
//! * when the dial is disabled, the button's fill is `on_surface` at 12 and
//!   its ink `on_surface` at 38 -- v1's numbers;
//! * the direction: which way the fan opens and which way its buttons line up.
//!
//! ## The icons
//!
//! v1's speed dial took **raw SVG markup** for the button's icon and for
//! every action's icon. 2.0 has no SVG pipeline -- the engine draws one
//! filled outline -- so the port takes an icon *name* and resolves it from
//! this kit's own table, the same way the ported rating does. The default
//! button icon is the table's `add` glyph, which is v1's plus: the two are
//! the same path.
//!
//! ## The open state
//!
//! v1 held it in an `RwSignal<bool>` that `view()` created. 2.0 cannot hold
//! it in a signal at all -- a slot closure has to be `Send + Sync` and an
//! `RwSignal` is `Rc`-based -- so the port holds a shared flag, and the
//! engine's `Conditional` follows it. The port adds `open_flag()` and
//! `with_open_flag()`, which v1 did not have, so a caller can drive the dial
//! from outside.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The pointer cursor.** v1 gave the button and every action button
//!   `cursor(CursorStyle::Pointer)`. 2.0's `Pressable` does not answer
//!   `Widget::cursor`, so the pointer is not carried over.
//! * **The callback in the shared props.** `SpeedDialProps::on_action` is a
//!   `Box<dyn Fn(usize)>`, which cannot cross a `Send + Sync` slot boundary.
//!   The kit keeps an `Arc<dyn Fn(usize) + Send + Sync>` of its own and the
//!   props' slot stays empty.
//! * **The accessors.** v1's Cupertino speed dial already had `props()`,
//!   `theme()` and `with_theme()`. The port gives Material the same three, so
//!   the three kits are one shape.
//!
//! ## The dead token
//!
//! `SpeedDialTokens::animation_ms` -- the open/close animation's duration --
//! was never read by v1: the speed dial had no animation at all. The port
//! keeps the number in its style, because v1's tokens are the shared record,
//! and names it here: nothing in the port animates either.
//!
//! ## The dead field
//!
//! An action's `label` was stored by v1 and never rendered -- the fan is
//! buttons, and a button carries an icon, not a caption. The port keeps the
//! label in the action, and keeps not rendering it.
//!
//! ## The v1 quirk the port reproduces
//!
//! v1's `disabled` prop says "whether the whole dial is non-interactive",
//! but only the button was gated: the action buttons answered a click
//! whatever the flag said. The port gates the button and leaves the action
//! buttons alone, which is v1's code rather than v1's promise.
//!
//! And v1's closed dial still had the gap between the empty fan and the
//! button, because the empty fan was a child of the stack all the same. The
//! port's `Conditional` is a child too, so the gap is there as well.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest, WidgetAny};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{SharedFlag, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, VectorIcon};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::speed_dial::{SpeedDialDirection, SpeedDialProps};
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_cupertino_icon::{VIEW_BOX, cupertino_icon_path};

/// The button's icon when the caller names none. v1's plus, which is the
/// table's `add` glyph -- the same path.
const DEFAULT_FAB_ICON: &str = "add";

/// The icon's edge length. v1 left the icons at their intrinsic size, which
/// the port has to spell: the icon table's view box, and Cupertino's icon size.
const ICON_SIZE: f32 = 24.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(usize) + Send + Sync + 'static>;

/// One action of the fan.
struct SpeedDialAction {
    /// The icon's name in this kit's table. v1's field held SVG markup.
    icon: String,
    /// The action's label. v1 stored it and never rendered it.
    #[allow(dead_code)]
    label: Option<String>,
}

/// The speed dial's appearance, in the terms v1 used.
///
/// v1 had no style struct at all: every number was written into the markup.
/// The port adds one, because a style that does not say what the widget draws
/// is not the widget's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoSpeedDialStyle {
    pub design_system: &'static str,
    /// The button's fill.
    pub fab_background: Color,
    /// The button's ink.
    pub fab_foreground: Color,
    /// An action button's fill.
    pub action_background: Color,
    /// An action button's ink.
    pub action_foreground: Color,
    pub fab_size: f32,
    pub action_size: f32,
    pub gap: f32,
    pub radius: f32,
    /// The open/close animation's duration. v1 declared the token and never
    /// animated anything; the port keeps the number and names it.
    pub animation_ms: f32,
}

/// Cupertino's speed dial.
pub struct CupertinoSpeedDial {
    props: SpeedDialProps,
    actions: Vec<SpeedDialAction>,
    theme: RwSignal<MultiTheme>,
    /// Whether the fan is open, shared with whoever opens it.
    open: SharedFlag,
    /// The action callback in the shape 2.0's slots need.
    on_action: Option<Callback>,
}

impl CupertinoSpeedDial {
    pub fn new() -> Self {
        Self {
            props: SpeedDialProps::default(),
            actions: Vec::new(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            open: shared_flag(false),
            on_action: None,
        }
    }

    /// Append an action to the fan.
    ///
    /// The icon is a *name* in this kit's own table, where v1 took SVG markup.
    /// The label is kept and, as in v1, never rendered.
    pub fn action(mut self, icon: impl Into<String>, label: Option<String>) -> Self {
        self.actions.push(SpeedDialAction {
            icon: icon.into(),
            label,
        });
        self
    }

    /// Which way the fan opens.
    pub fn direction(mut self, direction: SpeedDialDirection) -> Self {
        self.props.direction = direction;
        self
    }

    /// Whether the dial starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.props.open = open;
        write_flag(&self.open, open);
        self
    }

    /// The button's icon, by name in this kit's table. Empty means the
    /// default plus.
    pub fn fab_icon(mut self, icon: impl Into<String>) -> Self {
        self.props.fab_icon = icon.into();
        self
    }

    /// Whether the button is inert.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Called with the action's index when an action button is pressed.
    pub fn on_action(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

    /// The shared props.
    ///
    /// `on_action` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and
    /// `SpeedDialProps` is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0
    /// cannot use.
    pub fn props(&self) -> &SpeedDialProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The flag that says whether the fan is open.
    ///
    /// v1 held the open state in a signal `view()` created, and nothing
    /// outside could reach it. 2.0's shared state is a `SharedFlag`, and the
    /// port hands it out so a caller can drive the dial.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Use a flag the caller already has, so the thing that opens the dial and
    /// the dial itself share one truth.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    pub fn resolved_style(&self) -> CupertinoSpeedDialStyle {
        let theme = self.theme.get();
        let ComponentTokens::SpeedDial(tokens) = theme.component(ComponentId::SpeedDial) else {
            unreachable!("ComponentId::SpeedDial must resolve to SpeedDialTokens")
        };
        let disabled = self.props.disabled;
        CupertinoSpeedDialStyle {
            design_system: "cupertino",
            // v1: `primary` with `on_primary` ink, or `on_surface` at 12 and
            // 38 when the dial is disabled.
            fab_background: if disabled {
                with_alpha(theme.colors().on_surface, 12)
            } else {
                theme.colors().primary
            },
            fab_foreground: if disabled {
                with_alpha(theme.colors().on_surface, 38)
            } else {
                theme.colors().on_primary
            },
            // v1: the action buttons were `secondary_container` with
            // `on_secondary_container` ink, disabled or not.
            action_background: theme.colors().secondary_container,
            action_foreground: theme.colors().on_secondary_container,
            fab_size: tokens.fab_size,
            action_size: tokens.action_size,
            gap: tokens.gap,
            radius: tokens.radius,
            animation_ms: tokens.animation_ms,
        }
    }

    /// The dial: the fan and the button.
    pub fn view(self) -> Stack {
        let style = self.resolved_style();
        let open = self.open.clone();
        let disabled = self.props.disabled;
        let on_action = self.on_action.clone();
        let direction = self.props.direction;

        // The button: a filled circle with the icon in it, and the click
        // target around it.
        let fab_icon = if self.props.fab_icon.is_empty() {
            DEFAULT_FAB_ICON.to_string()
        } else {
            self.props.fab_icon.clone()
        };
        let fab_glyph = VectorIcon::from_path_data(cupertino_icon_path(&fab_icon), VIEW_BOX)
            .size(ICON_SIZE)
            .color(to_paint(style.fab_foreground));
        let fab_surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .width(style.fab_size)
                    .height(style.fab_size)
                    // v1's `.items_center().justify_center()`.
                    .main_align(Align::Center)
                    .cross_align(Align::Center),
            )
            .background(to_paint(style.fab_background))
            .radius(style.radius)
            .push({
                let fab_glyph = fab_glyph.clone();
                move || Box::new(fab_glyph.clone())
            });
        // The button's slot. The `Pressable` is built *inside* it, because
        // its callback is not `Send + Sync` and a slot closure has to be.
        let fab_slot = {
            let surface = fab_surface;
            let open = open.clone();
            move || {
                let surface = surface.clone();
                let open = open.clone();
                // The coercion is inside the closure: a closure whose return
                // type is inferred from its body keeps `Box<Pressable>`, and the
                // unsizing to `Box<dyn WidgetAny>` cannot happen afterwards.
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .disabled(disabled)
                    .on_click(move || {
                        // v1 toggled the open signal, and only when the dial was
                        // not disabled.
                        if !disabled {
                            let now = read_flag(&open);
                            write_flag(&open, !now);
                        }
                    })
                    .label("Speed dial"),
                ) as Box<dyn WidgetAny>
            }
        };

        // The fan: one filled circle per action.
        let mut buttons = match direction {
            SpeedDialDirection::Left | SpeedDialDirection::Right => {
                Stack::new(Axis::Horizontal)
            }
            SpeedDialDirection::Up | SpeedDialDirection::Down => Stack::new(Axis::Vertical),
        }
        .gap(style.gap)
        .align(Align::Start, cross_alignment(direction));

        for (index, action) in self.actions.iter().enumerate() {
            let glyph = VectorIcon::from_path_data(cupertino_icon_path(&action.icon), VIEW_BOX)
            .size(ICON_SIZE)
            .color(to_paint(style.action_foreground));
            let surface = Surface::new()
                .request(
                    LayoutRequest::new()
                        .width(style.action_size)
                        .height(style.action_size)
                        .main_align(Align::Center)
                        .cross_align(Align::Center),
                )
                .background(to_paint(style.action_background))
                .radius(style.radius)
                .push({
                    let glyph = glyph.clone();
                    move || Box::new(glyph.clone())
                });
            let on_action = on_action.clone();
            buttons = buttons.push(move || {
                // The `Pressable` is built *here*, inside the slot, for the
                // same reason the button's is; and the values its closure
                // takes are copied, because a slot is replayed.
                let surface = surface.clone();
                let on_action = on_action.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(surface.clone()),
                    ))
                    .on_click(move || {
                        // v1 ran the action's callback and nothing else: the
                        // fan stayed open.
                        if let Some(cb) = &on_action {
                            cb(index);
                        }
                    })
                    .label(format!("Action {index}")),
                )
            });
        }

        // The fan is mounted only while the dial is open. v1's `dyn_view`
        // answered the same question with an `Empty` view.
        //
        // `Conditional::new(false)` starts "not yet seen", so the first update
        // reads the flag as a *change*: a dial that starts closed would have
        // its fan mounted at once, and the button's tap -- which is what opens
        // the dial -- would find the flag already "seen" and never open it.
        // `sync_when` is the engine's door for a widget that already knows the
        // answer, and it is the one the navigation drawers use.
        let mut fan = Conditional::new(false).when_flag(open.clone());
        if !read_flag(&open) {
            fan.sync_when(false);
        }
        let fan = fan.show(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(buttons.clone()),
        ));

        let fan_slot = {
            let fan = fan;
            // The coercion is inside the closure: a closure whose return type
            // is inferred from its body keeps `Box<Conditional>`, and the
            // unsizing to `Box<dyn WidgetAny>` cannot happen afterwards.
            move || Box::new(fan.clone()) as Box<dyn WidgetAny>
        };

        let mut dial = Stack::new(dial_axis(direction))
            .gap(style.gap)
            .align(Align::Start, cross_alignment(direction));
        if fab_first(direction) {
            dial = dial.push(fab_slot);
            dial = dial.push(fan_slot);
        } else {
            dial = dial.push(fan_slot);
            dial = dial.push(fab_slot);
        }
        dial
    }
}

impl Default for CupertinoSpeedDial {
    fn default() -> Self {
        Self::new()
    }
}

/// The axis the dial runs on: vertical for up and down, horizontal for left
/// and right.
fn dial_axis(direction: SpeedDialDirection) -> Axis {
    match direction {
        SpeedDialDirection::Up | SpeedDialDirection::Down => Axis::Vertical,
        SpeedDialDirection::Left | SpeedDialDirection::Right => Axis::Horizontal,
    }
}

/// v1's `items_end` / `items_start` / `items_center`, which are the *cross*
/// axis of the stack: the fan lines up to the end, to the start, or in the
/// middle.
fn cross_alignment(direction: SpeedDialDirection) -> Align {
    match direction {
        SpeedDialDirection::Up => Align::End,
        SpeedDialDirection::Down => Align::Start,
        SpeedDialDirection::Left | SpeedDialDirection::Right => Align::Center,
    }
}

/// Whether the button comes before the fan. Up and left open upwards and
/// leftwards, so the fan leads; down and right put the button first.
fn fab_first(direction: SpeedDialDirection) -> bool {
    matches!(
        direction,
        SpeedDialDirection::Down | SpeedDialDirection::Right
    )
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
