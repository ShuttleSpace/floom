//! Fluent stepper, ported to the Floom 2.0 widget API.
//!
//! A stepper is a row of steps: a circle with a number in it -- or a tick, once
//! the step is behind you -- the step's label under it, and a connector
//! between each pair of steps. One step is the current one.
//!
//! ## What the port uses
//!
//! The engine's vocabulary for the row: a horizontal `Stack` of steps and
//! connectors, each step a vertical `Stack` of a marker (a `Surface`) and a
//! label (a `Text`), each clickable step a `Pressable`. The engine's icon
//! (`VectorIcon`) for the tick.
//!
//! **The engine's `Counter` is a different component.** v1's `md-counter` is
//! the `- / value / +` stepper, and the engine has promoted that to
//! `floom-ui-composites::Counter`. This family is the *wizard* stepper -- a
//! row of numbered steps with connectors -- and v1 never drew a counter in
//! it, so the engine's `Counter` is not the thing to reach for here.
//!
//! ## The engine gap
//!
//! **The engine has no stepper**, and the one thing the vocabulary cannot do
//! is *follow the selection*. 2.0's tree is built once: a child's builder
//! closure runs when its slot is mounted, and a slot that is still there is
//! not built again. v1's stepper read its `RwSignal<usize>` inside a style
//! closure, so the row's colours followed the selection as it moved. Here the
//! colours are decided when the row is built, so the kit-local widget reads
//! the selection in `update` and asks the host to reconcile its children --
//! and the row's slot key carries the selection, so the row that is rebuilt
//! is a new row.
//!
//! The shape belongs in `floom-ui-composites` beside `Select` and `Menu`; it
//! is in the kit because that crate is being edited concurrently.
//!
//! ## What this kit answers
//!
//! * the marker's diameter, the connector's thickness, the gap between the
//!   steps and the label's size: the tokens;
//! * the colours: a marker that is current or behind is `primary` with
//!   `on_primary` in it and a `primary` outline; one that is still ahead is
//!   `surface` with `on_surface_variant` in it and an `outline` outline; the
//!   label is `on_surface` when it is the current step, `primary` when the
//!   step is behind and `on_surface_variant` when it is ahead; the connector
//!   is `primary` behind the current step and `outline_variant` ahead of it.
//!
//! ## The numbers v1 wrote as literals
//!
//! The number inside a marker (14), the marker's radius (9999, a circle), the
//! marker's outline (1), the gap between a marker and its label (8), the
//! padding around the row (8 across, 12 down), the connector's smallest width
//! (16) and the tick's size (55 percent of the marker). v1's style reported
//! none of them, so the port's style adds them: a style that does not say what
//! the widget draws is not the widget's output.
//!
//! ## The props that never showed
//!
//! `StepperProps` carries `gap`, `connector_width`, `step_size` and
//! `label_font_size` -- the same four numbers the tokens carry, with the same
//! defaults. v1's `view()` read the *tokens*, so the props' four copies were
//! dead: the port reads the tokens too, and the props' copies stay dead.
//!
//! ## What v1 wrote that this port does not carry
//!
//! * **The pointer cursor.** v1 set `CursorStyle::Pointer` over a clickable
//!   step. 2.0's `Pressable` has no cursor of its own, so the affordance is
//!   the press itself.
//! * **The accessors.** v1's steppers had no `props()`, `theme()` or
//!   `with_theme()`. The port gives all three kits the same three, because a
//!   kit that cannot be given a theme cannot be tested against one.
//! * **The items' signal.** v1 held the labels in an `RwSignal<Vec<String>>`
//!   it never handed out, so the port holds them plainly.
//! * **The selection's signal.** v1 held the selected step in an
//!   `RwSignal<usize>` it never handed out either. A 2.0 caller needs a handle
//!   it can read, so the selection lives in the engine's `SharedMap` under the
//!   key `selected`, and the kit hands the map out.
//!
//! ## The theme
//!
//! Fluent takes a snapshot of its own design system, which is what v1 did: a
//! change of the app-wide theme does not reach this stepper. Material reads
//! the app-wide theme signal instead, so that a change of theme re-themes it
//! along with everything else.

use std::collections::HashMap;
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest, UpdateContext, Widget, WidgetAny,
};
use floom_signal::{RwSignal, SignalGet};
use floom_types::{Color as PaintColor, FontWeight, Insets};
use floom_widgets::{
    Align, Axis, Pressable, SharedMap, Stack, Surface, Text, VectorIcon, shared_map,
};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::stepper::StepperProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_fluent_icon::{VIEW_BOX, FluentIcon};

/// The key the selected step lives under in the shared map.
const SELECTED_KEY: &str = "selected";

/// The number inside a marker's font size. A v1 literal.
const MARKER_FONT_SIZE: f32 = 14.0;
/// The marker's radius: 9999 is a circle whatever the size.
const MARKER_RADIUS: f32 = 9999.0;
/// The marker's outline width. A v1 literal.
const MARKER_BORDER: f32 = 1.0;
/// The gap between a marker and its label. A v1 literal.
const ITEM_GAP: f32 = 8.0;
/// The padding around the row, left and right. A v1 literal.
const PADDING_HORIZ: f32 = 8.0;
/// The padding around the row, top and bottom. A v1 literal.
const PADDING_VERT: f32 = 12.0;
/// The connector's smallest width. A v1 literal.
const CONNECTOR_MIN_WIDTH: f32 = 16.0;
/// The tick's size, as a fraction of the marker's. A v1 literal.
const CHECK_SCALE: f32 = 0.55;

/// Theme colour -> engine colour.
fn to_paint(c: Color) -> PaintColor {
    PaintColor {
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
    }
}

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

/// The stepper's appearance, in the terms v1 used.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FluentStepperStyle {
    pub design_system: &'static str,
    pub step_size: f32,
    pub connector_width: f32,
    pub gap: f32,
    pub label_font_size: f32,
    pub primary: Color,
    pub on_primary: Color,
    pub surface: Color,
    pub on_surface: Color,
    pub on_surface_variant: Color,
    pub outline: Color,
    pub outline_variant: Color,
    // --- the numbers v1 wrote as literals, which v1's style did not report ---
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

/// Fluent's stepper.
pub struct FluentStepper {
    props: StepperProps,
    theme: RwSignal<MultiTheme>,
    /// The step labels.
    items: Vec<String>,
    /// The selected step, shared with whoever drives the stepper.
    selected: SharedMap,
    /// The change callback, in the shape a 2.0 slot needs.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl FluentStepper {
    pub fn new() -> Self {
        Self {
            props: StepperProps::default(),
            // v1's Fluent stepper took a *snapshot* of its own design system, so
            // a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Fluent,
            )),
            items: Vec::new(),
            selected: shared_map(HashMap::new()),
            on_change: None,
        }
    }

    /// The ordered step labels.
    pub fn items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }

    /// The initially-active step index.
    pub fn selected(mut self, selected: usize) -> Self {
        write_selected(&self.selected, selected);
        self
    }

    /// Fired when a step is activated by click (respecting `editable` and
    /// `non_linear`).
    ///
    /// `Send + Sync`, where v1's was not: the callback reaches a 2.0 slot.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }

    /// Allow completed/earlier steps to be clicked (navigate backward).
    pub fn editable(mut self, editable: bool) -> Self {
        self.props.editable = editable;
        self
    }

    /// Allow jumping to any step on click (not just the next one).
    pub fn non_linear(mut self, non_linear: bool) -> Self {
        self.props.non_linear = non_linear;
        self
    }

    pub fn props(&self) -> &StepperProps {
        &self.props
    }

    pub fn theme(&self) -> RwSignal<MultiTheme> {
        self.theme
    }

    pub fn with_theme(mut self, theme: MultiTheme) -> Self {
        self.theme = RwSignal::new(theme);
        self
    }

    /// The shared map the selected step lives in.
    pub fn selected_map(&self) -> SharedMap {
        self.selected.clone()
    }

    /// The selected step, as it stands.
    pub fn selected_index(&self) -> usize {
        read_selected(&self.selected)
    }

    /// Move the selection, after the stepper was built.
    ///
    /// The 2.0 replacement for writing to v1's `selected` signal, which the
    /// kit never handed out.
    pub fn set_selected(&self, index: usize) {
        write_selected(&self.selected, index);
    }

    pub fn resolved_style(&self) -> FluentStepperStyle {
        let theme = self.theme.get();
        let ComponentTokens::Stepper(tokens) = theme.component(ComponentId::Stepper) else {
            unreachable!("ComponentId::Stepper must resolve to StepperTokens")
        };
        let c = theme.colors();
        FluentStepperStyle {
            design_system: "fluent",
            step_size: tokens.step_size,
            connector_width: tokens.connector_width,
            gap: tokens.gap,
            label_font_size: tokens.label_font_size,
            primary: c.primary,
            on_primary: c.on_primary,
            surface: c.surface,
            on_surface: c.on_surface,
            on_surface_variant: c.on_surface_variant,
            outline: c.outline,
            outline_variant: c.outline_variant,
            marker_font_size: MARKER_FONT_SIZE,
            marker_radius: MARKER_RADIUS,
            marker_border: MARKER_BORDER,
            item_gap: ITEM_GAP,
            padding_horiz: PADDING_HORIZ,
            padding_vert: PADDING_VERT,
            connector_min_width: CONNECTOR_MIN_WIDTH,
            check_scale: CHECK_SCALE,
        }
    }

    /// The stepper, ready to mount.
    pub fn view(self) -> FluentStepperWidget {
        let style = self.resolved_style();
        FluentStepperWidget {
            style,
            items: self.items,
            selected: self.selected.clone(),
            editable: self.props.editable,
            non_linear: self.props.non_linear,
            on_change: self.on_change,
            built_for: read_selected(&self.selected),
        }
    }
}

impl Default for FluentStepper {
    fn default() -> Self {
        Self::new()
    }
}

/// The stepper's widget: the row of steps, rebuilt when the selection moves.
///
/// The engine gap, in one type: the engine has no stepper, and the engine's
/// tree does not rebuild a subtree on its own, so the widget that owns the row
/// asks for the rebuild.
pub struct FluentStepperWidget {
    style: FluentStepperStyle,
    items: Vec<String>,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    /// The selection the row was last built for.
    built_for: usize,
}

impl FluentStepperWidget {
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

impl Widget for FluentStepperWidget {
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
                move || {
                    build_row(
                        style,
                        items.clone(),
                        selected.clone(),
                        editable,
                        non_linear,
                        on_change.clone(),
                    )
                }
            },
        )]
    }

    /// v1's outer container: the full width, padded 8 across and 12 down.
    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .padding(Insets {
                    left: f64::from(self.style.padding_horiz),
                    right: f64::from(self.style.padding_horiz),
                    top: f64::from(self.style.padding_vert),
                    bottom: f64::from(self.style.padding_vert),
                }),
        )
    }
}

impl WidgetAny for FluentStepperWidget {
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
    style: FluentStepperStyle,
    items: Vec<String>,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
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
            move || {
                build_step(
                    style,
                    index,
                    label.clone(),
                    selected.clone(),
                    editable,
                    non_linear,
                    on_change.clone(),
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
    style: FluentStepperStyle,
    index: usize,
    label: String,
    selected: SharedMap,
    editable: bool,
    non_linear: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
) -> Box<dyn WidgetAny> {
    let current = read_selected(&selected);
    let completed = index < current;
    let done = current >= index;

    // The marker's colours: a step that is current or behind is filled; one
    // that is still ahead is not.
    let (background, foreground, border) = if done {
        (style.primary, style.on_primary, style.primary)
    } else {
        (
            style.surface,
            style.on_surface_variant,
            style.outline,
        )
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

    // The tick's path, resolved once: the glyph does not change with the
    // selection, only the step it is drawn in does.
    let check_path = FluentIcon::new("check").path_data();

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
                        .background(to_paint(background))
                        .border(style.marker_border, to_paint(border))
                        .push({
                            let style = style;
                            let completed = completed;
                            let index = index;
                            let foreground = foreground;
                            let check_path = check_path;
                            move || -> Box<dyn WidgetAny> {
                                if completed {
                                    // v1 chose between `on_primary` and
                                    // `on_surface_variant` for the tick, and the
                                    // tick is only drawn for a step that is
                                    // behind the current one -- where the choice
                                    // is always `on_primary`.
                                    Box::new(
                                        VectorIcon::from_path_data(check_path, VIEW_BOX)
                                            .size(style.step_size * style.check_scale)
                                            .color(to_paint(style.on_primary)),
                                    )
                                } else {
                                    Box::new(
                                        Text::new(format!("{}", index + 1))
                                            .size(style.marker_font_size)
                                            .weight(FontWeight::Medium)
                                            .color(to_paint(foreground)),
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
                        .color(to_paint(label_color)),
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
fn build_connector(
    style: FluentStepperStyle,
    index: usize,
    selected: SharedMap,
) -> Box<dyn WidgetAny> {
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
            .background(to_paint(color)),
    ) as Box<dyn WidgetAny>
}
