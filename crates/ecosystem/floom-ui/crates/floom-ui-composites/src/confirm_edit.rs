
use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, LayoutRequest, Rect, UpdateContext, Widget,
    WidgetAny,
};
use floom_widgets::{
    read_flag, shared_flag, write_flag, Align, Axis, Pressable, SharedBuffer, SharedFlag, Stack,
    Surface, Text, TextInput, TextInputStyle, VectorIcon,
};

/// Read a shared buffer.
fn read_buffer(buffer: &SharedBuffer) -> String {
    buffer.lock().map(|text| text.clone()).unwrap_or_default()
}

/// Write a shared buffer.
fn write_buffer(buffer: &SharedBuffer, text: impl Into<String>) {
    if let Ok(mut guard) = buffer.lock() {
        *guard = text.into();
    }
}

/// The three glyphs the confirm-edit's buttons carry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfirmEditGlyphs {
    /// The edit affordance's glyph.
    pub edit: &'static str,
    /// The tick's glyph.
    pub check: &'static str,
    /// The cross's glyph.
    pub close: &'static str,
    /// The box the glyphs are drawn in.
    pub view_box: Rect,
}

/// The confirm-edit's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConfirmEditStyle {
    /// The gap between the parts of a row.
    pub gap: f32,
    /// The field's radius.
    pub radius: f32,
    /// The token's icon size, which v1 never used: the glyphs were 16.
    pub icon_size: f32,
    /// The glyphs' size.
    pub glyph_size: f32,
    /// The padding around a glyph.
    pub button_padding: f32,
    /// The field's width.
    pub input_width: f32,
    /// The field's font size.
    pub input_font_size: f32,
    /// The field's horizontal padding.
    pub input_padding_horiz: f32,
    /// The field's outline width.
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

impl Default for ConfirmEditStyle {
    fn default() -> Self {
        Self {
            gap: 8.0,
            radius: 4.0,
            icon_size: 24.0,
            glyph_size: 16.0,
            button_padding: 8.0,
            input_width: 160.0,
            input_font_size: 14.0,
            input_padding_horiz: 8.0,
            input_border: 1.0,
            on_surface: Color::rgb(0x1C, 0x1B, 0x1F),
            on_surface_variant: Color::rgb(0x49, 0x45, 0x55),
            primary: Color::rgb(0x67, 0x50, 0xA4),
            outline: Color::rgb(0x79, 0x74, 0x7E),
        }
    }
}

/// The confirm-edit's widget: the display row, or the editing row.
///
/// The engine gap, in one type: the engine has no confirm-edit, and its tree
/// does not rebuild a subtree on its own, so the widget that owns the rows
/// asks for the rebuild.
pub struct ConfirmEdit {
    style: ConfirmEditStyle,
    value: SharedBuffer,
    draft: SharedBuffer,
    editing: SharedFlag,
    child: Option<Child>,
    on_save: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
    /// Whether the row that is mounted is the editing one.
    built_editing: bool,
    /// The glyphs the buttons carry.
    glyphs: ConfirmEditGlyphs,
}

impl ConfirmEdit {
    pub fn new() -> Self {
        Self {
            style: ConfirmEditStyle::default(),
            value: Arc::new(Mutex::new(String::new())),
            draft: Arc::new(Mutex::new(String::new())),
            editing: shared_flag(false),
            child: None,
            on_save: None,
            built_editing: false,
            glyphs: ConfirmEditGlyphs {
                edit: "",
                check: "",
                close: "",
                view_box: Rect::ZERO,
            },
        }
    }

    /// The confirm-edit's appearance.
    pub fn style(mut self, style: ConfirmEditStyle) -> Self {
        self.style = style;
        self
    }

    /// The committed value and the draft being edited.
    pub fn buffers(mut self, value: SharedBuffer, draft: SharedBuffer) -> Self {
        self.value = value;
        self.draft = draft;
        self
    }

    /// The flag that says whether the field is in edit mode.
    pub fn editing(mut self, editing: SharedFlag) -> Self {
        self.editing = editing;
        self
    }

    /// What the display mode shows instead of the value.
    pub fn child(mut self, child: Option<Child>) -> Self {
        self.child = child;
        self
    }

    /// Called with the new value when the edit is confirmed.
    pub fn on_save(mut self, callback: impl Fn(String) + Send + Sync + 'static) -> Self {
        self.on_save = Some(Arc::new(callback));
        self
    }

    /// The glyphs the buttons carry.
    pub fn glyphs(mut self, glyphs: ConfirmEditGlyphs) -> Self {
        self.glyphs = glyphs;
        self
    }
}

impl ConfirmEdit {
    /// Whether the field is in edit mode.
    pub fn is_editing(&self) -> bool {
        read_flag(&self.editing)
    }

    /// The committed value.
    pub fn value(&self) -> String {
        read_buffer(&self.value)
    }

    /// The draft -- what the field is editing.
    pub fn draft(&self) -> String {
        read_buffer(&self.draft)
    }

    /// The draft's buffer -- what the field is editing.
    pub fn draft_buffer(&self) -> SharedBuffer {
        self.draft.clone()
    }

    /// The "editing" flag.
    pub fn editing_flag(&self) -> SharedFlag {
        self.editing.clone()
    }

    /// Open the field.
    pub fn edit(&self) {
        write_buffer(&self.draft, read_buffer(&self.value));
        write_flag(&self.editing, true);
    }

    /// Close the field, throwing the draft away.
    pub fn cancel(&self) {
        write_buffer(&self.draft, read_buffer(&self.value));
        write_flag(&self.editing, false);
    }

    /// Save the draft: the value becomes it, the callback fires, and the field
    /// closes.
    pub fn save(&self) {
        let next = read_buffer(&self.draft);
        write_buffer(&self.value, next.clone());
        if let Some(on_save) = &self.on_save {
            on_save(next);
        }
        write_flag(&self.editing, false);
    }
}

impl Widget for ConfirmEdit {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The mode changed: the two rows are different subtrees, and 2.0's tree
    /// is built once, so the other one has to be mounted.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let editing = read_flag(&self.editing);
        if editing != self.built_editing {
            self.built_editing = editing;
            cx.mark_children_dirty(id);
            cx.mark_dirty(id);
        }
    }

    /// The row for the current mode.
    ///
    /// The two rows carry different keys, so the host mounts the other one
    /// rather than reusing the slot that is there.
    fn children(&self) -> Vec<Child> {
        if read_flag(&self.editing) {
            vec![Child::new(ChildKey::keyed(1), LayoutRequest::new(), {
                let style = self.style;
                let value = self.value.clone();
                let draft = self.draft.clone();
                let editing = self.editing.clone();
                let on_save = self.on_save.clone();
                let glyphs = self.glyphs;
                move || {
                    build_edit_row(
                        style,
                        value.clone(),
                        draft.clone(),
                        editing.clone(),
                        on_save.clone(),
                        glyphs,
                    )
                }
            })]
        } else {
            vec![Child::new(ChildKey::keyed(0), LayoutRequest::new(), {
                let style = self.style;
                let value = self.value.clone();
                let draft = self.draft.clone();
                let editing = self.editing.clone();
                let child = self.child.clone();
                let glyphs = self.glyphs;
                move || {
                    build_display_row(
                        style,
                        value.clone(),
                        draft.clone(),
                        editing.clone(),
                        child.clone(),
                        glyphs,
                    )
                }
            })]
        }
    }

    /// A row: the parts side by side.
    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().stack(Axis::Horizontal, 0.0))
    }
}

impl WidgetAny for ConfirmEdit {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Nothing of its own: the buttons are `Pressable`s and they name
    /// themselves.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// The display row: the value (or the caller's child) and the edit
/// affordance.
fn build_display_row(
    style: ConfirmEditStyle,
    value: SharedBuffer,
    draft: SharedBuffer,
    editing: SharedFlag,
    child: Option<Child>,
    glyphs: ConfirmEditGlyphs,
) -> Box<dyn WidgetAny> {
    let mut row = Stack::new(Axis::Horizontal)
        .gap(style.gap)
        // v1's `.items_center()`.
        .align(Align::Start, Align::Center);

    // The content: the caller's child, or the value.
    if let Some(child) = child {
        row = row.child(child);
    } else {
        let text = read_buffer(&value);
        row = row.push(move || {
            // v1's display label carried no style at all, so it inherited the
            // engine's default text.
            Box::new(Text::new(text.clone())) as Box<dyn WidgetAny>
        });
    }

    // The edit affordance.
    row = row.push({
        let style = style;
        let value = value.clone();
        let draft = draft.clone();
        let editing = editing.clone();
        let glyphs = glyphs;
        move || {
            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || {
                        Box::new(
                            Surface::new()
                                .request(
                                    LayoutRequest::new()
                                        .padding_horiz(f64::from(style.button_padding)),
                                )
                                .push({
                                    let style = style;
                                    move || {
                                        Box::new(
                                            VectorIcon::from_path_data(
                                                glyphs.edit,
                                                glyphs.view_box,
                                            )
                                            .size(style.glyph_size)
                                            .color(style.on_surface_variant),
                                        )
                                            as Box<dyn WidgetAny>
                                    }
                                }),
                        ) as Box<dyn WidgetAny>
                    },
                ))
                .on_click({
                    let draft = draft.clone();
                    let value = value.clone();
                    let editing = editing.clone();
                    move || {
                        // v1: the draft takes the value, then the field opens.
                        write_buffer(&draft, read_buffer(&value));
                        write_flag(&editing, true);
                    }
                })
                .label("Edit"),
            ) as Box<dyn WidgetAny>
        }
    });

    Box::new(row) as Box<dyn WidgetAny>
}

/// The editing row: the field, the tick and the cross.
fn build_edit_row(
    style: ConfirmEditStyle,
    value: SharedBuffer,
    draft: SharedBuffer,
    editing: SharedFlag,
    on_save: Option<Arc<dyn Fn(String) + Send + Sync + 'static>>,
    glyphs: ConfirmEditGlyphs,
) -> Box<dyn WidgetAny> {
    let mut row = Stack::new(Axis::Horizontal)
        .gap(style.gap)
        // v1's `.items_center()`.
        .align(Align::Start, Align::Center);

    // The field.
    row = row.push_layout(LayoutRequest::new().width(style.input_width), {
        let style = style;
        let draft = draft.clone();
        move || {
            // The engine's `TextInput::new` leaves the caret at the start;
            // a field opened with a value puts it at the end.
            let mut input = TextInput::new(draft.clone());
            input.set_caret(read_buffer(&draft).len());
            Box::new(input.style(TextInputStyle {
                font_size: style.input_font_size,
                color: style.on_surface,
                padding_horiz: f64::from(style.input_padding_horiz),
                radius: style.radius,
                border_color: style.outline,
                border_width: style.input_border,
                ..TextInputStyle::default()
            })) as Box<dyn WidgetAny>
        }
    });

    // The tick: save.
    row = row.push({
        let style = style;
        let value = value.clone();
        let draft = draft.clone();
        let editing = editing.clone();
        let on_save = on_save.clone();
        move || {
            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || {
                        Box::new(
                            Surface::new()
                                .request(
                                    LayoutRequest::new()
                                        .padding_horiz(f64::from(style.button_padding)),
                                )
                                .push({
                                    let style = style;
                                    move || {
                                        Box::new(
                                            VectorIcon::from_path_data(
                                                glyphs.check,
                                                glyphs.view_box,
                                            )
                                            .size(style.glyph_size)
                                            .color(style.primary),
                                        )
                                            as Box<dyn WidgetAny>
                                    }
                                }),
                        ) as Box<dyn WidgetAny>
                    },
                ))
                .on_click({
                    let draft = draft.clone();
                    let value = value.clone();
                    let editing = editing.clone();
                    let on_save = on_save.clone();
                    move || {
                        // v1: the value takes the draft, then the callback fires,
                        // then the field closes.
                        let next = read_buffer(&draft);
                        write_buffer(&value, next.clone());
                        if let Some(on_save) = &on_save {
                            on_save(next);
                        }
                        write_flag(&editing, false);
                    }
                })
                .label("Save"),
            ) as Box<dyn WidgetAny>
        }
    });

    // The cross: cancel.
    row = row.push({
        let style = style;
        let value = value.clone();
        let draft = draft.clone();
        let editing = editing.clone();
        move || {
            Box::new(
                Pressable::new(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    move || {
                        Box::new(
                            Surface::new()
                                .request(
                                    LayoutRequest::new()
                                        .padding_horiz(f64::from(style.button_padding)),
                                )
                                .push({
                                    let style = style;
                                    move || {
                                        Box::new(
                                            VectorIcon::from_path_data(
                                                glyphs.close,
                                                glyphs.view_box,
                                            )
                                            .size(style.glyph_size)
                                            .color(style.on_surface_variant),
                                        )
                                            as Box<dyn WidgetAny>
                                    }
                                }),
                        ) as Box<dyn WidgetAny>
                    },
                ))
                .on_click({
                    let draft = draft.clone();
                    let value = value.clone();
                    let editing = editing.clone();
                    move || {
                        // v1: the draft takes the value back, then the field
                        // closes.
                        write_buffer(&draft, read_buffer(&value));
                        write_flag(&editing, false);
                    }
                })
                .label("Cancel"),
            ) as Box<dyn WidgetAny>
        }
    });

    Box::new(row) as Box<dyn WidgetAny>
}
