//! Fluent form, ported to the Floom 2.0 widget API.

use std::sync::Arc;

use floom_core::{Child, ChildKey, LayoutRequest};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight};
use floom_widgets::text_input::{read_flag, shared_flag, write_flag, SharedFlag, SharedMap};
use floom_widgets::{Align, Axis, Conditional, Pressable, Stack, Surface, Text};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::form::FormProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

type Callback = Arc<dyn Fn() + Send + Sync + 'static>;

const SUBMIT_FONT_SIZE: f32 = 14.0;
const SUBMIT_PADDING_HORIZONTAL: f32 = 24.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentFormStyle {
    pub design_system: &'static str,
    pub surface: Color,
    pub pad: f32,
    pub gap: f32,
    pub radius: f32,
    pub submit_height: f32,
    pub submit_radius: f32,
    pub submit_background: Color,
    pub invalid_submit_background: Color,
    pub submit_ink: Color,
    pub invalid_submit_ink: Color,
    pub submit_font_size: f32,
    pub submit_font_weight: FontWeight,
    pub submit_padding_horizontal: f32,
    pub error_font_size: f32,
    pub error_gap: f32,
    pub error_color: Color,
}

#[derive(Clone)]
pub struct FluentForm {
    props: FormProps,
    theme: RwSignal<MultiTheme>,
    child: Option<Child>,
    valid: SharedFlag,
    invalid: SharedFlag,
    errors: SharedMap,
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

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn submit_label(mut self, submit_label: impl Into<String>) -> Self {
        self.props.submit_label = submit_label.into();
        self
    }

    pub fn show_submit(mut self, show_submit: bool) -> Self {
        self.props.show_submit = show_submit;
        self
    }

    pub fn on_submit(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_submit = Some(Arc::new(callback));
        self
    }

    pub fn is_valid(&self) -> bool {
        read_flag(&self.valid)
    }

    pub fn set_valid(&self, valid: bool) {
        write_flag(&self.valid, valid);
        write_flag(&self.invalid, !valid);
    }

    pub fn errors_map(&self) -> SharedMap {
        self.errors.clone()
    }

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

    pub fn errors(&self) -> Vec<String> {
        let guard = match self.errors.lock() {
            Ok(guard) => guard,
            Err(_) => return Vec::new(),
        };
        let mut indexed: Vec<(u64, String)> = guard
            .iter()
            .filter_map(|(key, message)| {
                key.parse::<u64>()
                    .ok()
                    .map(|index| (index, message.clone()))
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
            pad: self.props.pad,
            gap: self.props.gap,
            radius: theme.radius().md,
            submit_height: tokens.submit_height,
            submit_radius: tokens.submit_radius,
            submit_background: c.primary,
            invalid_submit_background: c.surface_variant,
            submit_ink: c.on_primary,
            invalid_submit_ink: c.on_surface_variant,
            submit_font_size: SUBMIT_FONT_SIZE,
            submit_font_weight: FontWeight::Medium,
            submit_padding_horizontal: SUBMIT_PADDING_HORIZONTAL,
            error_font_size: vmt.font_size,
            error_gap: vmt.gap,
            error_color: c.error,
        }
    }

    pub fn view(self) -> Surface {
        let style = self.resolved_style();
        let gap = style.gap;
        let pad = style.pad;

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

        let mut content = Stack::new(Axis::Vertical)
            .gap(gap)
            .align(Align::Start, Align::Start)
            .request(LayoutRequest::new().width_pct(100.0));

        match self.child {
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

#[derive(Clone)]
struct SubmitContext {
    style: FluentFormStyle,
    label: String,
    on_submit: Option<Callback>,
    valid: SharedFlag,
    invalid: SharedFlag,
}

fn build_submit(show_submit: bool, ctx: SubmitContext) -> Box<dyn floom_widgets::WidgetAny> {
    if !show_submit {
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

fn appearance(flag: SharedFlag, ctx: SubmitContext, valid: bool) -> Conditional {
    Conditional::new(false).when_flag(flag).show(Child::new(
        ChildKey::index(0),
        LayoutRequest::new(),
        move || build_button(&ctx, valid),
    ))
}

fn build_button(ctx: &SubmitContext, valid: bool) -> Box<dyn floom_widgets::WidgetAny> {
    let style = ctx.style;
    let background = if valid {
        style.submit_background
    } else {
        style.invalid_submit_background
    };

    let label = ctx.label.clone();
    let content = Stack::new(Axis::Horizontal)
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
            if !read_flag(&valid_flag) {
                return;
            }
            if let Some(cb) = &on_submit {
                cb();
            }
        }),
    )
}
