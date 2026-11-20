//! Fluent confirm-edit, ported to the Floom 2.0 widget API.

use std::sync::{Arc, Mutex};

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
pub use floom_ui_composites::ConfirmEdit;
use floom_ui_composites::{ConfirmEditGlyphs, ConfirmEditStyle};
use floom_widgets::{read_flag, shared_flag, SharedBuffer, SharedFlag};

use crate::widgets::icon::{FluentIcon, VIEW_BOX};
use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::confirm_edit::ConfirmEditProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;

const GLYPH_SIZE: f32 = 16.0;
const BUTTON_PADDING: f32 = 8.0;
const INPUT_WIDTH: f32 = 160.0;
const INPUT_FONT_SIZE: f32 = 14.0;
const INPUT_PADDING_HORIZ: f32 = 8.0;
const INPUT_BORDER: f32 = 1.0;

fn write_buffer(buffer: &SharedBuffer, text: impl Into<String>) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.into();
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentConfirmEditStyle {
    pub design_system: &'static str,
    pub gap: f32,
    pub radius: f32,
    pub icon_size: f32,
    pub glyph_size: f32,
    pub button_padding: f32,
    pub input_width: f32,
    pub input_font_size: f32,
    pub input_padding_horiz: f32,
    pub input_border: f32,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub outline: Color,
}

pub struct FluentConfirmEdit {
    props: ConfirmEditProps,
    theme: RwSignal<MultiTheme>,
    value: SharedBuffer,
    draft: SharedBuffer,
    editing: SharedFlag,
    child: Option<Child>,
    on_save: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
}

impl FluentConfirmEdit {
    pub fn new() -> Self {
        let props = ConfirmEditProps::default();
        Self {
            value: Arc::new(Mutex::new(props.value.clone())),
            draft: Arc::new(Mutex::new(props.value.clone())),
            editing: shared_flag(false),
            child: None,
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            props,
            on_save: None,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        write_buffer(&self.value, self.props.value.clone());
        self
    }

    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    pub fn on_save(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_save = Some(Arc::new(callback));
        self
    }

    pub fn props(&self) -> &ConfirmEditProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    pub fn value_buffer(&self) -> SharedBuffer {
        self.value.clone()
    }

    pub fn draft_buffer(&self) -> SharedBuffer {
        self.draft.clone()
    }

    pub fn editing_flag(&self) -> SharedFlag {
        self.editing.clone()
    }

    pub fn is_editing(&self) -> bool {
        read_flag(&self.editing)
    }

    pub fn resolved_style(&self) -> FluentConfirmEditStyle {
        let theme = self.theme.get();
        let ComponentTokens::ConfirmEdit(tokens) = theme.component(ComponentId::ConfirmEdit) else {
            unreachable!("ComponentId::ConfirmEdit must resolve to ConfirmEditTokens")
        };
        let c = theme.colors();
        FluentConfirmEditStyle {
            design_system: "fluent",
            gap: tokens.gap,
            radius: tokens.radius,
            icon_size: tokens.icon_size,
            glyph_size: GLYPH_SIZE,
            button_padding: BUTTON_PADDING,
            input_width: INPUT_WIDTH,
            input_font_size: INPUT_FONT_SIZE,
            input_padding_horiz: INPUT_PADDING_HORIZ,
            input_border: INPUT_BORDER,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            primary: c.primary,
            outline: c.outline,
        }
    }

    pub fn view(self) -> ConfirmEdit {
        let style = self.resolved_style();
        let mut widget = ConfirmEdit::new()
            .style(ConfirmEditStyle {
                gap: style.gap,
                radius: style.radius,
                icon_size: style.icon_size,
                glyph_size: style.glyph_size,
                button_padding: style.button_padding,
                input_width: style.input_width,
                input_font_size: style.input_font_size,
                input_padding_horiz: style.input_padding_horiz,
                input_border: style.input_border,
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                primary: to_paint(style.primary),
                outline: to_paint(style.outline),
            })
            .buffers(self.value, self.draft)
            .editing(self.editing)
            .child(self.child)
            .glyphs(ConfirmEditGlyphs {
                edit: FluentIcon::new("edit").path_data(),
                check: FluentIcon::new("check").path_data(),
                close: FluentIcon::new("close").path_data(),
                view_box: VIEW_BOX,
            });
        if let Some(on_save) = self.on_save {
            widget = widget.on_save(move |value| on_save(value));
        }
        widget
    }
}

impl Default for FluentConfirmEdit {
    fn default() -> Self {
        Self::new()
    }
}
