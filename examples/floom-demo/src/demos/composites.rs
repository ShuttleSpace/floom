//! Composites: widgets that arrange or swap other widgets.

use floom_core::{Axis, Child, ChildKey, Color, LayoutRequest, WidgetAny};
use floom_widgets::{Conditional, Container, Grid, List, Stack, Text};

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
            id: "conditional",
            title: "Conditional & Layers",
            section: Section::Composites,
            build: build_conditional,
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

/// A `Conditional` that is on, beside a layer covering its parent's box.
///
/// The layer is an absolute-fill `Container` stacked over the content — the
/// engine's own in-tree overlay vocabulary, and all a root-workspace user
/// needs for "something painted above this box".
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
                        .push_layout(LayoutRequest::new().absolute_fill(), || {
                            Box::new(
                                Container::new()
                                    .background(Color::rgba(70, 120, 220, 90))
                                    .radius(6.0),
                            )
                        }),
                )
            }),
    )
}
