
use std::collections::HashMap;
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Insets, LayoutRequest, Rect, UpdateContext,
    Widget, WidgetAny,
};
use floom_types::FontWeight;
use floom_widgets::{
    shared_map, Align, Axis, Pressable, SharedMap, Stack, Surface, Text, VectorIcon,
};

/// The key the selected step lives under in the shared map.
const SELECTED_KEY: &str = "selected";

/// Read the selected step out of the shared map.
fn read_selected(selected: &SharedMap) -> usize {
    let Ok(guard) = selected.lock() else {
        return 0;
    };
    guard
        .get(SELECTED_KEY)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

/// Write the selected step into the shared map.
fn write_selected(selected: &SharedMap, index: usize) {
    if let Ok(mut guard) = selected.lock() {
        guard.insert(SELECTED_KEY.to_string(), format!("{index}"));
    }
}

/// The tick a completed step carries.
///
/// The glyph is a design system's own, so the stepper is told what to draw
/// rather than deciding for it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StepperTick {
    /// The glyph's path data.
    pub path: &'static str,
    /// The box the path is drawn in.
    pub view_box: Rect,
}

/// The stepper's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StepperStyle {
    /// The marker's size.
    pub step_size: f32,
    /// The connector's thickness.
    pub connector_width: f32,
    /// The gap between two steps.
    pub gap: f32,
    /// The label's size.
    pub label_font_size: f32,
    /// The marker's fill, for a step that is current or behind.
    pub primary: Color,
    /// The marker's ink, for a step that is current or behind.
    pub on_primary: Color,
    /// The marker's fill, for a step that is still ahead.
    pub surface: Color,
    /// The current step's label ink.
    pub on_surface: Color,
    /// The other labels' ink.
    pub on_surface_variant: Color,
    /// The marker's outline, for a step that is still ahead.
    pub outline: Color,
    /// The connector's colour, ahead of the current step.
    pub outline_variant: Color,
    /// The number inside a marker.
    pub marker_font_size: f32,
    /// The marker's radius.
    pub marker_radius: f32,
    /// The marker's outline width.
    pub marker_border: f32,
    /// The gap between a marker and its label.
    pub item_gap: f32,
    /// The padding around the row, left and right.
    pub padding_horiz: f32,
    /// The padding around the row, top and bottom.
    pub padding_vert: f32,
    /// The connector's smallest width.
    pub connector_min_width: f32,
    /// The tick's size, as a fraction of the marker's.
    pub check_scale: f32,
}

impl Default for StepperStyle {
    fn default() -> Self {
        Self {
            step_size: 24.0,
            connector_width: 2.0,
            gap: 8.0,
            label_font_size: 14.0,
            primary: Color::rgb(0x67, 0x50, 0xA4),
            on_primary: Color::rgb(0xFF, 0xFF, 0xFF),
            surface: Color::rgb(0xFF, 0xFF, 0xFF),
            on_surface: Color::rgb(0x1C, 0x1B, 0x1F),
            on_surface_variant: Color::rgb(0x49, 0x45, 0x55),
            outline: Color::rgb(0x79, 0x74, 0x7E),
            outline_variant: Color::rgb(0xE7, 0xE0, 0xEC),
            marker_font_size: 14.0,
            marker_radius: 9999.0,
            marker_border: 1.0,
            item_gap: 8.0,
            padding_horiz: 8.0,
            padding_vert: 12.0,
            connector_min_width: 16.0,
            check_scale: 0.55,
        }
    }
}

/// The stepper's widget: the row of steps, rebuilt when the selection moves.
///
/// The engine gap, in one type: the engine has no stepper, and the engine's
/// tree does not rebuild a subtree on its own, so the widget that owns the row
/// asks for the rebuild.
pub struct Stepper {
    style: StepperStyle,
    items: Vec<String>,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    /// The selection the row was last built for.
    built_for: usize,
    /// The tick a completed step carries.
    tick: StepperTick,
}

impl Stepper {
    pub fn new() -> Self {
        Self {
            style: StepperStyle::default(),
            items: Vec::new(),
            selected: shared_map(HashMap::new()),
            editable: false,
            non_linear: false,
            on_change: None,
            built_for: usize::MAX,
            tick: StepperTick {
                path: "",
                view_box: Rect::ZERO,
            },
        }
    }

    /// The stepper's appearance.
    pub fn style(mut self, style: StepperStyle) -> Self {
        self.style = style;
        self
    }

    /// The ordered step labels.
    pub fn labels(mut self, labels: Vec<String>) -> Self {
        self.items = labels;
        self
    }

    /// The shared map the current step lives in.
    ///
    /// The row is keyed by the step it shows, so the first frame does not
    /// rebuild: the key already carries the right step.
    pub fn selection(mut self, selected: SharedMap) -> Self {
        self.built_for = read_selected(&selected);
        self.selected = selected;
        self
    }

    /// Allow the steps behind the current one to be clicked.
    pub fn editable(mut self, editable: bool) -> Self {
        self.editable = editable;
        self
    }

    /// Allow jumping to any step on click.
    pub fn non_linear(mut self, non_linear: bool) -> Self {
        self.non_linear = non_linear;
        self
    }

    /// Called with the step's index whenever a step is activated.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// The tick a completed step carries.
    pub fn tick(mut self, tick: StepperTick) -> Self {
        self.tick = tick;
        self
    }
}

impl Stepper {
    /// The selected step, as it stands.
    pub fn selected(&self) -> usize {
        read_selected(&self.selected)
    }

    /// The shared map the selected step lives in.
    pub fn selected_map(&self) -> SharedMap {
        self.selected.clone()
    }

    /// Move the selection.
    ///
    /// The row is rebuilt by the host on the next frame, because the colours
    /// follow the selection.
    pub fn set_selected(&self, index: usize) {
        write_selected(&self.selected, index);
    }

    /// The step labels.
    pub fn items(&self) -> &[String] {
        &self.items
    }
}

impl Widget for Stepper {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The selection moved: the row's colours follow it, and 2.0's tree is
    /// built once, so the row has to be rebuilt for them to move.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let selected = read_selected(&self.selected);
        if selected != self.built_for {
            self.built_for = selected;
            cx.mark_children_dirty(id);
            cx.mark_dirty(id);
        }
    }

    /// The row, as one slot whose key carries the selection.
    ///
    /// The key is what makes the rebuild a rebuild: the host diffs children by
    /// key, so a new key mounts a new row and unmounts the old one.
    fn children(&self) -> Vec<Child> {
        let selected = read_selected(&self.selected);
        vec![Child::new(
            ChildKey::keyed(selected as u64),
            LayoutRequest::new(),
            {
                let style = self.style;
                let items = self.items.clone();
                let selected = self.selected.clone();
                let editable = self.editable;
                let non_linear = self.non_linear;
                let on_change = self.on_change.clone();
                let tick = self.tick.clone();
                move || {
                    build_row(
                        style,
                        items.clone(),
                        selected.clone(),
                        editable,
                        non_linear,
                        on_change.clone(),
                        tick.clone(),
                    )
                }
            },
        )]
    }

    /// v1's outer container: the full width, padded 8 across and 12 down.
    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().width_pct(100.0).padding(Insets {
            left: f64::from(self.style.padding_horiz),
            right: f64::from(self.style.padding_horiz),
            top: f64::from(self.style.padding_vert),
            bottom: f64::from(self.style.padding_vert),
        }))
    }
}

impl WidgetAny for Stepper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// Nothing of its own: each step's `Pressable` names itself.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// The row: the steps and the connectors between them.
///
/// v1 returned an empty view when there were no steps; an empty row is an
/// empty stack here, which paints nothing.
fn build_row(
    style: StepperStyle,
    items: Vec<String>,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    tick: StepperTick,
) -> Box<dyn WidgetAny> {
    let count = items.len();
    let mut row = Stack::new(Axis::Horizontal)
        .gap(style.gap)
        // v1's `.items_center()` on the row.
        .align(Align::Start, Align::Center)
        .request(LayoutRequest::new().width_pct(100.0));

    for (index, label) in items.iter().enumerate() {
        row = row.push({
            let style = style;
            let label = label.clone();
            let selected = selected.clone();
            let editable = editable;
            let non_linear = non_linear;
            let on_change = on_change.clone();
            let tick = tick.clone();
            move || {
                build_step(
                    style,
                    index,
                    label.clone(),
                    selected.clone(),
                    editable,
                    non_linear,
                    on_change.clone(),
                    tick.clone(),
                )
            }
        });

        if index + 1 < count {
            row = row.push({
                let style = style;
                let selected = selected.clone();
                let index = index;
                move || build_connector(style, index, selected.clone())
            });
        }
    }

    Box::new(row) as Box<dyn WidgetAny>
}

/// One step: its marker and its label, clickable when the rules allow it.
fn build_step(
    style: StepperStyle,
    index: usize,
    label: String,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    tick: StepperTick,
) -> Box<dyn WidgetAny> {
    let current = read_selected(&selected);
    let completed = index < current;
    let done = current >= index;

    // The marker's colours: a step that is current or behind is filled; one
    // that is still ahead is not.
    let (background, foreground, border) = if done {
        (style.primary, style.on_primary, style.primary)
    } else {
        (style.surface, style.on_surface_variant, style.outline)
    };

    // The label's colours.
    let label_color = if current == index {
        style.on_surface
    } else if current > index {
        style.primary
    } else {
        style.on_surface_variant
    };
    let label_weight = if current == index {
        FontWeight::Medium
    } else {
        FontWeight::Normal
    };

    // The tick's path, handed in: the glyph does not change with the
    // selection, only the step it is drawn in does.
    let check_path = tick.path;
    let check_view_box = tick.view_box;

    let step = Stack::new(Axis::Vertical)
        .gap(style.item_gap)
        // v1's `.items_center()` on the step.
        .align(Align::Start, Align::Center)
        .push({
            let style = style;
            let completed = completed;
            let index = index;
            let foreground = foreground;
            let background = background;
            let border = border;
            move || {
                Box::new(
                    Surface::new()
                        .request(
                            LayoutRequest::new()
                                .width(style.step_size)
                                .height(style.step_size)
                                // v1's `.items_center().justify_center()`.
                                .main_align(Align::Center)
                                .cross_align(Align::Center),
                        )
                        .radius(style.marker_radius)
                        .background(background)
                        .border(style.marker_border, border)
                        .push({
                            let style = style;
                            let completed = completed;
                            let index = index;
                            let foreground = foreground;
                            let check_path = check_path;
                            move || -> Box<dyn WidgetAny> {
                                if completed {
                                    // v1 drew a tick in `on_primary` for a step
                                    // that is behind the current one.
                                    Box::new(
                                        VectorIcon::from_path_data(check_path, check_view_box)
                                            .size(style.step_size * style.check_scale)
                                            .color(style.on_primary),
                                    )
                                } else {
                                    Box::new(
                                        Text::new(format!("{}", index + 1))
                                            .size(style.marker_font_size)
                                            .weight(FontWeight::Medium)
                                            .color(foreground),
                                    )
                                }
                            }
                        }),
                ) as Box<dyn WidgetAny>
            }
        })
        .push({
            let style = style;
            let label = label.clone();
            let label_color = label_color;
            let label_weight = label_weight;
            move || {
                Box::new(
                    Text::new(label.clone())
                        .size(style.label_font_size)
                        .weight(label_weight)
                        .color(label_color),
                ) as Box<dyn WidgetAny>
            }
        });

    // v1's rule: a step is clickable when it is behind the current one and
    // the stepper is editable, or when the stepper is non-linear.
    let clickable = (editable && index < current) || non_linear;
    if clickable {
        Box::new(
            Pressable::new(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                move || Box::new(step.clone()) as Box<dyn WidgetAny>,
            ))
            .on_click({
                let selected = selected.clone();
                let on_change = on_change.clone();
                move || {
                    write_selected(&selected, index);
                    if let Some(on_change) = &on_change {
                        on_change(index);
                    }
                }
            })
            // An addition rather than a translation: v1's stepper published no
            // accessibility description, and a button nobody can name is a
            // button nobody can use.
            .label(label),
        ) as Box<dyn WidgetAny>
    } else {
        Box::new(step) as Box<dyn WidgetAny>
    }
}

/// The connector between two steps.
fn build_connector(style: StepperStyle, index: usize, selected: SharedMap) -> Box<dyn WidgetAny> {
    let current = read_selected(&selected);
    // v1: the connector behind the current step is `primary`, the one ahead of
    // it is `outline_variant`.
    let color = if index < current {
        style.primary
    } else {
        style.outline_variant
    };
    Box::new(
        Surface::new()
            .request(
                LayoutRequest::new()
                    .height(style.connector_width)
                    .grow(1.0)
                    .min_width(style.connector_min_width),
            )
            .background(color),
    ) as Box<dyn WidgetAny>
}
