use std::rc::Rc;

use floem::event::EventPropagation;
use floem::prelude::*;
use floem::style::CursorStyle;
use floem::view::AnyView;
use floem_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floem_core_theme::props::form::FormProps;
use floem_core_theme::theme::{MultiTheme, Theme};
use floem_core_theme::tokens::color::Color;
use floem_core_theme::tokens::typography::FontWeight;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoFormStyle {
    pub surface: Color,
    pub pad: f32,
    pub gap: f32,
    pub radius: f32,
    pub submit_height: f32,
    pub submit_radius: f32,
}

pub struct CupertinoForm {
    props: FormProps,
    theme: RwSignal<MultiTheme>,
    child: Option<AnyView>,
    /// Consumer-driven validity gate for the submit button.
    valid: RwSignal<bool>,
    /// Consumer-driven form-level error summary (rendered above the button).
    errors: RwSignal<Vec<String>>,
    on_submit: Option<Box<dyn Fn()>>,
}

impl CupertinoForm {
    pub fn new() -> Self {
        Self {
            props: FormProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floem_core_theme::global_theme::DesignSystem::Cupertino)),
            child: None,
            valid: RwSignal::new(true),
            errors: RwSignal::new(Vec::new()),
            on_submit: None,
        }
    }

    /// Supply the form's content (fields, etc.).
    pub fn child(mut self, child: impl IntoView + 'static) -> Self {
        self.child = Some(child.into_any());
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
    pub fn on_submit(mut self, callback: impl Fn() + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    /// Expose the validity signal so the consumer can enable/disable submit.
    pub fn valid_signal(&self) -> RwSignal<bool> {
        self.valid
    }

    /// Expose the form-level error signal so the consumer can show a summary.
    pub fn errors_signal(&self) -> RwSignal<Vec<String>> {
        self.errors
    }

    pub fn props(&self) -> &FormProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn resolved_style(&self) -> CupertinoFormStyle {
        let theme = self.theme.get();
        let ComponentTokens::Form(tokens) = theme.component(ComponentId::Form) else {
            unreachable!("ComponentId::Form must resolve to FormTokens")
        };
        CupertinoFormStyle {
            surface: theme.colors().surface,
            pad: self.props.pad,
            gap: self.props.gap,
            radius: theme.radius().md,
            submit_height: tokens.submit_height,
            submit_radius: tokens.submit_radius,
        }
    }

    /// Build the form surface: a vertical stack of `[content, error summary,
    /// submit button]`. The submit button is disabled (greyed, non-clickable)
    /// whenever `valid` is false.
    pub fn view(self) -> impl View {
        let theme = self.theme;
        let props = self.props;
        let child = self.child.unwrap_or_else(|| Empty::new().into_any());
        let valid = self.valid;
        let errors = self.errors;
        let on_submit: Rc<Option<Box<dyn Fn()>>> = Rc::new(self.on_submit);

        let ComponentTokens::Form(tokens) = theme.get().component(ComponentId::Form) else {
            unreachable!("ComponentId::Form must resolve to FormTokens")
        };
        let submit_height = tokens.submit_height;
        let submit_radius = tokens.submit_radius;
        let pad = props.pad;
        let gap = props.gap;
        let radius = theme.get().radius().md;
        let submit_label = props.submit_label.clone();
        let show_submit = props.show_submit;

        // Reactive form-level error summary (reuses ValidationMessages tokens).
        let error_view = dyn_view(move || {
            let errs = errors.get();
            let theme = theme;
            let ComponentTokens::ValidationMessages(vmt) =
                theme.get().component(ComponentId::ValidationMessages)
            else {
                unreachable!("ComponentId::ValidationMessages must resolve")
            };
            if errs.is_empty() {
                return Empty::new().into_any();
            }
            let items: Vec<AnyView> = errs
                .iter()
                .map(|e| {
                    Label::new(e.clone())
                        .style(move |s| {
                            s.font_size(vmt.font_size)
                                .color(theme.get().colors().error.to_peniko())
                        })
                        .into_any()
                })
                .collect();
            list(items).style(move |s| s.gap(vmt.gap)).into_any()
        });

        // Submit button (rendered only when show_submit is set).
        let submit_btn: AnyView = if show_submit {
            let lbl = submit_label.clone();
            Container::new(
                Label::new(lbl).style(move |s| {
                    let t = theme.get();
                    s.font_size(14.0)
                        .font_weight(FontWeight::Medium.to_floem())
                        .color(t.colors().on_primary.to_peniko())
                }),
            )
            .style(move |s| {
                let t = theme.get();
                let ok = valid.get();
                let mut s = s
                    .height(submit_height)
                    .padding_horiz(24.0)
                    .border_radius(submit_radius)
                    .items_center()
                    .justify_center();
                if ok {
                    s = s
                        .background(t.colors().primary.to_peniko())
                        .cursor(CursorStyle::Pointer);
                } else {
                    s = s
                        .background(t.colors().surface_variant.to_peniko())
                        .color(t.colors().on_surface_variant.to_peniko());
                }
                s
            })
            .on_event(listener::Click, move |_, _| {
                if valid.get() {
                    if let Some(cb) = on_submit.as_ref() {
                        cb();
                    }
                }
                EventPropagation::Continue
            })
            .into_any()
        } else {
            Empty::new().into_any()
        };

        let content = Stack::vertical((child, error_view.into_any(), submit_btn))
            .style(move |s| s.gap(gap).items_start().width_pct(100.0));

        Container::new(content).style(move |s| {
            let t = theme.get();
            s.width_pct(100.0)
                .padding(pad)
                .border_radius(radius)
                .background(t.colors().surface.to_peniko())
        })
    }
}

impl Default for CupertinoForm {
    fn default() -> Self {
        Self::new()
    }
}
