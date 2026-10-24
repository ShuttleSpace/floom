//! Cupertino stepper, ported to the Floom 2.0 widget API.
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
//! ## The engine's stepper
//!
//! The one thing the vocabulary cannot do on its own is *follow the
//! selection*. 2.0's tree is built once: a child's builder closure runs when
//! its slot is mounted, and a slot that is still there is not built again.
//! v1's stepper read its `RwSignal<usize>` inside a style closure, so the
//! row's colours followed the selection as it moved. Here the colours are
//! decided when the row is built, so the widget reads the selection in
//! `update` and asks the host to reconcile its children -- and the row's slot
//! key carries the selection, so the row that is rebuilt is a new row.
//!
//! That widget is the engine's now: `floom_ui_composites::Stepper` is a row
//! of steps, one of them current, and the three kits use it. The one thing
//! the engine cannot know is the glyph a completed step carries: it is a
//! design system's own icon, so the kit hands the stepper the path data and
//! the box it is drawn in.
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
//! Cupertino takes a snapshot of its own design system, which is what v1 did: a
//! change of the app-wide theme does not reach this stepper. Material reads
//! the app-wide theme signal instead, so that a change of theme re-themes it
//! along with everything else.

use std::collections::HashMap;
use std::sync::Arc;

use floom_signal::{RwSignal, SignalGet};
use floom_ui_composites::{Stepper, StepperStyle, StepperTick};
use floom_widgets::{shared_map, SharedMap};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::props::stepper::StepperProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::to_engine_color as to_paint;
use floom_core_theme::tokens::color::Color;
use floom_types::Color as PaintColor;
use floom_widget_cupertino_icon::{CupertinoIcon, VIEW_BOX};

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
pub struct CupertinoStepperStyle {
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

/// Cupertino's stepper.
pub struct CupertinoStepper {
    props: StepperProps,
    theme: RwSignal<MultiTheme>,
    /// The step labels.
    items: Vec<String>,
    /// The selected step, shared with whoever drives the stepper.
    selected: SharedMap,
    /// The change callback, in the shape a 2.0 slot needs.
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
}

impl CupertinoStepper {
    pub fn new() -> Self {
        Self {
            props: StepperProps::default(),
            // v1's Cupertino stepper took a *snapshot* of its own design system, so
            // a change of the app-wide theme does not reach it.
            theme: RwSignal::new(MultiTheme::new_for(
                floom_core_theme::global_theme::DesignSystem::Cupertino,
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
    pub fn selected(self, selected: usize) -> Self {
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

    pub fn resolved_style(&self) -> CupertinoStepperStyle {
        let theme = self.theme.get();
        let ComponentTokens::Stepper(tokens) = theme.component(ComponentId::Stepper) else {
            unreachable!("ComponentId::Stepper must resolve to StepperTokens")
        };
        let c = theme.colors();
        CupertinoStepperStyle {
            design_system: "cupertino",
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
    pub fn view(self) -> Stepper {
        let style = self.resolved_style();
        let mut stepper = Stepper::new()
            .style(StepperStyle {
                step_size: style.step_size,
                connector_width: style.connector_width,
                gap: style.gap,
                label_font_size: style.label_font_size,
                primary: to_paint(style.primary),
                on_primary: to_paint(style.on_primary),
                surface: to_paint(style.surface),
                on_surface: to_paint(style.on_surface),
                on_surface_variant: to_paint(style.on_surface_variant),
                outline: to_paint(style.outline),
                outline_variant: to_paint(style.outline_variant),
                marker_font_size: style.marker_font_size,
                marker_radius: style.marker_radius,
                marker_border: style.marker_border,
                item_gap: style.item_gap,
                padding_horiz: style.padding_horiz,
                padding_vert: style.padding_vert,
                connector_min_width: style.connector_min_width,
                check_scale: style.check_scale,
            })
            .labels(self.items)
            .selection(self.selected.clone())
            .editable(self.props.editable)
            .non_linear(self.props.non_linear)
            .tick(StepperTick {
                path: CupertinoIcon::new("check").path_data(),
                view_box: VIEW_BOX,
            });
        if let Some(on_change) = self.on_change {
            stepper = stepper.on_change(move |index| on_change(index));
        }
        stepper
    }
}

impl Default for CupertinoStepper {
    fn default() -> Self {
        Self::new()
    }
}
