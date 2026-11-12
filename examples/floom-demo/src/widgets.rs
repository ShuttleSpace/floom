//! The gallery's chrome: the building blocks the shell and the pages draw
//! with.
//!
//! Everything here is an engine primitive -- `Stack`, `Button`, `Text`,
//! `Surface` -- rather than a `floom-ui` component. The chrome has to look
//! the same whichever design system is being previewed, and it must never
//! compete with the component being previewed, so it stays out of the
//! satellite's palette entirely.

use std::sync::Arc;

use floom::floom_core::{Align, Axis, Child, ChildKey, LayoutRequest};
use floom::floom_types::{Color, FontWeight};
use floom::floom_widgets::{Button, Stack, Surface, Text};

use crate::api::ApiRow;

/// The line between two things.
pub const BORDER: Color = Color::rgb(0xE1, 0xE5, 0xEB);
/// The rail's background.
pub const SIDEBAR: Color = Color::rgb(0xF4, 0xF6, 0xF8);
/// The accent: the selected entry, and the brand.
pub const ACCENT: Color = Color::rgb(0x2B, 0x6C, 0xB0);
/// The quietest ink: captions and section labels.
pub const FAINT: Color = Color::rgb(0x8A, 0x91, 0x9E);
/// The darkest ink: headings.
pub const INK: Color = Color::rgb(0x1A, 0x1D, 0x23);
/// The middle ink: body text.
pub const MUTED: Color = Color::rgb(0x5C, 0x63, 0x70);

/// A single-styled text leaf.
pub fn label(
    key: &'static str,
    content: impl Into<String>,
    size: f32,
    weight: FontWeight,
    color: Color,
) -> Child {
    let content = content.into();
    Child::new(ChildKey::from(key), LayoutRequest::new(), move || {
        Box::new(
            Text::new(content.clone())
                .size(size)
                .weight(weight)
                .color(color),
        )
    })
}

/// A label that takes the full width of its column, so the host's text
/// measure wraps it. A bare [`label`] keeps its intrinsic width, which is one
/// long line and a flex item that cannot shrink.
pub fn paragraph(key: &'static str, content: impl Into<String>, size: f32, color: Color) -> Child {
    let content = content.into();
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0),
        move || {
            Box::new(
                Text::new(content.clone())
                    .size(size)
                    .weight(FontWeight::Normal)
                    .color(color),
            )
        },
    )
}

/// A caption above a group of things.
pub fn section_label(key: &'static str, content: &'static str) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().height(26.0),
        move || {
            Box::new(
                Text::new(content)
                    .size(10.0)
                    .weight(FontWeight::SemiBold)
                    .color(FAINT),
            )
        },
    )
}

/// Empty space.
pub fn spacer(key: &'static str, height: f64) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0).height(height as f32),
        move || Box::new(floom::floom_widgets::Spacer::new(height)),
    )
}

/// A bordered panel around a body.
pub fn panel(key: &'static str, body: Stack) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0),
        move || {
            let body = body.clone();
            Box::new(
                Surface::new()
                    .background(Color::WHITE)
                    .border(1.0, BORDER)
                    .radius(10.0)
                    .request(LayoutRequest::new().min_width(0.0))
                    .padding(floom::floom_types::Insets::uniform(16.0))
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0),
                        move || Box::new(body.clone()),
                    )),
            )
        },
    )
}

/// One entry in the rail.
///
/// One entry in the rail.
///
/// The entry is the page's own name, so a reader -- and a test -- finds a
/// component by the name they know it by.
pub fn nav_item(
    key: &'static str,
    text: &'static str,
    active: bool,
    on_click: impl Fn() + Send + Sync + 'static,
) -> Child {
    let on_click = Arc::new(on_click);
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0).height(28.0),
        move || {
            let on_click = on_click.clone();
            Box::new(
                Button::new(text)
                    .color(if active { ACCENT } else { Color::WHITE })
                    .text_color(if active { Color::WHITE } else { INK })
                    .radius(6.0)
                    .on_click(move || (on_click)()),
            )
        },
    )
}

/// An API table: a heading, then one row per entry.
pub fn table(key: &'static str, title: &str, rows: &[ApiRow]) -> Stack {
    let mut stack = Stack::new(Axis::Vertical).gap(4.0);
    stack = stack.child(label(key, title, 13.0, FontWeight::SemiBold, INK));

    let mut header = Stack::new(Axis::Horizontal).gap(14.0);
    header = header.child(label(
        "column-name",
        "Name",
        10.0,
        FontWeight::SemiBold,
        FAINT,
    ));
    header = header.child(label(
        "column-type",
        "Type",
        10.0,
        FontWeight::SemiBold,
        FAINT,
    ));
    header = header.child(label(
        "column-default",
        "Default",
        10.0,
        FontWeight::SemiBold,
        FAINT,
    ));
    header = header.child(label(
        "column-description",
        "Description",
        10.0,
        FontWeight::SemiBold,
        FAINT,
    ));
    let header_key = format!("{key}-header");
    stack = stack.child(Child::new(
        ChildKey::from(header_key.as_str()),
        LayoutRequest::new().width_pct(100.0),
        move || Box::new(header.clone()),
    ));

    for (index, row) in rows.iter().enumerate() {
        let name = row.name.clone();
        let type_ = row.type_.clone();
        let default = row.default.clone();
        let description = row.description.clone();

        let mut line = Stack::new(Axis::Horizontal)
            .gap(14.0)
            .align(Align::Start, Align::Start);
        line = line.child(label("name", name.clone(), 12.0, FontWeight::SemiBold, INK));
        line = line.child(label(
            "type",
            type_.clone(),
            12.0,
            FontWeight::Normal,
            MUTED,
        ));
        line = line.child(label(
            "default",
            default.clone(),
            12.0,
            FontWeight::Normal,
            MUTED,
        ));
        line = line.child(label(
            "description",
            description.clone(),
            12.0,
            FontWeight::Normal,
            MUTED,
        ));

        let row_key = format!("{key}-row-{index}");
        stack = stack.child(Child::new(
            ChildKey::from(row_key.as_str()),
            LayoutRequest::new().width_pct(100.0),
            move || Box::new(line.clone()),
        ));
    }

    stack
}

/// One column of a compare row: a caption above the component.
///
/// `child` is cloned for each build call because a `Child`'s closure may run
/// more than once; cloning keeps every rebuild independent.
pub fn column(caption: &'static str, child: Child) -> Child {
    Child::new(
        ChildKey::from(caption),
        LayoutRequest::new().fill(),
        move || {
            let child = child.clone();
            Box::new(
                Stack::new(Axis::Vertical)
                    .gap(6.0)
                    .child(label("caption", caption, 11.0, FontWeight::SemiBold, FAINT))
                    .child(child),
            )
        },
    )
}

/// Lay three children out as the three columns of a compare row, captioned by
/// the design system each shows.
pub fn compare_row(children: (Child, Child, Child)) -> Stack {
    let (material, cupertino, fluent) = children;
    Stack::new(Axis::Horizontal)
        .gap(28.0)
        .child(column("Material", material))
        .child(column("Cupertino", cupertino))
        .child(column("Fluent", fluent))
}
