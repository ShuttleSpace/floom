use floom::prelude::*;
use floom::style::CursorStyle;
use floom::views::{Decorators, Label, Stack};
use floom::reactive::{RwSignal, SignalGet};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::file_input::FileInputProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color_ext::*;

/// Material 3 file input: a trigger affordance plus a readout of the
/// currently selected file path. The native picker is host-provided — the
/// trigger fires `on_change` with the current path so the consumer can open a
/// real dialog, then write the result back through `path_signal()`.
pub struct CupertinoFileInput {
    props: FileInputProps,
    theme: RwSignal<MultiTheme>,
    path: RwSignal<Option<String>>,
}

impl CupertinoFileInput {
    pub fn new() -> Self {
        Self {
            props: FileInputProps::default(),
            theme: RwSignal::new(MultiTheme::new_for(floom_core_theme::global_theme::DesignSystem::Cupertino)),
            path: RwSignal::new(None),
        }
    }

    /// Trigger label (defaults to "Upload").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.props.label = label.into();
        self
    }

    /// Comma-separated MIME / extension hint passed to the host picker.
    pub fn accept(mut self, accept: impl Into<String>) -> Self {
        self.props.accept = accept.into();
        self
    }

    /// Allow multiple files (stored for the host picker; the readout shows the
    /// first selected path).
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.props.multiple = multiple;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.props.disabled = disabled;
        self
    }

    /// Fires when the user clicks the trigger; the current path is passed so
    /// the host can open a native picker and write the result via
    /// `path_signal()`.
    pub fn on_change(mut self, callback: impl Fn(Option<String>) + 'static) -> Self {
        self.props.on_change = Some(Box::new(callback));
        self
    }

    pub fn props(&self) -> &FileInputProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    /// Reactive source of truth for the selected path; the host writes here
    /// after a real picker returns a value.
    pub fn path_signal(&self) -> RwSignal<Option<String>> {
        self.path
    }

    pub fn view(self) -> impl View {
        let theme = self.theme;
        let disabled = RwSignal::new(self.props.disabled);
        let on_change = self.props.on_change;
        let path = self.path;
        let label_text = self.props.label.clone();

        let tokens = match theme.get().component(ComponentId::FileInput) {
            ComponentTokens::FileInput(t) => t,
            _ => unreachable!(),
        };
        let height = tokens.height;
        let radius = tokens.border_radius;
        let pad_h = tokens.padding_horizontal;
        let font_size = tokens.font_size;

        let trigger = Stack::horizontal((
            Label::new(label_text).style(move |s| {
                let t = theme.get();
                s.color(t.colors().primary.to_peniko())
                    .font_size(font_size)
                    .font_weight(t.typography().label.large.font_weight.to_floom())
            }),
            Label::derived(move || {
                path.get()
                    .clone()
                    .unwrap_or_else(|| "No file chosen".to_string())
            })
            .style(move |s| {
                let t = theme.get();
                let c = if path.get().is_some() {
                    t.colors().on_surface
                } else {
                    t.colors().on_surface_variant
                };
                s.color(c.to_peniko()).font_size(font_size)
            }),
        ))
        .style(move |s| s.gap(12.0).items_center().justify_between().width_pct(100.0));

        Container::new(trigger)
            .style(move |s| {
                let t = theme.get();
                let border = if disabled.get() {
                    with_alpha(t.colors().on_surface, 38)
                } else {
                    t.colors().outline
                };
                let bg = if disabled.get() {
                    with_alpha(t.colors().on_surface, 4)
                } else {
                    t.colors().surface
                };
                s.height(height)
                    .padding_horiz(pad_h)
                    .border(1.0)
                    .border_color(border.to_peniko())
                    .border_radius(radius)
                    .background(bg.to_peniko())
                    .items_center()
                    .cursor(CursorStyle::Pointer)
                    .set_disabled(disabled.get())
            })
            .action(move || {
                if !disabled.get() {
                    if let Some(cb) = &on_change {
                        cb(path.get());
                    }
                }
            })
    }
}

impl Default for CupertinoFileInput {
    fn default() -> Self {
        Self::new()
    }
}
