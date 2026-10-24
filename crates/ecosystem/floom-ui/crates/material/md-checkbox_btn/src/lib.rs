//! Material checkbox button, ported to the Floom 2.0 widget API.
//!
//! A checkbox button is a *chip-shaped* toggle: a rounded box with a check mark
//! in it and a label beside it. It is not the engine's square `Checkbox`, and
//! it is not a radio: one of them is chosen by being switched on.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the pill, a `VectorIcon` for the
//! mark -- this kit's own icon table, `MdIcon::new("check")` -- and a `Text` for
//! the label. There is nothing here that a widget could own, so the shape stays
//! in the kit.
//!
//! ## The checked state
//!
//! v1 kept the state in an `RwSignal<bool>` and read it inside the pill's
//! style closure, so the pill repainted itself the moment the state changed.
//! 2.0's tree is mounted once, and an `RwSignal` is `Rc`-based, so the state is
//! the engine's [`SharedFlag`] -- an `Arc<Mutex<bool>>` -- and the engine's
//! [`Conditional`] follows it: the flag is read in `update`, so whoever writes
//! it drives the pill.
//!
//! ## The two flags, and why there are two
//!
//! The pill has two appearances and the engine's `Conditional` shows one child
//! at a time, so the kit keeps **a pair** of flags -- the state and its
//! inverse -- and writes them together. The pair is the kit's own business:
//! [`MaterialCheckboxBtn::set_checked`] is the only writer, so the two cannot
//! drift apart, and no raw flag is handed out for a host to write behind the
//! kit's back.
//!
//! ## The style the port adds
//!
//! v1's checkbox button had **no** `resolved_style` at all: every colour was
//! read inline inside the pill's style closure. The port adds
//! [`MaterialCheckboxBtnStyle`], because a kit whose appearance cannot be asked
//! about cannot be tested against one.
//!
//! ## The mark's colour while disabled and unchecked
//!
//! This is the one place the three kits disagree, and the disagreement is in
//! the pixels. Material wrote the disabled tint *inside* the checked branch, so
//! an unchecked and disabled button drew its mark **transparent**. Cupertino
//! and Fluent wrote it *outside*, so an unchecked and disabled button drew a
//! visible grey check mark on a greyed pill. The port keeps each kit's own
//! answer: Material's is
//! [`MaterialCheckboxBtnStyle::unchecked_disabled_mark_color`] and the other
//! two's is the same field with a different value.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`.** 2.0's paint context has no cursor
//!   vocabulary. The pill is still a `Pressable`, so it is still activatable by
//!   keyboard, which the pointer was not.
//! * **`justify_center()`.** v1 centred the mark and the label in the pill. In
//!   2.0 the stack centres its children the same way, and the effect shows only
//!   when the pill is wider than its content -- which it is not, because
//!   nothing gives it a width.
//! * **`keyboard_navigable()`.** v1's flag made the pill answer Enter and
//!   Space. 2.0's `Pressable` answers the activation keys itself, so the
//!   behaviour is there without the flag.
//!
//! ## The accessors
//!
//! v1's checkbox buttons had none: no `props()`, no `theme()`, no
//! `with_theme()`. The port gives them all three, because a kit that cannot be
//! given a theme cannot be tested against one.
//!
//! ## The one thing the port adds
//!
//! v1 published nothing to assistive technology. 2.0's `Pressable` does, so the
//! port gives the pill the label as its accessible name and reports the
//! checked state to assistive technology from the flag -- live, the way the
//! rest of the state is. That is a fix rather than a translation, and it is
//! named here rather than slipped in.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::checkbox_btn::CheckboxBtnProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color_ext::with_alpha;
use floom_widget_md_icon::MdIcon;
use floom_widget_runtime::theme_signal;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

/// The rim's width. A v1 literal.
const BORDER_WIDTH: f32 = 1.0;
/// The mark's size, as a multiple of the label's size. A v1 literal.
const MARK_SCALE: f32 = 1.2;
/// v1's disabled fill: `on_surface` at 12.
const DISABLED_FILL_ALPHA: u8 = 12;
/// v1's disabled rim, mark and label: `on_surface` at 38.
const DISABLED_ALPHA: u8 = 38;

/// The checkbox button's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialCheckboxBtnStyle {
    pub design_system: &'static str,
    /// The pill's fill, unchecked.
    pub background: Color,
    /// The pill's rim, unchecked.
    pub border_color: Color,
    /// The pill's fill, checked.
    pub checked_background: Color,
    /// The pill's rim, checked.
    pub checked_border_color: Color,
    /// The pill's fill, disabled. v1's disabled fill won over the checked one.
    pub disabled_background: Color,
    /// The pill's rim, disabled.
    pub disabled_border_color: Color,
    /// The mark's ink, checked and enabled.
    pub mark_color: Color,
    /// The mark's ink, checked and disabled.
    pub disabled_mark_color: Color,
    /// The mark's ink, unchecked and enabled: v1 drew it transparent.
    pub unchecked_mark_color: Color,
    /// The mark's ink, unchecked and disabled.
    ///
    /// Material's is transparent -- v1 wrote the disabled tint inside the
    /// checked branch. Cupertino's and Fluent's are the disabled tint, because
    /// they wrote it outside.
    pub unchecked_disabled_mark_color: Color,
    /// The label's ink.
    pub label_color: Color,
    /// The label's ink, disabled.
    pub disabled_label_color: Color,
    /// The pill's height, from the token.
    pub height: f32,
    /// The pill's padding, from the token.
    pub padding: f32,
    /// The pill's radius, from the token.
    pub radius: f32,
    /// The gap between the mark and the label, from the token.
    pub gap: f32,
    /// The label's size, from the token.
    pub font_size: f32,
    /// The mark's size: v1 wrote the font size times 1.2.
    pub mark_size: f32,
    /// The rim's width. A v1 literal.
    pub border_width: f32,
}

/// Material's checkbox button.
#[derive(Clone)]
pub struct MaterialCheckboxBtn {
    props: CheckboxBtnProps,
    theme: RwSignal<MultiTheme>,
    /// The checked state, as the engine's flag.
    checked: SharedFlag,
    /// The same state, inverted.
    ///
    /// The engine's `Conditional` follows one flag and the pill has two
    /// appearances, so the kit keeps a pair and writes them together.
    unchecked: SharedFlag,
    /// The change callback in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn(bool)>` in the shared
    /// props, which cannot cross that boundary.
    on_change: Option<Callback>,
}

impl MaterialCheckboxBtn {
    pub fn new() -> Self {
        Self {
            props: CheckboxBtnProps::default(),
            theme: theme_signal(),
            checked: shared_flag(false),
            unchecked: shared_flag(true),
            on_change: None,
        }
    }

    /// The initial checked state.
    pub fn checked(mut self, checked: bool) -> Self {
        // v1 wrote the state into the props as well as into the signal, so
        // `props()` reported it.
        self.props.checked = checked;
        self.set_checked(checked);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = Some(label.into());
        self
    }

    /// Called with the new checked state whenever it toggles.
    pub fn on_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The shared props, for a caller that reads the design-system-neutral shape.
    ///
    /// `on_change` is deliberately absent from what it reports: the kit holds
    /// that as an `Arc` of its own (see the field's note), and
    /// `CheckboxBtnProps` is the v1 carrier of a `Box<dyn Fn(..)>` that 2.0
    /// cannot use.
    pub fn props(&self) -> &CheckboxBtnProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// Whether the button is checked right now.
    pub fn is_checked(&self) -> bool {
        read_flag(&self.checked)
    }

    /// Set the checked state.
    ///
    /// The only writer of the pair of flags: the kit keeps them in step, so a
    /// host cannot leave the pill showing one appearance while the state says
    /// another.
    pub fn set_checked(&self, checked: bool) {
        write_flag(&self.checked, checked);
        write_flag(&self.unchecked, !checked);
    }

    pub fn resolved_style(&self) -> MaterialCheckboxBtnStyle {
        let theme = self.theme.get();
        let ComponentTokens::CheckboxBtn(tokens) = theme.component(ComponentId::CheckboxBtn) else {
            unreachable!("ComponentId::CheckboxBtn must resolve to CheckboxBtnTokens")
        };
        let c = theme.colors();
        MaterialCheckboxBtnStyle {
            design_system: "material",
            background: c.surface_variant,
            border_color: c.outline,
            checked_background: c.primary,
            checked_border_color: c.primary,
            disabled_background: with_alpha(c.on_surface, DISABLED_FILL_ALPHA),
            disabled_border_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            mark_color: c.on_primary,
            disabled_mark_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            // v1 drew the unchecked mark transparent, and wrote the disabled tint
            // *inside* the checked branch -- so an unchecked and disabled button
            // drew it transparent too.
            unchecked_mark_color: Color::rgba(0, 0, 0, 0),
            unchecked_disabled_mark_color: Color::rgba(0, 0, 0, 0),
            label_color: c.on_surface,
            disabled_label_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            height: tokens.height,
            padding: tokens.padding,
            radius: tokens.radius,
            gap: tokens.gap,
            font_size: tokens.font_size,
            mark_size: tokens.font_size * MARK_SCALE,
            border_width: BORDER_WIDTH,
        }
    }

    /// The pill: two appearances, and the engine's `Conditional` choosing
    /// between them.
    pub fn view(self) -> Stack {
        let ctx = PillContext {
            style: self.resolved_style(),
            disabled: self.props.disabled,
            label: self.props.label.clone(),
            on_change: self.on_change.clone(),
            checked: self.checked.clone(),
            unchecked: self.unchecked.clone(),
        };
        let checked_flag = ctx.checked.clone();
        let unchecked_flag = ctx.unchecked.clone();

        Stack::new(Axis::Horizontal)
            .push({
                let ctx = ctx.clone();
                let flag = checked_flag.clone();
                move || Box::new(appearance(flag.clone(), ctx.clone(), true))
            })
            .push({
                let ctx = ctx.clone();
                let flag = unchecked_flag.clone();
                move || Box::new(appearance(flag.clone(), ctx.clone(), false))
            })
    }
}

impl Default for MaterialCheckboxBtn {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything the pill needs that is not in the style.
#[derive(Clone)]
struct PillContext {
    style: MaterialCheckboxBtnStyle,
    disabled: bool,
    label: Option<String>,
    on_change: Option<Callback>,
    checked: SharedFlag,
    unchecked: SharedFlag,
}

/// The conditional that shows the pill in one of its two appearances.
///
/// The flag is the engine's own: the `Conditional` reads it in `update`, so
/// whoever writes it mounts the other pill.
fn appearance(flag: SharedFlag, ctx: PillContext, checked: bool) -> Conditional {
    Conditional::new(false).when_flag(flag).show(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || build_pill(&ctx, checked),
    ))
}

/// The pill itself: the box, the mark, the label, and the click that toggles.
fn build_pill(ctx: &PillContext, checked: bool) -> Box<dyn floom_widgets::WidgetAny> {
    let style = ctx.style;
    let disabled = ctx.disabled;

    // v1's order: the disabled fill and rim won over the checked ones, and the
    // mark's colour was read off the same three-way answer.
    let (background, border_color, mark_ink, label_ink) = if disabled {
        (
            style.disabled_background,
            style.disabled_border_color,
            if checked {
                style.disabled_mark_color
            } else {
                style.unchecked_disabled_mark_color
            },
            style.disabled_label_color,
        )
    } else if checked {
        (
            style.checked_background,
            style.checked_border_color,
            style.mark_color,
            style.label_color,
        )
    } else {
        (
            style.background,
            style.border_color,
            style.unchecked_mark_color,
            style.label_color,
        )
    };

    // v1's mark: this kit's own icon table, sized at the font size times 1.2.
    let mark = MdIcon::new("check")
        .size(style.mark_size)
        .resolve()
        .color(to_paint(mark_ink));

    // v1's label, or `Empty::new()` when there was none. An empty `Text` is the
    // same thing: it paints nothing and asks for no space.
    let label = ctx.label.clone().unwrap_or_default();
    // The accessible name is a second copy: the label itself is moved into the
    // content's slot.
    let accessible = label.clone();

    let content = Stack::new(Axis::Horizontal)
        .gap(style.gap)
        // v1's `.items_center().justify_center()`.
        .align(Align::Center, Align::Center)
        .push(move || Box::new(mark.clone()))
        .push(move || {
            Box::new(
                Text::new(label.clone())
                    .size(style.font_size)
                    .color(to_paint(label_ink)),
            )
        });

    let surface = Surface::new()
        .request(LayoutRequest::new().height(style.height))
        .padding_horiz(f64::from(style.padding))
        .background(to_paint(background))
        .radius(style.radius)
        .border(style.border_width, to_paint(border_color))
        .push(move || Box::new(content.clone()));

    let checked_flag = ctx.checked.clone();
    let unchecked_flag = ctx.unchecked.clone();
    let reported_flag = ctx.checked.clone();
    let on_change = ctx.on_change.clone();

    let mut pressable = Pressable::new(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || Box::new(surface.clone()),
    ))
    // v1's `set_disabled(disabled)`.
    .disabled(disabled)
    // The checked state, reported to assistive technology from the flag.
    .selected_from(move || read_flag(&reported_flag))
    .on_click(move || {
        // v1's toggle: nothing happens while disabled, and the callback is
        // handed the state the button moved to.
        if disabled {
            return;
        }
        let next = !read_flag(&checked_flag);
        write_flag(&checked_flag, next);
        write_flag(&unchecked_flag, !next);
        if let Some(cb) = &on_change {
            cb(next);
        }
    });

    if !accessible.is_empty() {
        pressable = pressable.label(accessible);
    }

    Box::new(pressable)
}
