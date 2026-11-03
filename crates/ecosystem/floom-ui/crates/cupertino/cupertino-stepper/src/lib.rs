//! Cupertino stepper, ported to the Floom 2.0 widget API.
//!

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
