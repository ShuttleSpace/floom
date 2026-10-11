//! Styling: what a filled box can be made of, and the design-system tokens.
//!
//! The first demo is the four ways a `Container` renders (flat, gradient,
//! elevated, blurred); the second is the same two roles read from each design
//! system, so the four palettes can be compared side by side.

use floom_core::{Axis, Color, LayoutRequest, Point, Shadow, WidgetAny};
use floom_types::Brush;
use floom_widgets::{Container, Stack, Text};

use super::{Demo, Section};

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "container-style",
            title: "Gradient, elevation & blur",
            section: Section::Styling,
            build: build_containers,
        },
        Demo {
            id: "themes",
            title: "Design-system palettes",
            section: Section::Styling,
            build: build_themes,
        },
    ]
}

fn swatch(color: Color, radius: f32) -> Container {
    Container::new().background(color).radius(radius)
}

fn build_containers() -> Box<dyn WidgetAny> {
    let ramp = Brush::linear(
        &[(0.0, Color::rgb(30, 60, 220)), (1.0, Color::rgb(220, 60, 30))],
        Point::new(0.0, 0.0),
        Point::new(64.0, 0.0),
    );

    Box::new(
        Stack::new(Axis::Horizontal)
            .gap(16.0)
            .push_layout(LayoutRequest::new().width(64.0).height(48.0), || {
                Box::new(swatch(Color::rgb(70, 120, 220), 6.0))
            })
            .push_layout(LayoutRequest::new().width(64.0).height(48.0), move || {
                Box::new(Container::new().background(ramp.clone()).radius(6.0))
            })
            .push_layout(LayoutRequest::new().width(64.0).height(48.0), || {
                Box::new(
                    Container::new()
                        .background(Color::WHITE)
                        .radius(6.0)
                        .shadow(Shadow {
                            color: Color::rgba(0, 0, 0, 170),
                            blur: 6.0,
                            offset_x: 0.0,
                            offset_y: 6.0,
                            spread: 0.0,
                        }),
                )
            })
            .push_layout(LayoutRequest::new().width(64.0).height(48.0), || {
                Box::new(
                    Container::new()
                        .background(Color::rgba(70, 120, 220, 200))
                        .radius(6.0)
                        .blur(6.0),
                )
            }),
    )
}

/// One design system per row: the accent, a container fill, and the error role
/// — the three a control actually needs.
fn build_themes() -> Box<dyn WidgetAny> {
    let material = floom_material::MaterialTheme::light();
    let cupertino = floom_cupertino::CupertinoTheme::light();
    let fluent = floom_fluent::FluentTheme::light();
    let harmony = floom_harmony::HarmonyTheme::light();

    let rows: [(&'static str, [Color; 3]); 4] = [
        (
            "Material",
            [
                material.roles.primary,
                material.roles.primary_container,
                material.roles.error,
            ],
        ),
        (
            "Cupertino",
            [
                cupertino.colors.tint,
                cupertino.colors.fill,
                cupertino.colors.destructive,
            ],
        ),
        (
            "Fluent",
            [
                fluent.colors.accent,
                fluent.colors.card_background,
                fluent.colors.stroke_default,
            ],
        ),
        (
            "Harmony",
            [
                harmony.colors.brand,
                harmony.colors.card,
                harmony.colors.error,
            ],
        ),
    ];

    let mut column = Stack::new(Axis::Vertical).gap(12.0);
    for (name, colors) in rows {
        let mut row = Stack::new(Axis::Horizontal).gap(10.0);
        row = row.push_layout(LayoutRequest::new().width(80.0).height(24.0), move || {
            Box::new(Text::new(name).size(12.0))
        });
        for color in colors {
            row = row.push_layout(
                LayoutRequest::new().width(48.0).height(24.0),
                move || Box::new(swatch(color, 4.0)),
            );
        }
        column = column.push_layout(LayoutRequest::new().height(24.0), move || {
            Box::new(row.clone())
        });
    }
    Box::new(column)
}
