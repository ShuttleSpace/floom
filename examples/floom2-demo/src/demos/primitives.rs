//! Primitives: the leaves everything else is built from.

use floom_core::{
    Axis, Child, ChildKey, Color, LayoutRequest, PaintContext, Point, Shadow, WidgetAny,
};
use floom_types::{Brush, Path, Stroke};
use floom_widgets::{Canvas, Container, Divider, Icon, Spacer, Stack, Text};

use super::{Demo, Section};

/// A `Child` whose builder is written inline, so a demo can keep its tree in
/// one expression without naming a function per leaf.
macro_rules! leaf {
    ($layout:expr, $build:expr) => {
        Child::new(ChildKey::index(0), $layout, $build)
    };
}

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "text",
            title: "Text",
            section: Section::Primitives,
            build: build_text,
        },
        Demo {
            id: "icon",
            title: "Icon",
            section: Section::Primitives,
            build: build_icon,
        },
        Demo {
            id: "divider",
            title: "Divider & Spacer",
            section: Section::Primitives,
            build: build_divider,
        },
        Demo {
            id: "container",
            title: "Container",
            section: Section::Primitives,
            build: build_container,
        },
        Demo {
            id: "canvas",
            title: "Canvas",
            section: Section::Primitives,
            build: build_canvas,
        },
    ]
}

fn build_text() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .push_layout(LayoutRequest::new(), || {
                Box::new(Text::new("Display — 24 pt").size(24.0))
            })
            .push_layout(LayoutRequest::new(), || {
                Box::new(Text::new("Body — the default 14 pt").size(14.0))
            })
            .push_layout(LayoutRequest::new(), || {
                Box::new(Text::new("Caption — 11 pt").size(11.0))
            }),
    )
}

fn build_icon() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Horizontal)
            .gap(14.0)
            .push_layout(LayoutRequest::new().width(28.0).height(28.0), || {
                Box::new(Icon::new('★').size(24.0).color(Color::rgb(230, 160, 30)))
            })
            .push_layout(LayoutRequest::new().width(28.0).height(28.0), || {
                Box::new(Icon::new('♥').size(24.0).color(Color::rgb(210, 60, 90)))
            })
            .push_layout(LayoutRequest::new().width(28.0).height(28.0), || {
                Box::new(Icon::new('✓').size(24.0).color(Color::rgb(50, 160, 90)))
            }),
    )
}

fn build_divider() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .push_layout(LayoutRequest::new(), || Box::new(Text::new("above")))
            .push_layout(LayoutRequest::new().height(1.0), || Box::new(Divider::new()))
            .push_layout(LayoutRequest::new().width(120.0).height(16.0), || {
                Box::new(Spacer::new(16.0))
            })
            .push_layout(LayoutRequest::new(), || Box::new(Text::new("below")))
            .push_layout(LayoutRequest::new().width(90.0).height(1.0), || {
                Box::new(Divider::new())
            }),
    )
}

/// A filled, rounded box: the shape every demo in this section is about.
fn swatch(color: Color, radius: f32) -> Container {
    Container::new().background(color).radius(radius)
}

fn build_container() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Horizontal)
            .gap(14.0)
            .child(leaf!(
                LayoutRequest::new().width(56.0).height(40.0),
                || Box::new(swatch(Color::rgb(70, 120, 220), 4.0))
            ))
            .child(leaf!(
                LayoutRequest::new().width(56.0).height(40.0),
                || Box::new(swatch(Color::rgb(70, 120, 220), 20.0))
            ))
            .child(leaf!(
                LayoutRequest::new().width(56.0).height(40.0),
                || Box::new(
                    Container::new()
                        .background(Color::WHITE)
                        .radius(8.0)
                        .shadow(Shadow {
                            color: Color::rgba(0, 0, 0, 160),
                            blur: 5.0,
                            offset_x: 0.0,
                            offset_y: 5.0,
                            spread: 0.0,
                        })
                )
            ))
            .child(leaf!(
                LayoutRequest::new().width(56.0).height(40.0),
                || Box::new(
                    Container::new()
                        .background(Brush::linear(
                            &[
                                (0.0, Color::rgb(30, 60, 220)),
                                (1.0, Color::rgb(220, 60, 30)),
                            ],
                            Point::new(0.0, 0.0),
                            Point::new(56.0, 0.0),
                        ))
                        .radius(8.0)
                )
            )),
    )
}

fn build_canvas() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Horizontal).gap(14.0).child(leaf!(
            LayoutRequest::new().width(140.0).height(80.0),
            || {
                Box::new(
                    Canvas::new(|cx: &mut dyn PaintContext| {
                        let b = cx.bounds();
                        let bars = [0.35, 0.7, 0.5, 0.9, 0.25];
                        let gap = 4.0;
                        let w =
                            (b.width() - gap * (bars.len() as f64 - 1.0)) / bars.len() as f64;
                        for (i, value) in bars.iter().enumerate() {
                            let x = b.x0 + i as f64 * (w + gap);
                            let top = b.y1 - b.height() * value;
                            let mut bar = Path::new();
                            bar.move_to(Point::new(x, b.y1));
                            bar.line_to(Point::new(x, top));
                            bar.line_to(Point::new(x + w, top));
                            bar.line_to(Point::new(x + w, b.y1));
                            bar.close();
                            cx.fill_path(&bar, Color::rgb(70, 120, 220));
                        }
                        let mut axis = Path::new();
                        axis.move_to(Point::new(b.x0, b.y1));
                        axis.line_to(Point::new(b.x1, b.y1));
                        cx.stroke_path(
                            &axis,
                            &Stroke::new(1.0, Color::rgb(140, 140, 140)),
                        );
                    })
                    .size(140.0, 80.0)
                    .label("a five-bar chart"),
                )
            }
        )),
    )
}
