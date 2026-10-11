//! Composites: widgets that arrange or swap other widgets.

use floom_core::{Axis, Child, ChildKey, Color, LayoutRequest, WidgetAny};
use floom_widgets::{
    BottomSheet, Button, Conditional, Container, Dialog, Grid, List, Overlay, Popup, Stack, Tabs,
    Text,
};

use super::{Demo, Section};

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "list",
            title: "List",
            section: Section::Composites,
            build: build_list,
        },
        Demo {
            id: "grid",
            title: "Grid",
            section: Section::Composites,
            build: build_grid,
        },
        Demo {
            id: "tabs",
            title: "Tabs",
            section: Section::Composites,
            build: build_tabs,
        },
        Demo {
            id: "conditional",
            title: "Conditional & Overlay",
            section: Section::Composites,
            build: build_conditional,
        },
        Demo {
            id: "dialog",
            title: "Dialog, Popup & BottomSheet",
            section: Section::Composites,
            build: build_dialog,
        },
    ]
}

fn item_key(item: &String) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in item.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn items() -> Vec<String> {
    (1..=6).map(|n| format!("Row {n}")).collect()
}

fn build_list() -> Box<dyn WidgetAny> {
    let items = items();
    Box::new(
        Stack::new(Axis::Vertical).push_layout(LayoutRequest::new(), move || {
            Box::new(
                List::<String>::new(Axis::Vertical, items.clone(), item_key, |item: String| {
                    Box::new(Text::new(item))
                })
                .gap(6.0),
            )
        }),
    )
}

fn build_grid() -> Box<dyn WidgetAny> {
    let items = items();
    Box::new(
        Stack::new(Axis::Vertical).push_layout(LayoutRequest::new(), move || {
            Box::new(
                Grid::<String>::new(3, items.clone(), item_key, |item: String| {
                    Box::new(Text::new(item))
                })
                .gap(8.0, 8.0)
                .item_layout(LayoutRequest::new().width(72.0).height(28.0)),
            )
        }),
    )
}

fn build_tabs() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(LayoutRequest::new().height(120.0), || {
            Box::new(
                Tabs::new()
                    .tab(
                        "One",
                        Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                            Box::new(Text::new("first panel"))
                        }),
                    )
                    .tab(
                        "Two",
                        Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                            Box::new(Text::new("second panel"))
                        }),
                    ),
            )
        }),
    )
}

/// A `Conditional` that is on, beside an `Overlay` covering its parent's box.
/// Both build their subtrees inside their own builder, so nothing but a plain
/// `fn` is captured.
fn build_conditional() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .push_layout(LayoutRequest::new().height(24.0), || {
                Box::new(Conditional::new(true).show(Child::new(
                    ChildKey::index(0),
                    LayoutRequest::new(),
                    || Box::new(Text::new("shown when true")),
                )))
            })
            .push_layout(LayoutRequest::new().width(220.0).height(56.0), || {
                Box::new(
                    Stack::new(Axis::Vertical)
                        .push_layout(LayoutRequest::new(), || {
                            Box::new(Text::new("under the layer"))
                        })
                        .push_layout(LayoutRequest::new(), || {
                            Box::new(Overlay::new().content(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new(),
                                || {
                                    Box::new(
                                        Container::new()
                                            .background(Color::rgba(70, 120, 220, 90))
                                            .radius(6.0),
                                    )
                                },
                            )))
                        }),
                )
            }),
    )
}

/// Modals are *shown* by the app, not by the gallery: leaving one present would
/// bury every other demo, so this row carries the trigger and the three
/// surfaces are declared hidden. Their bodies still build, which is what keeps
/// this demo honest about what the widgets need.
fn build_dialog() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .push_layout(LayoutRequest::new().width(160.0).height(36.0), || {
                Box::new(Button::new("Open dialog").on_click(|| {}))
            })
            .push_layout(LayoutRequest::new(), || {
                let mut dialog = Dialog::new().title("Dialog").body(dialog_body());
                dialog.set_present(false);
                Box::new(dialog)
            })
            .push_layout(LayoutRequest::new(), || {
                let mut popup = Popup::new().content(dialog_body());
                popup.set_present(false);
                Box::new(popup)
            })
            .push_layout(LayoutRequest::new(), || {
                let mut sheet = BottomSheet::new()
                    .title("Sheet")
                    .height(180.0)
                    .body(dialog_body());
                sheet.set_present(false);
                Box::new(sheet)
            }),
    )
}

/// The one body all three surfaces are given.
fn dialog_body() -> Child {
    Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("modal body"))
    })
}
