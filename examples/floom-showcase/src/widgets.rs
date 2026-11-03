//! Small reusable building blocks: labels, cards, buttons, signal-backed text.
//!
//! Depends only on `crate::routes` (for `go_button`) and external crates.

use std::sync::Arc;

use floom::floom_core::{
    Child, ChildKey, CreateContext, ElementId, LayoutRequest, SemanticsHint, UpdateContext,
    Widget, WidgetAny,
};
use floom::floom_types::{Color, FontWeight, Insets};
use floom::floom_widgets::{Button, Spacer, Stack, Surface, Text};

use crate::routes::{AppRoute, NavIntent};

/// Shared theme constants.
pub const BORDER: Color = Color::rgb(0xE1, 0xE5, 0xEB);
pub const SIDEBAR: Color = Color::rgb(0xF4, 0xF6, 0xF8);
pub const ACCENT: Color = Color::rgb(0x2B, 0x6C, 0xB0);
pub const FAINT: Color = Color::rgb(0x8A, 0x91, 0x9E);
pub const INK: Color = Color::rgb(0x1A, 0x1D, 0x23);
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

/// A label that takes the full width of its column so the host's text measure
/// wraps it. A bare `label` keeps its intrinsic width, which is one long line
/// and a flex item that cannot shrink.
pub fn paragraph(
    key: &'static str,
    content: impl Into<String>,
    size: f32,
    weight: FontWeight,
    color: Color,
) -> Child {
    let content = content.into();
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0),
        move || {
            Box::new(
                Text::new(content.clone())
                    .size(size)
                    .weight(weight)
                    .color(color),
            )
        },
    )
}

pub fn section_label(key: &'static str, content: &'static str) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().height(28.0),
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

pub fn spacer(key: &'static str, height: f64) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0).height(height as f32),
        move || Box::new(Spacer::new(height)),
    )
}

pub fn card(key: &'static str, height: f32, body: Stack) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().height(height),
        move || {
            let body = body.clone();
            Box::new(
                Surface::new()
                    .background(Color::WHITE)
                    .border(1.0, BORDER)
                    .radius(10.0)
                    .request(LayoutRequest::new().min_width(0.0))
                    .padding(Insets::uniform(16.0))
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || Box::new(body.clone()),
                    )),
            )
        },
    )
}

pub fn button_child(
    key: &'static str,
    text: &'static str,
    layout: LayoutRequest,
    on_click: impl Fn() + Send + Sync + 'static,
) -> Child {
    let on_click = Arc::new(on_click);
    Child::new(ChildKey::from(key), layout, move || {
        let on_click = on_click.clone();
        Box::new(Button::new(text).radius(6.0).on_click(move || (on_click)()))
    })
}

pub fn disabled_button(key: &'static str, text: &'static str) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width(104.0).height(36.0),
        move || Box::new(Button::new(text).radius(6.0).disabled(true)),
    )
}

pub fn nav_item(
    key: &'static str,
    item: AppRoute,
    current: AppRoute,
    pending: Arc<std::sync::Mutex<Option<NavIntent>>>,
) -> Child {
    let active = item == current;
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0).height(34.0),
        move || {
            let pending = pending.clone();
            Box::new(
                Button::new(item.nav_label())
                    .color(if active { ACCENT } else { SIDEBAR })
                    .text_color(if active { Color::WHITE } else { INK })
                    .radius(6.0)
                    .on_click(move || {
                        *pending.lock().unwrap() = Some(NavIntent::Go(item));
                    }),
            )
        },
    )
}

pub fn go_button(
    key: &'static str,
    text: &'static str,
    route: AppRoute,
    pending: Arc<std::sync::Mutex<Option<NavIntent>>>,
) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width(94.0).height(32.0),
        move || {
            let pending = pending.clone();
            Box::new(Button::new(text).radius(6.0).on_click(move || {
                *pending.lock().unwrap() = Some(NavIntent::Go(route));
            }))
        },
    )
}

pub fn signal_text(
    key: &'static str,
    read: Arc<dyn Fn() -> String + Send + Sync>,
    size: f32,
    color: Color,
) -> Child {
    Child::new(ChildKey::from(key), LayoutRequest::new(), move || {
        Box::new(
            crate::widgets::SignalText::new(read.clone())
                .size(size)
                .color(color),
        )
    })
}

/// A `Text` whose content comes from a signal.
///
/// A `Text` child is rebuilt only when its parent re-declares children, so this
/// parent polls the signal in `update` and marks its child dirty on a change.
/// The host updates every node each frame, which is what makes the read cheap.
pub struct SignalText {
    read: Arc<dyn Fn() -> String + Send + Sync>,
    last: String,
    size: f32,
    color: Color,
}

impl SignalText {
    pub fn new(read: Arc<dyn Fn() -> String + Send + Sync>) -> Self {
        Self {
            read,
            last: String::new(),
            size: 13.0,
            color: INK,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Widget for SignalText {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let now = (self.read)();
        if now != self.last {
            self.last = now;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let content = (self.read)();
        let size = self.size;
        let color = self.color;
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new(),
            move || Box::new(Text::new(content.clone()).size(size).color(color)),
        )]
    }
}

impl WidgetAny for SignalText {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        None
    }
}
