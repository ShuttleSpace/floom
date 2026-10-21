//! Fluent confirm-edit, ported to the Floom 2.0 widget API.
//!
//! A confirm-edit is a value with an edit affordance beside it: click the
//! affordance and the value becomes a field with a tick and a cross beside it;
//! the tick saves and the cross cancels.
//!
//! ## What the port uses
//!
//! The engine's vocabulary: a horizontal `Stack` for each of the two rows, the
//! engine's `TextInput` for the field, a `Pressable` for each of the three
//! buttons, and the engine's `VectorIcon` for the three glyphs.
//!
//! **The engine's `Dialog` is not used.** v1's confirm-edit is an inline row,
//! not a modal panel: the edit affordance swaps a field in beside the value,
//! with no scrim and no overlay. `floom-ui-composites::Dialog` would be the
//! wrong shape for it.
//!
//! ## The engine's confirm-edit
//!
//! The one thing the vocabulary cannot do is *choose between two rows*. 2.0's
//! tree is built once: a child's builder closure runs when its slot is
//! mounted, and a slot that is still there is not built again. The engine's
//! `Conditional` follows a shared flag, but it holds *one* child and shows it
//! while the flag is true -- so it can show the editing row and not the display
//! row, but not both.
//!
//! So the widget reads the "editing" flag in `update` and asks the host to
//! reconcile its children; the two rows carry different keys, so the host
//! mounts the other one. That widget is the engine's now:
//! `floom_ui_composites::ConfirmEdit` is a value and the buttons that edit it,
//! and the three kits use it.
//!
//! The three glyphs -- the edit affordance, the tick and the cross -- are the
//! design system's own, so the kit hands the widget their path data and the
//! box they are drawn in.
//!
//! ## What this kit answers
//!
//! * the gap between the parts of a row and the field's radius: the tokens;
//! * the tick's colour: `primary`; the cross's and the edit affordance's:
//!   `on_surface_variant`; the field's text: `on_surface`; its outline:
//!   `outline`.
//!
//! ## The token that never showed
//!
//! `ConfirmEditTokens::icon_size` is 20, and v1 never used it: all three
//! glyphs were drawn at 16. The port keeps the number in the style and draws
//! the glyphs at v1's 16.
//!
//! ## The numbers v1 wrote as literals
//!
//! The glyphs' size (16), the padding around a glyph (8), the field's width
//! (160), its font size (14), its horizontal padding (8) and its outline (1).
//! v1's style reported none of them, so the port's style adds them.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The display label's style.** v1's display label carried no style at
//!   all -- no size, no colour -- so it inherited the engine's default text.
//!   The port does the same, which is why the value is black while the field
//!   is `on_surface`.
//! * **The child's factory.** v1's `child(..)` took a closure that built an
//!   `AnyView`; 2.0's takes a [`floom_core::Child`], because a 2.0 widget is
//!   mounted by its parent rather than returned.
//! * **The callback's shape.** v1's was `Box<dyn Fn(String)>`; the port's is
//!   `Arc<dyn Fn(String) + Send + Sync>`, because a callback that reaches a
//!   2.0 slot has to cross a thread boundary. The props' own `on_save` slot
//!   stays empty: the shared props type is `Box<dyn Fn(String)>` and cannot
//!   hold the `Send + Sync` handle the kit uses.
//! * **The three signals.** v1 held the value, the draft and the "editing"
//!   flag in `RwSignal`s. A 2.0 slot closure is `Send + Sync` and an
//!   `RwSignal` is `Rc`-based, so the port uses the engine's shared types: a
//!   `SharedBuffer` for the value, a `SharedBuffer` for the draft and a
//!   `SharedFlag` for the flag.
//! * **The accessors.** v1's confirm-edits had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three.
//!
//! ## The theme
//!
//! Fluent takes a snapshot of its own design system, which is what v1 did: a
//! change of the app-wide theme does not reach this confirm-edit. Material
//! reads the app-wide theme signal instead, so that a change of theme
//! re-themes it along with everything else.

use std::sync::{Arc, Mutex};

use floom_core::Child;
use floom_signal::{RwSignal, SignalGet};
use floom_types::Color as PaintColor;
pub use floom_ui_composites::ConfirmEdit;
use floom_ui_composites::{ConfirmEditGlyphs, ConfirmEditStyle};
use floom_widgets::{
    SharedBuffer, SharedFlag, read_flag, shared_flag,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::confirm_edit::ConfirmEditProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_fluent_icon::{VIEW_BOX, FluentIcon};

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

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

/// Write a shared buffer.
fn write_buffer(buffer: &SharedBuffer, text: impl Into<String>) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.into();
    }
}

/// The confirm-edit's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentConfirmEditStyle {
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

/// Fluent's confirm-edit.
pub struct FluentConfirmEdit {
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

impl FluentConfirmEdit {
    pub fn new() -> Self {
        let props = ConfirmEditProps::default();
        Self {
            value: Arc::new(Mutex::new(props.value.clone())),
            draft: Arc::new(Mutex::new(props.value.clone())),
            editing: shared_flag(false),
            child: None,
            // v1's Fluent confirm-edit took a *snapshot* of its own design system,
            // so a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
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
