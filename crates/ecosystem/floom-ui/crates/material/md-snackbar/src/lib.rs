//! Material snackbar, ported to the Floom 2.0 widget API.
//!
//! A snackbar is a message that appears over the page for a while: a dark
//! bar at the bottom with the message in it, and an optional action at the
//! end.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: the engine's `Overlay` -- a layer that covers its
//! parent's box -- holding a `Conditional` that mounts the snackbar while a
//! shared flag says it is up, and inside that a `Surface` for the bar and a
//! horizontal `Stack` for the message and the action.
//!
//! The `Overlay` is a *layout* primitive: the layer covers the box the
//! snackbar is declared in and is clipped by it. v1's `Overlay` was a layer
//! above everything on the page, so a snackbar declared inside the page is now
//! inside the page's box -- and one declared at the root still covers the
//! window. The engine's other overlay layers are `Dialog`, which is modal, and
//! `Popup`, which is anchored to a control and dismissed by a click beside it;
//! neither is a toast.
//!
//! ## What this kit answers
//!
//! * the bar's padding, radius, minimum height, maximum width, the message's
//!   size and the gap before the action: the tokens;
//! * the bar's fill: `on_surface`; the message's ink: `surface`; the action's
//!   ink: `primary`;
//! * the action's size and weight: the theme's `label.large`, in medium
//!   weight;
//! * the bar's shadow: a black-at-40 blur of 16 with a spread of 1;
//! * where the bar sits: centred at the bottom, 24 above the bottom edge.
//!
//! ## The timer
//!
//! **2.0's engine has no timer, and that is a gap the port names rather than
//! hides.** v1 armed the dismissal itself: an effect on the open signal
//! started a timer when the snackbar appeared, and the timer closed it
//! again. 2.0 has no timer to arm, and -- because a kit is not told that a
//! flag it does not own went true -- no way to be told that the snackbar
//! opened. The port keeps `duration_ms` in the props and offers
//! [`MaterialSnackbar::dismiss_after`], which is v1's dismissal asked for by
//! the caller rather than by the kit. A caller that wants v1's behaviour
//! writes the flag and asks for the dismissal; a caller that wants the
//! snackbar to stay until it is dismissed does nothing.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The slide and the fade.** v1's animation moved the snackbar up 40
//!   points *and* faded it in, over 250 milliseconds, on the way in and on
//!   the way out. 2.0's presence transition carries one channel per element,
//!   so the port slides -- the motion that says "this came from the bottom" --
//!   and the fade is dropped.
//! * **The pointer cursor.** v1 gave the action `cursor(CursorStyle::Pointer)`.
//!   2.0's `Pressable` does not answer `Widget::cursor`, so the pointer is
//!   not carried over.
//! * **The callback in the shared props.** `SnackbarProps::on_action` is a
//!   `Box<dyn Fn()>`, which cannot cross a `Send + Sync` slot boundary. The
//!   kit keeps an `Arc<dyn Fn() + Send + Sync>` of its own and the props'
//!   slot stays empty.
//! * **The accessors.** v1's Material snackbar had no `props()`, `theme()` or
//!   `with_theme()` -- the other two kits had all three. The port gives all
//!   three kits the same three, because a kit that cannot be given a theme
//!   cannot be tested against one.
//!
//! ## The one field the port's style adds
//!
//! v1's style reported the bar's padding, radius, height, width, the
//! message's size and the gap -- and nothing about the action's own size and
//! weight, its padding, the shadow, or where the bar sits, all of which the
//! paint used. The port's style reports all of them, because a style that
//! does not say what the widget draws is not the widget's output.

use std::sync::Arc;
use std::time::Duration;

use floom_core::{Child, ChildKey, LayoutRequest, PresenceTransition};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_types::{FontWeight, Insets, Shadow};
use floom_ui_composites::Overlay;
use floom_widgets::text_input::{SharedFlag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::snackbar::SnackbarProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_core_theme::tokens::typography::FontWeight as ThemeFontWeight;
use floom_widget_runtime::theme_signal;

/// The action's padding at the sides. A v1 literal.
const ACTION_PADDING_HORIZONTAL: f64 = 8.0;

/// How far the snackbar slides up into place, in points. A v1 literal.
const SLIDE_DISTANCE: f32 = 40.0;

/// How long the slide takes, in seconds. v1 wrote 250 milliseconds.
const ANIMATION_SECONDS: f64 = 0.25;

/// How far above the bottom edge the bar sits. A v1 literal.
const BOTTOM_PADDING: f64 = 24.0;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

/// The snackbar's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialSnackbarStyle {
    pub design_system: &'static str,
    /// The bar's fill.
    pub surface: Color,
    /// The message's ink.
    pub on_surface: Color,
    /// The action's ink.
    pub action_color: Color,
    pub padding: f32,
    pub border_radius: f32,
    pub min_height: f32,
    pub font_size: f32,
    pub max_width: f32,
    pub action_gap: f32,
    // --- the fields v1's style did not report, and the paint used ---
    /// The action's own size: the theme's `label.large`.
    pub action_font_size: f32,
    /// The action's own weight.
    pub action_font_weight: ThemeFontWeight,
    /// The action's padding at the sides.
    pub action_padding_horizontal: f32,
    /// The bar's shadow.
    pub shadow: Shadow,
    /// The inset the layer keeps around the bar: v1's `padding_bottom(24.0)`.
    pub layer_padding: Insets,
    /// How long the slide takes, in milliseconds.
    pub animation_ms: f32,
}

/// Material's snackbar.
pub struct MaterialSnackbar {
    props: SnackbarProps,
    /// Whether the snackbar is up, shared with whoever shows it.
    show: SharedFlag,
    theme: RwSignal<MultiTheme>,
    /// The action callback in the shape 2.0's slots need.
    on_action: Option<Callback>,
}

impl MaterialSnackbar {
    pub fn new() -> Self {
        Self {
            props: SnackbarProps::default(),
            show: shared_flag(false),
            theme: theme_signal(),
            on_action: None,
        }
    }

    /// The message in the bar.
    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.props.message = message.into();
        self
    }

    /// The action's label. `None` leaves the bar without an action.
    pub fn action_label(mut self, action_label: Option<String>) -> Self {
        self.props.action_label = action_label;
        self
    }

    /// Called when the action is pressed.
    pub fn on_action(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_action = Some(Arc::new(callback));
        self
    }

    /// How long the snackbar is meant to stay up, in milliseconds.
    ///
    /// 2.0's engine has no timer, so this number does not dismiss anything by
    /// itself: see [`Self::dismiss_after`].
    pub fn duration_ms(mut self, duration_ms: u32) -> Self {
        self.props.duration_ms = duration_ms;
        self
    }

    /// The shared props.
    ///
    /// `on_action` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and
    /// `SnackbarProps` is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0
    /// cannot use.
    pub fn props(&self) -> &SnackbarProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The flag that says whether the snackbar is up.
    ///
    /// v1 handed out an `RwSignal<bool>` as `open_signal()`; 2.0's shared
    /// state is a `SharedFlag`, because a value that changes after mount
    /// cannot be held in a signal and read during a render.
    pub fn open_flag(&self) -> SharedFlag {
        self.show.clone()
    }

    /// Use a flag the caller already has, so the thing that shows the snackbar
    /// and the snackbar itself share one truth.
    pub fn with_open_flag(mut self, show: SharedFlag) -> Self {
        self.show = show;
        self
    }

    /// Dismiss the snackbar after `duration_ms`.
    ///
    /// v1 armed this itself -- an effect on the open signal started a timer
    /// when the snackbar appeared -- and 2.0's engine has no timer, and no way
    /// to be told that a flag the kit does not own went true. So the port
    /// offers v1's dismissal as something the caller asks for.
    ///
    /// The thread is the shape v1's own Cupertino snackbar used.
    pub fn dismiss_after(&self, duration_ms: u32) {
        let show = self.show.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(u64::from(duration_ms)));
            write_flag(&show, false);
        });
    }

    pub fn resolved_style(&self) -> MaterialSnackbarStyle {
        let theme = self.theme.get();
        let ComponentTokens::Snackbar(tokens) = theme.component(ComponentId::Snackbar) else {
            unreachable!("ComponentId::Snackbar must resolve to SnackbarTokens")
        };
        MaterialSnackbarStyle {
            design_system: "material",
            // v1: the bar is `on_surface` and the message is `surface` -- the
            // two are the other way round from the naming.
            surface: theme.colors().on_surface,
            on_surface: theme.colors().surface,
            action_color: theme.colors().primary,
            padding: tokens.padding,
            border_radius: tokens.border_radius,
            min_height: tokens.min_height,
            font_size: tokens.font_size,
            max_width: tokens.max_width,
            action_gap: tokens.action_gap,
            // v1's action: the theme's `label.large`, in medium weight.
            action_font_size: theme.typography().label.large.font_size,
            action_font_weight: ThemeFontWeight::Medium,
            action_padding_horizontal: ACTION_PADDING_HORIZONTAL as f32,
            // v1: `box_shadow_blur(16.0).box_shadow_spread(1.0)
            // .box_shadow_color(with_alpha(black, 40))`.
            shadow: Shadow {
                color: to_paint(with_alpha(Color::rgba(0, 0, 0, 255), 40)),
                blur: 16.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
            layer_padding: Insets {
                top: 0.0,
                right: 0.0,
                bottom: BOTTOM_PADDING,
                left: 0.0,
            },
            animation_ms: (ANIMATION_SECONDS * 1000.0) as f32,
        }
    }

    /// The snackbar: the layer, the bar, the message and the action.
    pub fn view(self) -> Overlay {
        let style = self.resolved_style();
        let show = self.show.clone();
        let on_action = self.on_action.clone();
        let message = self.props.message.clone();
        let action_label = self.props.action_label.clone();

        // The row: the message, and the action when there is one.
        let mut row = Stack::new(Axis::Horizontal)
            // v1's `.items_center().gap(gap)`.
            .gap(style.action_gap)
            .align(Align::Start, Align::Center)
            .push({
                let message = message.clone();
                let ink = to_paint(style.on_surface);
                move || Box::new(Text::new(message.clone()).size(style.font_size).color(ink))
            });

        if let Some(label) = action_label {
            let show = show.clone();
            row = row.push(move || {
                // The `Pressable` is built *here*, inside the slot: its
                // callback is not `Send + Sync` and a slot closure has to be.
                let show = show.clone();
                let on_action = on_action.clone();
                let label = label.clone();
                let ink = to_paint(style.action_color);
                let size = style.action_font_size;
                let weight = style.action_font_weight;
                let pad = style.action_padding_horizontal;
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || {
                            let label = label.clone();
                            Box::new(
                                Stack::new(Axis::Horizontal)
                                    // v1's `padding_horiz(8.0)` on the action.
                                    .request(
                                        LayoutRequest::new()
                                            .padding_horiz(f64::from(pad)),
                                    )
                                    .push(move || {
                                        Box::new(
                                            Text::new(label.clone())
                                                .size(size)
                                                .weight(to_engine_weight(weight))
                                                .color(ink),
                                        )
                                    }),
                            )
                        },
                    ))
                    .on_click(move || {
                        // v1 ran the action's callback and then closed the
                        // snackbar.
                        if let Some(cb) = &on_action {
                            cb();
                        }
                        write_flag(&show, false);
                    })
                    .label("Snackbar action"),
                )
            });
        }

        // The bar.
        let surface = Surface::new()
            .request(
                LayoutRequest::new()
                    .min_height(style.min_height)
                    .max_width(style.max_width)
                    // v1's `items_center()`.
                    .cross_align(Align::Center),
            )
            .padding(Insets {
                left: f64::from(style.padding),
                right: f64::from(style.padding),
                top: f64::from(style.padding),
                bottom: f64::from(style.padding),
            })
            .background(to_paint(style.surface))
            .radius(style.border_radius)
            .shadow(style.shadow)
            .push(move || Box::new(row.clone()));

        // The layer the bar sits in: the whole overlay's box, with the bar
        // centred at the bottom.
        let layer = Stack::new(Axis::Vertical)
            .request(
                LayoutRequest::new()
                    .width_pct(100.0)
                    .height_pct(100.0)
                    // v1's `justify_end()`.
                    .main_align(Align::End)
                    // v1's `items_center()`.
                    .cross_align(Align::Center)
                    .padding(style.layer_padding),
            )
            .push(move || Box::new(surface.clone()));

        // The snackbar is mounted only while it is up. v1 hid the same layer
        // with `apply_if(!open, |s| s.hide())`.
        let content = Conditional::new(false)
            .when_flag(show.clone())
            .enter(PresenceTransition::slide(ANIMATION_SECONDS, SLIDE_DISTANCE))
            .exit(PresenceTransition::slide(ANIMATION_SECONDS, SLIDE_DISTANCE))
            .show(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(layer.clone()),
            ));

        Overlay::new().content(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(content.clone()),
        ))
    }
}

impl Default for MaterialSnackbar {
    fn default() -> Self {
        Self::new()
    }
}

/// The theme's weight -> the engine's.
fn to_engine_weight(weight: ThemeFontWeight) -> FontWeight {
    match weight {
        ThemeFontWeight::Thin => FontWeight::Thin,
        ThemeFontWeight::ExtraLight => FontWeight::ExtraLight,
        ThemeFontWeight::Light => FontWeight::Light,
        ThemeFontWeight::Regular => FontWeight::Normal,
        ThemeFontWeight::Medium => FontWeight::Medium,
        ThemeFontWeight::SemiBold => FontWeight::SemiBold,
        ThemeFontWeight::Bold => FontWeight::Bold,
        ThemeFontWeight::ExtraBold => FontWeight::ExtraBold,
        ThemeFontWeight::Black => FontWeight::Black,
    }
}

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor { r: c.r, g: c.g, b: c.b, a: c.a }
}
