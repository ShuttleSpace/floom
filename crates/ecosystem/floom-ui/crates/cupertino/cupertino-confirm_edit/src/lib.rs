//! Cupertino confirm-edit, ported to the Floom 2.0 widget API.
//!

use std::sync::{Arc, Mutex};

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
pub use floom_ui_composites::ConfirmEdit;
use floom_ui_composites::{ConfirmEditGlyphs, ConfirmEditStyle};
use floom_widgets::{read_flag, shared_flag, SharedBuffer, SharedFlag};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::confirm_edit::ConfirmEditProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_widget_cupertino_icon::{CupertinoIcon, VIEW_BOX};

/// The glyphs' size. A v1 literal -- the token's `icon_size` (20) never showed.
const GLYPH_SIZE: f32 = 16.0;
/// The padding around a glyph. A v1 literal.
const BUTTON_PADDING: f32 = 8.0;
/// The field's width. A v1 literal.
const INPUT_WIDTH: f32 = 160.0;
/// The field's font size. A v1 literal.
const INPUT_FONT_SIZE: f32 = 14.0;
/// The field's horizontal padding. A v1 literal.
const INPUT_PADDING_HORIZ: f32 = 8.0;
/// The field's outline width. A v1 literal.
const INPUT_BORDER: f32 = 1.0;

/// Write a shared buffer.
fn write_buffer(buffer: &SharedBuffer, text: impl Into<String>) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.into();
    }
}

/// The confirm-edit's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CupertinoConfirmEditStyle {
    pub design_system: &'static str,
    /// The gap between the parts of a row. The token's `gap`.
    pub gap: f32,
    /// The field's radius. The token's `radius`.
    pub radius: f32,
    /// The token's icon size, which v1 never used: the glyphs were 16.
    pub icon_size: f32,
    /// The glyphs' size. A v1 literal.
    pub glyph_size: f32,
    /// The padding around a glyph. A v1 literal.
    pub button_padding: f32,
    /// The field's width. A v1 literal.
    pub input_width: f32,
    /// The field's font size. A v1 literal.
    pub input_font_size: f32,
    /// The field's horizontal padding. A v1 literal.
    pub input_padding_horiz: f32,
    /// The field's outline width. A v1 literal.
    pub input_border: f32,
    /// The field's ink.
    pub on_surface: Color,
    /// The cross's and the edit affordance's ink.
    pub on_surface_variant: Color,
    /// The tick's ink.
    pub primary: Color,
    /// The field's outline.
    pub outline: Color,
}

/// Cupertino's confirm-edit.
pub struct CupertinoConfirmEdit {
    props: ConfirmEditProps,
    theme: RwSignal<MultiTheme>,
    /// The committed value.
    value: SharedBuffer,
    /// The text being edited.
    draft: SharedBuffer,
    /// Whether the field is in edit mode.
    editing: SharedFlag,
    /// The caller's display content.
    child: Option<Child>,
    /// The save callback, in the shape a 2.0 slot needs.
    on_save: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
}

impl CupertinoConfirmEdit {
    pub fn new() -> Self {
        let props = ConfirmEditProps::default();
        Self {
            value: Arc::new(Mutex::new(props.value.clone())),
            draft: Arc::new(Mutex::new(props.value.clone())),
            editing: shared_flag(false),
            child: None,
            // v1's Cupertino confirm-edit took a *snapshot* of its own design system,
            // so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
            )),
            props,
            on_save: None,
        }
    }

    /// The value being edited.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.props.value = value.into();
        write_buffer(&self.value, self.props.value.clone());
        self
    }

    /// What the display mode shows instead of the value.
    pub fn child(mut self, child: Child) -> Self {
        self.child = Some(child);
        self
    }

    /// Fired with the new value when the edit is confirmed.
    ///
    /// `Send + Sync`, where v1's was not: the callback reaches a 2.0 slot.
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

    /// The committed value's buffer.
    pub fn value_buffer(&self) -> SharedBuffer {
        self.value.clone()
    }

    /// The draft's buffer -- what the field is editing.
    pub fn draft_buffer(&self) -> SharedBuffer {
        self.draft.clone()
    }

    /// The "editing" flag, so an external trigger can open the field.
    pub fn editing_flag(&self) -> SharedFlag {
        self.editing.clone()
    }

    /// Whether the field is in edit mode.
    pub fn is_editing(&self) -> bool {
        read_flag(&self.editing)
    }

    pub fn resolved_style(&self) -> CupertinoConfirmEditStyle {
        let theme = self.theme.get();
        let ComponentTokens::ConfirmEdit(tokens) = theme.component(ComponentId::ConfirmEdit) else {
            unreachable!("ComponentId::ConfirmEdit must resolve to ConfirmEditTokens")
        };
        let c = theme.colors();
        CupertinoConfirmEditStyle {
            design_system: "cupertino",
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

    /// The confirm-edit, ready to mount.
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
                edit: CupertinoIcon::new("edit").path_data(),
                check: CupertinoIcon::new("check").path_data(),
                close: CupertinoIcon::new("close").path_data(),
                view_box: VIEW_BOX,
            });
        if let Some(on_save) = self.on_save {
            widget = widget.on_save(move |value| on_save(value));
        }
        widget
    }
}

impl Default for CupertinoConfirmEdit {
    fn default() -> Self {
        Self::new()
    }
}
