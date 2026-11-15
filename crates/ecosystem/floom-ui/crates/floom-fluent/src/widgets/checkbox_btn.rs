//! Fluent checkbox button, ported to the Floom 2.0 widget API.

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
use crate::widgets::icon::FluentIcon;

type Callback = Arc<dyn Fn(bool) + Send + Sync + 'static>;

const BORDER_WIDTH: f32 = 1.0;
const MARK_SCALE: f32 = 1.2;
const DISABLED_FILL_ALPHA: u8 = 12;
const DISABLED_ALPHA: u8 = 38;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentCheckboxBtnStyle {
    pub design_system: &'static str,
    pub background: Color,
    pub border_color: Color,
    pub checked_background: Color,
    pub checked_border_color: Color,
    pub disabled_background: Color,
    pub disabled_border_color: Color,
    pub mark_color: Color,
    pub disabled_mark_color: Color,
    pub unchecked_mark_color: Color,
    pub unchecked_disabled_mark_color: Color,
    pub label_color: Color,
    pub disabled_label_color: Color,
    pub height: f32,
    pub padding: f32,
    pub radius: f32,
    pub gap: f32,
    pub font_size: f32,
    pub mark_size: f32,
    pub border_width: f32,
}

#[derive(Clone)]
pub struct FluentCheckboxBtn {
    props: CheckboxBtnProps,
    theme: RwSignal<MultiTheme>,
    checked: SharedFlag,
    unchecked: SharedFlag,
    on_change: Option<Callback>,
}

impl FluentCheckboxBtn {
    pub fn new() -> Self {
        Self {
            props: CheckboxBtnProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            checked: shared_flag(false),
            unchecked: shared_flag(true),
            on_change: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
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

    pub fn on_change(mut self, callback: impl Fn(bool) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// that as an `Arc` of its own (see the field's note), and
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

    pub fn is_checked(&self) -> bool {
        read_flag(&self.checked)
    }

    pub fn set_checked(&self, checked: bool) {
        write_flag(&self.checked, checked);
        write_flag(&self.unchecked, !checked);
    }

    pub fn resolved_style(&self) -> FluentCheckboxBtnStyle {
        let theme = self.theme.get();
        let ComponentTokens::CheckboxBtn(tokens) = theme.component(ComponentId::CheckboxBtn) else {
            unreachable!("ComponentId::CheckboxBtn must resolve to CheckboxBtnTokens")
        };
        let c = theme.colors();
        FluentCheckboxBtnStyle {
            design_system: "fluent",
            background: c.surface_variant,
            border_color: c.outline,
            checked_background: c.primary,
            checked_border_color: c.primary,
            disabled_background: with_alpha(c.on_surface, DISABLED_FILL_ALPHA),
            disabled_border_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            mark_color: c.on_primary,
            disabled_mark_color: with_alpha(c.on_surface, DISABLED_ALPHA),
            unchecked_mark_color: Color::rgba(0, 0, 0, 0),
            unchecked_disabled_mark_color: with_alpha(c.on_surface, DISABLED_ALPHA),
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

impl Default for FluentCheckboxBtn {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
struct PillContext {
    style: FluentCheckboxBtnStyle,
    disabled: bool,
    label: Option<String>,
    on_change: Option<Callback>,
    checked: SharedFlag,
    unchecked: SharedFlag,
}

fn appearance(flag: SharedFlag, ctx: PillContext, checked: bool) -> Conditional {
    Conditional::new(false).when_flag(flag).show(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || build_pill(&ctx, checked),
    ))
}

fn build_pill(ctx: &PillContext, checked: bool) -> Box<dyn floom_widgets::WidgetAny> {
    let style = ctx.style;
    let disabled = ctx.disabled;

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

    let mark = FluentIcon::new("check")
        .size(style.mark_size)
        .resolve()
        .color(to_paint(mark_ink));

    let label = ctx.label.clone().unwrap_or_default();
    let accessible = label.clone();

    let content = Stack::new(Axis::Horizontal)
        .gap(style.gap)
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
    .disabled(disabled)
    .selected_from(move || read_flag(&reported_flag))
    .on_click(move || {
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
