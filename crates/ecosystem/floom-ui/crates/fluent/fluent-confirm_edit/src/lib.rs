use std::rc::Rc;

use floom::event::EventPropagation;
use floom::prelude::*;
use floom_widget_fluent_icon::fluent_icon_svg;
use floom::reactive::{RwSignal, SignalGet, SignalUpdate};
use floom::style::CursorStyle;
use floom::view::AnyView;
use floom::views::{Container, Decorators, TextInput, Stack, Label};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::confirm_edit::ConfirmEditProps;
use floom_core_theme::theme::{MultiTheme, Theme};

/// Material 3 click-to-edit wrapper (Vuetify `VConfirmEdit`).
///
/// Shows `child` (or the `value` text) in display mode; clicking the edit
/// affordance swaps in a `TextInput` plus save (✓) / cancel (✗) buttons. Save
/// fires `on_save` with the edited string.
pub struct FluentConfirmEdit {
    props: ConfirmEditProps,
    child: Option<Box<dyn Fn() -> AnyView>>,
    theme: RwSignal<MultiTheme>,
}

impl FluentConfirmEdit {
    pub fn new() -> Self {
        Self {
            props: ConfirmEditProps::default(),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Fluent)),
        }
    }

    pub fn child(mut self, child: impl Fn() -> AnyView + 'static) -> Self {
        self.child = Some(Box::new(child));
        self
    }

    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.props.value = v.into();
        self
    }

    pub fn on_save(mut self, callback: impl Fn(String) + 'static) -> Self {
        self.props.on_save = Some(Box::new(callback));
        self
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let on_save = Rc::new(self.props.on_save);
        let value: RwSignal<String> = RwSignal::new(self.props.value.clone());
        let editing: RwSignal<bool> = RwSignal::new(false);
        let draft: RwSignal<String> = RwSignal::new(self.props.value.clone());
        let child_factory = self.child;

        let tokens = match theme.get().component(ComponentId::ConfirmEdit) {
            ComponentTokens::ConfirmEdit(t) => t,
            _ => unreachable!(),
        };
        let tk = tokens;

        dyn_view(move || {
            if editing.get() {
                let input = TextInput::new(draft).style(move |s| {
                    let t = theme.get();
                    s.width(160.0)
                        .font_size(14.0)
                        .color(t.colors().on_surface.to_peniko())
                        .padding_horiz(8.0)
                        .border(1.0)
                        .border_color(t.colors().outline.to_peniko())
                        .border_radius(tk.radius)
                });
                let save = {
                    let value = value;
                    let editing = editing;
                    let on_save = on_save.clone();
                    let draft = draft;
                    Container::new(svg(move || fluent_icon_svg("check")).style(move |s| { let t = theme.get(); s.size(16.0, 16.0).color(t.colors().primary.to_peniko()) }))
                        .style(move |s| {
                            let t = theme.get();
                            s.padding_horiz(8.0)
                                .color(t.colors().primary.to_peniko())
                                .cursor(CursorStyle::Pointer)
                        })
                        .on_event(listener::Click, move |_, _| {
                            let nv = draft.get();
                            value.set(nv.clone());
                            if let Some(cb) = on_save.as_ref() {
                                cb(nv);
                            }
                            editing.set(false);
                            EventPropagation::Continue
                        })
                };
                let cancel = {
                    let editing = editing;
                    let draft = draft;
                    let value = value;
                    Container::new(svg(move || fluent_icon_svg("close")).style(move |s| { let t = theme.get(); s.size(16.0, 16.0).color(t.colors().on_surface_variant.to_peniko()) }))
                        .style(move |s| {
                            let t = theme.get();
                            s.padding_horiz(8.0)
                                .color(t.colors().on_surface_variant.to_peniko())
                                .cursor(CursorStyle::Pointer)
                        })
                        .on_event(listener::Click, move |_, _| {
                            draft.set(value.get());
                            editing.set(false);
                            EventPropagation::Continue
                        })
                };
                Stack::horizontal((input, save, cancel))
                    .style(move |s| s.gap(tk.gap).items_center())
                    .into_any()
            } else {
                let content: AnyView = match &child_factory {
                    Some(f) => f(),
                    None => Label::new(value.get()).into_any(),
                };
                let edit_btn = {
                    let editing = editing;
                    let draft = draft;
                    let value = value;
                    Container::new(svg(move || fluent_icon_svg("edit")).style(move |s| { let t = theme.get(); s.size(16.0, 16.0).color(t.colors().on_surface_variant.to_peniko()) }))
                        .style(move |s| {
                            let t = theme.get();
                            s.padding_horiz(8.0)
                                .color(t.colors().on_surface_variant.to_peniko())
                                .cursor(CursorStyle::Pointer)
                        })
                        .on_event(listener::Click, move |_, _| {
                            draft.set(value.get());
                            editing.set(true);
                            EventPropagation::Continue
                        })
                };
                Stack::horizontal((content, edit_btn))
                    .style(move |s| s.gap(tk.gap).items_center())
                    .into_any()
            }
        })
    }
}

impl Default for FluentConfirmEdit {
    fn default() -> Self {
        Self::new()
    }
}
