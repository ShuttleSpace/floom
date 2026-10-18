//! Controls: the interactive widgets.

use floom_core::{Axis, Color, LayoutRequest, WidgetAny};
use floom_widgets::{Button, Progress, Spinner, Stack, Switch};

use super::{Demo, Section};

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "buttons",
            title: "Button",
            section: Section::Controls,
            build: build_buttons,
        },
        Demo {
            id: "toggles",
            title: "Switch",
            section: Section::Controls,
            build: build_toggles,
        },
        Demo {
            id: "progress",
            title: "Progress",
            section: Section::Controls,
            build: build_progress,
        },
        Demo {
            id: "spinner",
            title: "Spinner",
            section: Section::Controls,
            build: build_spinner,
        },
    ]
}

fn build_buttons() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Primary").on_click(|| {}))
            })
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Accented").color(Color::rgb(70, 120, 220)).on_click(|| {}))
            })
            .push_layout(LayoutRequest::new().width(160.0).height(40.0), || {
                Box::new(Button::new("Disabled").disabled(true).on_click(|| {}))
            }),
    )
}

/// `TextField` had a demo row here until it moved to `floom-ui-composites`: it is
/// the widest of the moved controls, because what a text field *looks like* is
/// four kinds of ink and a focus affordance, and the engine's paint had no way
/// to name any of them. `floom-demo` cannot depend on the satellite `floom`
/// workspace to keep a gallery row, so the row is gone.
/// The last of the three toggles, and the only one still a primitive.
///
/// `Radio` sat here first and left for `floom-ui-composites`: it is the one whose
/// appearance is contested (a stroked ring in every kit, a filled disc in the
/// engine). `Checkbox` followed it, and is the wider of the two disagreements --
/// the engine painted a bare fill with a font glyph and no rim, where every kit
/// paints a 2px rim, a transparent unchecked interior and a vector mark, and
/// where two of the three draw a focus ring and one answers focus with a state
/// layer instead. `floom-demo` cannot depend on the satellite `floom-ui` workspace
/// to keep a demo row, so the row is now one widget wide; the gallery still
/// builds, which is what the demo is for.
fn build_toggles() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(200.0).height(28.0), || {
                Box::new(Switch::new("Switch").on(true))
            }),
    )
}

fn build_progress() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .push_layout(LayoutRequest::new().width(220.0).height(8.0), || {
                Box::new(Progress::determinate(0.6))
            })
            .push_layout(LayoutRequest::new().width(220.0).height(8.0), || {
                Box::new(Progress::indeterminate())
            }),
    )
}

fn build_spinner() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Horizontal)
            .gap(16.0)
            .push_layout(LayoutRequest::new().width(32.0).height(32.0), || {
                Box::new(Spinner::new().size(28.0).line_width(3.0))
            })
            .push_layout(LayoutRequest::new().width(32.0).height(32.0), || {
                Box::new(
                    Spinner::new()
                        .size(28.0)
                        .line_width(3.0)
                        .color(Color::rgb(70, 120, 220)),
                )
            }),
    )
}
