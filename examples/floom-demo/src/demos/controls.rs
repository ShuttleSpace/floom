//! Controls: the interactive widgets.

use floom_core::{Axis, Color, LayoutRequest, WidgetAny};
use floom_widgets::{Button, Checkbox, Progress, Radio, Spinner, Stack, Switch, TextField};

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
            id: "text-field",
            title: "TextField",
            section: Section::Controls,
            build: build_text_field,
        },
        Demo {
            id: "toggles",
            title: "Checkbox, Radio & Switch",
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

fn build_text_field() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(260.0).height(36.0), || {
                Box::new(TextField::new().placeholder("empty field"))
            })
            .push_layout(LayoutRequest::new().width(260.0).height(36.0), || {
                Box::new(TextField::new().value("prefilled text"))
            })
            .push_layout(LayoutRequest::new().width(260.0).height(36.0), || {
                Box::new(TextField::new().value("read-only").disabled(true))
            }),
    )
}

fn build_toggles() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new().width(200.0).height(28.0), || {
                Box::new(Checkbox::new("Checkbox").checked(true))
            })
            .push_layout(LayoutRequest::new().width(200.0).height(28.0), || {
                Box::new(Radio::new("Radio").selected(true))
            })
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
