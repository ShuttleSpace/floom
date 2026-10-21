//! Fluent form, ported to the Floom 2.0 widget API.
//!
//! A form is a grouping surface: the caller's own fields, an optional summary
//! of the errors, and a submit button that only answers while the form is
//! valid.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a [`Surface`] for the form's surface, a vertical
//! `Stack` for the column inside it, `Text` for the error messages, a
//! [`Pressable`] for the submit button, and the engine's [`Conditional`] to
//! choose between the button's two appearances. There is nothing here that a
//! widget could own, so the shape stays in the kit.
//!
//! ## What this kit answers
//!
//! * the form surface's fill: `surface`; its radius: the theme's `radius().md`;
//! * the padding inside the surface and the gap between the stacked children:
//!   the **props**, not the tokens -- see the dead tokens below;
//! * the submit button's height and radius: the form token; its fill:
//!   `primary` while the form is valid and `surface_variant` while it is not;
//! * the error messages' size and the gap between them: the validation
//!   messages' token; their ink: `error`;
//! * the submit label's size (14), its weight (medium) and the button's
//!   horizontal padding (24): v1 literals.
//!
//! ## The content slot
//!
//! v1's `child(..)` took an `impl IntoView`; 2.0's takes a [`Child`], because a
//! 2.0 widget is mounted by its parent rather than returned. The content is
//! otherwise the caller's own, in the same place.
//!
//! ## The validity gate
//!
//! v1's `valid_signal()` returned an `RwSignal<bool>`. 2.0's slot closures must
//! be `Send + Sync` and an `RwSignal` is `Rc`-based, so the gate is the
//! engine's [`SharedFlag`]. The button has two appearances and the engine's
//! `Conditional` follows one flag, so the kit keeps **a pair** -- the flag and
//! its inverse -- and writes them together through
//! [`FluentForm::set_valid`]. That is the only writer, so the two cannot
//! drift apart.
//!
//! ## The errors
//!
//! v1's `errors_signal()` returned an `RwSignal<Vec<String>>`. The port's
//! errors are the engine's [`SharedMap`], keyed by the message's index, so
//! that a caller can write them from outside the tree; [`FluentForm::set_errors`]
//! and [`FluentForm::errors`] are the readable way in and out.
//!
//! The summary is the errors **as they were when the kit was built**: 2.0's
//! tree is mounted once, so a host that changes the errors afterwards has to
//! rebuild the kit. v1's summary was a `dyn_view` over the signal and followed
//! it.
//!
//! ## The style the port adds
//!
//! v1's `FluentFormStyle` reported the surface, the padding, the gap, the radius
//! and the submit button's height and radius -- and no colours at all, though
//! the paint used six. The port reports them, and it reports the submit
//! button's label ink, which the paint used. The port also adds a
//! `design_system` field, which v1's form style did not have: the other three
//! families' styles name theirs, and a style that does not say which design
//! system it belongs to cannot be told apart from its siblings.
//!
//! ## The dead tokens
//!
//! v1's `FormTokens` carry a `pad` and a `gap`, and v1's view read **neither**:
//! it read the *props*' `pad` and `gap`, which `FormProps` defaults to the same
//! two numbers. The port reports the props' numbers, as v1 did, and the
//! tokens' are named here so they are not mistaken for a second answer.
//!
//! ## The invalid submit button's label
//!
//! v1 wrote the invalid button's colour on the **container**, where the label's
//! own `on_primary` overrode it -- so the invalid button's label was
//! `on_primary` on a `surface_variant` fill, which is very nearly invisible.
//! The port reproduces that, because a port that quietly improves a design is
//! a port that changes the design. The colour v1 wrote and never showed is
//! still in the style, as
//! [`FluentFormStyle::invalid_submit_ink`], so the trap cannot re-arm itself.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **`cursor(CursorStyle::Pointer)`** on the valid submit button. 2.0's paint
//!   context has no cursor vocabulary. The button is still a `Pressable`, so it
//!   is still activatable by keyboard, which the pointer was not.
//! * **The accessors.** v1's form had no `with_theme()`. The port gives all
//!   three kits the same set, because a kit that cannot be given a theme
//!   cannot be tested against one.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::text_input::{SharedFlag, SharedMap, read_flag, shared_flag, write_flag};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::form::FormProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_core_theme::tokens::color::to_engine_color as to_paint;

/// A callback in the shape 2.0's slots need.
type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

/// The submit label's size. A v1 literal.
const SUBMIT_FONT_SIZE: f32 = 14.0;
/// The submit button's horizontal padding. A v1 literal.
const SUBMIT_PADDING_HORIZONTAL: f32 = 24.0;

/// The form's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFormStyle {
    pub design_system: &'static str,
    /// The form surface's fill.
    pub surface: Color,
    /// The padding inside the surface, from the props.
    pub pad: f32,
    /// The gap between the stacked children, from the props.
    pub gap: f32,
    /// The form surface's radius, from the theme's radius scale.
    pub radius: f32,
    /// The submit button's height, from the token.
    pub submit_height: f32,
    /// The submit button's radius, from the token.
    pub submit_radius: f32,
    /// The submit button's fill, while the form is valid.
    pub submit_background: Color,
    /// The submit button's fill, while the form is not valid.
    pub invalid_submit_background: Color,
    /// The submit label's ink.
    ///
    /// v1 wrote `on_primary` on the label in both cases, so this is the label's
    /// ink either way.
    pub submit_ink: Color,
    /// The colour v1 wrote on the invalid button's *container*, where the
    /// label's own ink overrode it. It never showed.
    pub invalid_submit_ink: Color,
    /// The submit label's size. A v1 literal.
    pub submit_font_size: f32,
    /// The submit label's weight. v1 wrote medium.
    pub submit_font_weight: FontWeight,
    /// The submit button's horizontal padding. A v1 literal.
    pub submit_padding_horizontal: f32,
    /// The error messages' size, from the validation messages' token.
    pub error_font_size: f32,
    /// The gap between the error messages, from the validation messages' token.
    pub error_gap: f32,
    /// The error messages' ink.
    pub error_color: Color,
}

/// Fluent's form.
#[derive(Clone)]
pub struct FluentForm {
    props: FormProps,
    theme: RwSignal<MultiTheme>,
    /// The caller's own content.
    child: Option<Child>,
    /// The validity gate, as the engine's flag.
    valid: SharedFlag,
    /// The same gate, inverted.
    ///
    /// The engine's `Conditional` follows one flag and the submit button has
    /// two appearances, so the kit keeps a pair and writes them together.
    invalid: SharedFlag,
    /// The error messages, shared with whoever writes them.
    errors: SharedMap,
    /// The submit callback in the shape 2.0's slots need.
    ///
    /// `Send + Sync` is required, not incidental: 2.0 rebuilds a widget inside a
    /// `Child` closure the host may hand between threads, so the callback is
    /// shared as an `Arc`. v1's callback was a `Box<dyn Fn()>`, which cannot
    /// cross that boundary.
    on_submit: Option<Callback>,
}

impl FluentForm {
    pub fn new() -> Self {
        Self {
            props: FormProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            child: None,
            valid: shared_flag(true),
            invalid: shared_flag(false),
            errors: floom_widgets::shared_map(Default::default()),
            on_submit: None,
        }
    }

    /// Supply the form's content (fields, etc.).
    ///
    /// v1's `child(impl IntoView)`. 2.0 takes a [`Child`], because a 2.0 widget
    /// is mounted by its parent rather than returned.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Label shown on the submit button.
    pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
        self.props.submit_label = submit_label.into();
        self
    }

    /// Whether to render the submit button (default true).
    pub fn show_submit(mut self, show_submit: bool) -> Self {
        self.props.show_submit = show_submit;
        self
    }

    /// Callback fired when the submit button is clicked while valid.
    pub fn on_submit(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_submit = Some(Arc::new(callback));
        self
    }

    /// Whether the form is valid right now.
    pub fn is_valid(&self) -> bool {
        read_flag(&self.valid)
    }

    /// Set the validity.
    ///
    /// The only writer of the pair of flags: the kit keeps them in step, so the
    /// button cannot look one way while the gate says another.
    pub fn set_valid(&self, valid: bool) {
        write_flag(&self.valid, valid);
        write_flag(&self.invalid, !valid);
    }

    /// The shared error map, for a caller that writes the messages directly.
    ///
    /// v1's `errors_signal()`. The type changed because the type had to: a 2.0
    /// slot closure cannot hold an `RwSignal`. The keys are the messages'
    /// indices, written as decimal strings.
    pub fn errors_map(&self) -> SharedMap {
        self.errors.clone()
    }

    /// Write the error messages, replacing whatever was there.
    pub fn set_errors<S: Into<String>>(&self, errors: impl IntoIterator<Item = S>) {
        let mut guard = match self.errors.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };
        guard.clear();
        for (index, message) in errors.into_iter().enumerate() {
            guard.insert(index.to_string(), message.into());
        }
    }

    /// The error messages, in the order they were written.
    pub fn errors(&self) -> Vec<String> {
        let guard = match self.errors.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let mut indexed: Vec<(u64, String)> = guard
            .iter()
            .filter_map(|(key, message)| {
                key.parse::<u64>().ok().map(|index| (index, message.clone()))
            })
            .collect();
        indexed.sort_by_key(|(index, _)| *index);
        indexed.into_iter().map(|(_, message)| message).collect()
    }

    pub fn props(&self) -> &FormProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn resolved_style(&self) -> FluentFormStyle {
        let theme = self.theme.get();
        let ComponentTokens::Form(tokens) = theme.component(ComponentId::Form) else {
            unreachable!("ComponentId::Form must resolve to FormTokens")
        };
        let ComponentTokens::ValidationMessages(vmt) =
            theme.component(ComponentId::ValidationMessages)
        else {
            unreachable!("ComponentId::ValidationMessages must resolve to ValidationMessagesTokens")
        };
        let c = theme.colors();
        FluentFormStyle {
            design_system: "fluent",
            surface: c.surface,
            // v1's style read the *props*, not the form token.
            pad: self.props.pad,
            gap: self.props.gap,
            radius: theme.radius().md,
            submit_height: tokens.submit_height,
            submit_radius: tokens.submit_radius,
            submit_background: c.primary,
            invalid_submit_background: c.surface_variant,
            // v1's label was always `on_primary`.
            submit_ink: c.on_primary,
            // v1 wrote this on the container, where the label's own ink won.
            invalid_submit_ink: c.on_surface_variant,
            submit_font_size: SUBMIT_FONT_SIZE,
            submit_font_weight: FontWeight::Medium,
            submit_padding_horizontal: SUBMIT_PADDING_HORIZONTAL,
            error_font_size: vmt.font_size,
            error_gap: vmt.gap,
            error_color: c.error,
        }
    }

    /// The form: the surface, the column, and the three things in it.
    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let gap = style.gap;
        let pad = style.pad;

        // v1's error summary: a list of messages, or `Empty::new()` when there
        // were none. The slot was always there, so the gap always counted.
        let error_font_size = style.error_font_size;
        let error_gap = style.error_gap;
        let error_color = to_paint(style.error_color);
        let mut summary = Stack::new(Axis::Vertical).gap(error_gap);
        for message in self.errors() {
            let message = message.clone();
            summary = summary.push(move || {
                Box::new(
                    Text::new(message.clone())
                        .size(error_font_size)
                        .color(error_color),
                )
            });
        }

        let submit_ctx = SubmitContext {
            style,
            label: self.props.submit_label.clone(),
            on_submit: self.on_submit.clone(),
            valid: self.valid.clone(),
            invalid: self.invalid.clone(),
        };
        let show_submit = self.props.show_submit;

        // v1's column: the caller's content, the error summary, the submit
        // button.
        let mut content = Stack::new(Axis::Vertical)
            .gap(gap)
            // v1's `.items_start()`.
            .align(Align::Start, Align::Start)
            .request(LayoutRequest::new().width_pct(100.0));

        match self.child {
            // v1's `Empty::new()` when there was no child.
            Some(child) => content = content.child(child),
            None => content = content.push(|| Box::new(Stack::new(Axis::Horizontal))),
        }

        content = content.push(move || Box::new(summary.clone()));
        content = content.push(move || build_submit(show_submit, submit_ctx.clone()));

        Surface::new()
            .request(LayoutRequest::new().width_pct(100.0))
            .padding_horiz(f64::from(pad))
            .padding_vert(f64::from(pad))
            .background(to_paint(style.surface))
            .radius(style.radius)
            .push(move || Box::new(content.clone()))
    }
}

impl Default for FluentForm {
    fn default() -> Self {
        Self::new()
    }
}

/// Everything the submit button needs that is not in the style.
#[derive(Clone)]
struct SubmitContext {
    style: FluentFormStyle,
    label: String,
    on_submit: Option<Callback>,
    valid: SharedFlag,
    invalid: SharedFlag,
}

/// The submit slot: the button, or v1's empty leaf when it is not shown.
fn build_submit(show_submit: bool, ctx: SubmitContext) -> Box<dyn floom_widgets::WidgetAny> {
    if !show_submit {
        // v1's `Empty::new()`: the slot was still there, so the gap still
        // counted.
        return Box::new(Stack::new(Axis::Horizontal));
    }
    let valid_flag = ctx.valid.clone();
    let invalid_flag = ctx.invalid.clone();
    Box::new(
        Stack::new(Axis::Horizontal)
            .push({
                let ctx = ctx.clone();
                let flag = valid_flag.clone();
                move || Box::new(appearance(flag.clone(), ctx.clone(), true))
            })
            .push({
                let ctx = ctx.clone();
                let flag = invalid_flag.clone();
                move || Box::new(appearance(flag.clone(), ctx.clone(), false))
            }),
    )
}

/// The conditional that shows the submit button in one of its two appearances.
fn appearance(flag: SharedFlag, ctx: SubmitContext, valid: bool) -> Conditional {
    Conditional::new(false)
        .when_flag(flag)
        .show(Child::new(ChildKey::index(0), LayoutRequest::new(), move || {
            build_button(&ctx, valid)
        }))
}

/// The submit button itself.
fn build_button(ctx: &SubmitContext, valid: bool) -> Box<dyn floom_widgets::WidgetAny> {
    let style = ctx.style;
    // v1's two fills. The label's ink is `on_primary` in both cases: v1 set
    // the invalid colour on the container, where the label's own colour won.
    let background = if valid {
        style.submit_background
    } else {
        style.invalid_submit_background
    };

    let label = ctx.label.clone();
    let content = Stack::new(Axis::Horizontal)
        // v1's `.items_center().justify_center()`.
        .align(Align::Center, Align::Center)
        .push(move || {
            Box::new(
                Text::new(label.clone())
                    .size(style.submit_font_size)
                    .weight(style.submit_font_weight)
                    .color(to_paint(style.submit_ink)),
            )
        });

    let surface = Surface::new()
        .request(LayoutRequest::new().height(style.submit_height))
        .padding_horiz(f64::from(style.submit_padding_horizontal))
        .background(to_paint(background))
        .radius(style.submit_radius)
        .push(move || Box::new(content.clone()));

    let valid_flag = ctx.valid.clone();
    let on_submit = ctx.on_submit.clone();

    Box::new(
        Pressable::new(Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(surface.clone()),
        ))
        .on_click(move || {
            // v1's guard: the callback fires only while the form is valid.
            if !read_flag(&valid_flag) {
                return;
            }
            if let Some(cb) = &on_submit {
                cb();
            }
        }),
    )
}

