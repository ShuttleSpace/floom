// SPDX-License-Identifier: MIT
//! Interactive rows for the Elements tree, and the context menu they open.
//!
//! ## Why a row widget and not `floom-ui-composites::tree_view`
//!
//! The satellite workspace has a `TreeView`, and it is not usable here for two
//! structural reasons. It lives under `crates/ecosystem`, which the root manifest
//! excludes and which is a nested workspace of its own — depending on it would
//! make this crate pull a whole design-system workspace in. More importantly its
//! rows are a *presentation* of a tree the caller owns; here the tree is the live
//! host's, and a row has to write back into the devtools state (collapse, select)
//! rather than report a selection up a callback chain.
//!
//! ## What a row is
//!
//! A row paints itself rather than composing `Text`s, for one reason: the
//! selected row needs a filled background *behind* its label, and this host paints
//! a flat scene in node order, so a sibling box cannot be drawn behind another
//! sibling's text. `fill_rect` then `fill_text` inside one widget is the only
//! ordering available.
//!
//! ## What a row does not do
//!
//! **No hover highlight.** The host never synthesises `PointerPhase::Leave` —
//! there is no such event in `floom-app`, `floom-runtime-core` or
//! `floom-platform-desktop` — so a row that painted itself on hover would stay
//! highlighted forever after the pointer left it, and un-highlighting the
//! *previous* row is not something a widget can ask for: `request_repaint` names
//! one element, and the previous row is a different element. The selection
//! background is the only state a row shows, and it is honest: it comes from the
//! row's own data.
//!
//! ## Why the menu is one widget and not a stack of `Button`s
//!
//! A `Button` reports its click through a callback that fires on *release* and
//! writes nothing itself, so a menu built from buttons would need the panel to
//! notice the write and rebuild before the menu could close — two frames for one
//! click, and a menu that lingers over the row it was about. The menu instead
//! resolves the entry from the press position, applies it to the shared state,
//! and closes its own overlay, all inside one dispatch.

#![forbid(unsafe_code)]

use std::sync::{Arc, Mutex};

use floom_core::{
    Child, ChildKey, Color, ElementId, Event, EventContext, FontWeight, LayoutRequest,
    OverlayHandle, OverlayKind, OverlayRequest, PaintContext, Point, PointerButton, PointerPhase,
    Rect, Size, UpdateContext, Widget, WidgetAny,
};
use floom_widgets::text_width_at;

use crate::panel::DevtoolsState;

/// The interactive half of a readout row: the node it stands for and the three
/// facts a row needs in order to draw and act on it.
///
/// Kept as plain data on [`crate::panel::ReadoutRow`] rather than read from the
/// shared cell at paint time, so a readout stays a pure function of the inspector
/// and the state, and a test can assert what a row *is* without a host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowNode {
    /// The live-tree key this row selects.
    pub key: String,
    /// Whether the node has children — which decides whether a chevron is drawn
    /// and whether the menu offers expand/collapse at all.
    pub has_children: bool,
    /// Whether the node's children are currently shown.
    pub expanded: bool,
    /// Whether this row is the element the style and layout readouts are about.
    pub selected: bool,
}

/// One row of the Elements tree, as a widget that can be clicked.
pub struct TreeRow {
    state: Arc<Mutex<DevtoolsState>>,
    node: RowNode,
    /// What tier the node is, drawn after its key in a dimmer ink.
    kind: String,
    /// How many levels deep the row sits; the indent is this times [`INDENT`].
    depth: usize,
}

/// Height of one row. Fixed rather than derived from the label: a tree whose rows
/// changed height with their content would make every row below jump when a key
/// got longer.
pub const ROW_HEIGHT: f64 = 20.0;

/// Horizontal step per level of depth.
pub const INDENT: f64 = 12.0;

/// Space before the chevron, so a row's content starts inside its own box.
pub const LEADING: f64 = 4.0;

/// The chevron's column, and its hit area.
pub const CHEVRON_WIDTH: f64 = 14.0;

/// Font size of a row's key.
pub const ROW_FONT: f32 = 11.0;

const TEXT: Color = Color::rgb(0xd8, 0xdc, 0xe4);
const MUTED: Color = Color::rgb(0x8a, 0x93, 0xa4);
const SELECTED_FILL: Color = Color::rgb(0x2c, 0x5b, 0x8a);

/// Top of a single line of text inside a box, so the glyphs sit centred rather
/// than against the top edge.
fn text_top(bounds: Rect, size: f32) -> f64 {
    bounds.y0 + (bounds.height() - size as f64) * 0.5
}

impl TreeRow {
    pub fn new(
        state: Arc<Mutex<DevtoolsState>>,
        node: RowNode,
        kind: impl Into<String>,
        depth: usize,
    ) -> Self {
        Self {
            state,
            node,
            kind: kind.into(),
            depth,
        }
    }

    /// The glyph in the chevron column: a filled triangle when the node has
    /// children, a space of the same width when it does not, so the keys of a
    /// leaf and of a parent line up.
    fn chevron(&self) -> char {
        if !self.node.has_children {
            ' '
        } else if self.node.expanded {
            '\u{25BE}'
        } else {
            '\u{25B8}'
        }
    }

    /// The x range a press has to land in to hit the chevron.
    fn chevron_span(&self, bounds: Rect) -> (f64, f64) {
        let start = bounds.x0 + LEADING + self.depth as f64 * INDENT;
        (start, start + CHEVRON_WIDTH)
    }

    /// Selects the row, or toggles it when the press landed on the chevron.
    ///
    /// Toggling is confined to the chevron on purpose: clicking the key of a
    /// parent selects it, which is what the Style and Layout readouts are for,
    /// and only the triangle collapses.
    ///
    /// Both writes land in the shared cell, which the panel only reads on its
    /// next `update`. A widget cannot mark the host dirty — `event` has no
    /// `UpdateContext` — so the write is followed by a repaint request for this
    /// element, which is the one channel that wakes the frame loop from outside
    /// it. The panel's next `update` then sees the new state and rebuilds its
    /// rows; this row's own repaint is the frame that gets it there.
    fn press(&self, cx: &dyn EventContext, id: ElementId, at: Point) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let (start, end) = self.chevron_span(cx.bounds());
        if self.node.has_children && at.x >= start && at.x < end {
            state.toggle_collapsed(&self.node.key);
        } else {
            state.select(self.node.key.clone());
        }
        drop(state);
        floom_core::request_repaint(id);
    }

    /// Opens the context menu at the pointer, clamped into the viewport.
    ///
    /// Non-modal so that a press anywhere else dismisses it through the host's
    /// ordinary outside-press path; a modal popup would swallow that press and
    /// leave the menu up until a second click.
    fn open_menu(&self, cx: &mut dyn EventContext, at: Point) {
        let items = self.menu_items();
        let bounds = menu_bounds(at, cx.viewport(), items.len());
        let state = self.state.clone();
        cx.open_overlay(OverlayRequest::new(
            OverlayKind::Popup,
            false,
            bounds,
            move |handle| Box::new(ContextMenu::new(state, items, handle)),
        ));
    }

    /// The entries this row's menu offers.
    ///
    /// Only actions that actually do something appear: a leaf gets no
    /// expand/collapse entry, and "clear selection" is absent when nothing is
    /// selected. A menu whose entries are dead on arrival is worse than a short
    /// one.
    fn menu_items(&self) -> Vec<MenuItem> {
        let mut items = Vec::new();
        if self.node.has_children {
            items.push(MenuItem::toggle(&self.node.key, self.node.expanded));
        }
        items.push(MenuItem::select(&self.node.key));
        let has_selection = self
            .state
            .lock()
            .map(|state| state.selected().is_some())
            .unwrap_or(false);
        if has_selection {
            items.push(MenuItem::clear());
        }
        items
    }
}

impl Widget for TreeRow {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(LayoutRequest::new().height(ROW_HEIGHT as f32))
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if self.node.selected {
            cx.fill_rounded_rect(bounds, 3.0, SELECTED_FILL);
        }
        let origin = Point::new(
            bounds.x0 + LEADING + self.depth as f64 * INDENT,
            text_top(bounds, ROW_FONT),
        );
        let chevron = self.chevron().to_string();
        cx.fill_text(origin, &chevron, ROW_FONT, FontWeight::Normal, MUTED);
        let key_x = origin.x + CHEVRON_WIDTH;
        let ink = if self.node.selected {
            Color::WHITE
        } else {
            TEXT
        };
        cx.fill_text(
            Point::new(key_x, origin.y),
            &self.node.key,
            ROW_FONT,
            FontWeight::Normal,
            ink,
        );
        if !self.kind.is_empty() {
            let kind_x = key_x + text_width_at(&self.node.key, ROW_FONT) as f64 + 8.0;
            cx.fill_text(
                Point::new(kind_x, origin.y),
                &self.kind,
                ROW_FONT,
                FontWeight::Normal,
                MUTED,
            );
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Down => {
                let Some(at) = cx.cursor_position() else {
                    return false;
                };
                match pointer.button {
                    Some(PointerButton::Primary) => self.press(cx, id, at),
                    Some(PointerButton::Secondary) => self.open_menu(cx, at),
                    _ => return false,
                }
                true
            }
            // A press that started on a row is not a press on whatever is behind
            // the panel: consuming the release keeps the app from seeing a click
            // it never got the press for.
            PointerPhase::Up => true,
            _ => false,
        }
    }

    fn cursor(&self) -> floom_types::CursorStyle {
        floom_types::CursorStyle::Pointer
    }
}

impl WidgetAny for TreeRow {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

/// Builds one interactive row as a child slot.
///
/// A free function rather than a method on the panel so the panel's column stays
/// a single `Stack` of slots and the row's own construction lives with the row.
pub fn row_child(
    key: u64,
    state: Arc<Mutex<DevtoolsState>>,
    node: RowNode,
    kind: String,
    depth: usize,
) -> Child {
    Child::new(ChildKey::keyed(key), LayoutRequest::new(), move || {
        Box::new(TreeRow::new(
            state.clone(),
            node.clone(),
            kind.clone(),
            depth,
        ))
    })
}

// ---------------------------------------------------------------------------
// The context menu
// ---------------------------------------------------------------------------

/// Width of the menu box.
pub const MENU_WIDTH: f64 = 176.0;

/// Height of one menu entry.
pub const MENU_ITEM_HEIGHT: f64 = 22.0;

/// Space above the first entry and below the last.
pub const MENU_PADDING: f64 = 4.0;

/// Where a menu of `item_count` entries opens for a press at `at`.
///
/// Flips left of the cursor when it would overhang the viewport's right edge, and
/// up when it would overhang the bottom — what a menu opened near a window corner
/// needs. `viewport == Rect::ZERO` means the host cannot say where its content is
/// visible; then the menu opens at the cursor and is allowed to overhang, because
/// guessing an edge would put it in a worse place.
pub fn menu_bounds(at: Point, viewport: Rect, item_count: usize) -> Rect {
    let width = MENU_WIDTH;
    let height = MENU_PADDING * 2.0 + MENU_ITEM_HEIGHT * item_count as f64;
    let mut x = at.x;
    let mut y = at.y;
    if viewport != Rect::ZERO {
        if x + width > viewport.x1 {
            x = (at.x - width).max(viewport.x0);
        }
        if y + height > viewport.y1 {
            y = (at.y - height).max(viewport.y0);
        }
    }
    Rect::from_origin_size(Point::new(x, y), Size::new(width, height))
}

/// What a menu entry does when it is clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Flip the node's collapsed flag.
    Toggle { key: String },
    /// Make the node the selected element.
    Select { key: String },
    /// Forget the selection.
    ClearSelection,
}

/// One entry of the context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub label: &'static str,
    pub action: MenuAction,
}

impl MenuItem {
    pub fn toggle(key: &str, expanded: bool) -> Self {
        Self {
            label: if expanded { "Collapse" } else { "Expand" },
            action: MenuAction::Toggle {
                key: key.to_string(),
            },
        }
    }

    pub fn select(key: &str) -> Self {
        Self {
            label: "Select element",
            action: MenuAction::Select {
                key: key.to_string(),
            },
        }
    }

    pub fn clear() -> Self {
        Self {
            label: "Clear selection",
            action: MenuAction::ClearSelection,
        }
    }
}

/// The menu a row opens on a right click.
///
/// Self-drawn and self-resolving: it reads the press position to pick an entry,
/// writes the shared state, and closes its own overlay, all inside one `event`.
pub struct ContextMenu {
    state: Arc<Mutex<DevtoolsState>>,
    items: Vec<MenuItem>,
    handle: OverlayHandle,
}

impl ContextMenu {
    pub fn new(
        state: Arc<Mutex<DevtoolsState>>,
        items: Vec<MenuItem>,
        handle: OverlayHandle,
    ) -> Self {
        Self {
            state,
            items,
            handle,
        }
    }

    /// The entries, in order — what the menu is about to offer.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// The entry under `at`, if the press landed on one.
    fn item_at(&self, bounds: Rect, at: Point) -> Option<usize> {
        if at.y < bounds.y0 + MENU_PADDING || at.y >= bounds.y1 - MENU_PADDING {
            return None;
        }
        let offset = at.y - bounds.y0 - MENU_PADDING;
        let index = (offset / MENU_ITEM_HEIGHT).floor();
        if index < 0.0 {
            return None;
        }
        let index = index as usize;
        (index < self.items.len()).then_some(index)
    }

    fn apply(&self, action: &MenuAction) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        match action {
            MenuAction::Toggle { key } => state.toggle_collapsed(key),
            MenuAction::Select { key } => state.select(key.clone()),
            MenuAction::ClearSelection => state.clear_selection(),
        }
    }
}

impl Widget for ContextMenu {
    fn create(&self, cx: &mut dyn floom_core::CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, _cx: &mut dyn UpdateContext, _id: ElementId) {}

    fn layout_request(&self) -> Option<LayoutRequest> {
        Some(
            LayoutRequest::new()
                .width(MENU_WIDTH as f32)
                .height((MENU_PADDING * 2.0 + MENU_ITEM_HEIGHT * self.items.len() as f64) as f32),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        cx.fill_rounded_rect(bounds, 4.0, Color::rgb(0x2a, 0x2e, 0x38));
        let mut y = bounds.y0 + MENU_PADDING;
        for item in &self.items {
            let row = Rect {
                x0: bounds.x0 + MENU_PADDING,
                y0: y,
                x1: bounds.x1 - MENU_PADDING,
                y1: y + MENU_ITEM_HEIGHT,
            };
            cx.fill_text(
                Point::new(row.x0 + 8.0, text_top(row, ROW_FONT)),
                item.label,
                ROW_FONT,
                FontWeight::Normal,
                TEXT,
            );
            y += MENU_ITEM_HEIGHT;
        }
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        match pointer.phase {
            PointerPhase::Down => {
                let Some(at) = cx.cursor_position() else {
                    return false;
                };
                if let Some(index) = self.item_at(cx.bounds(), at) {
                    self.apply(&self.items[index].action);
                }
                // Closes either way: a press in the menu's own padding is a press
                // on the menu, and a menu that stayed up for it would look stuck.
                cx.close_overlay(self.handle);
                true
            }
            PointerPhase::Up => true,
            _ => false,
        }
    }

    fn cursor(&self) -> floom_types::CursorStyle {
        floom_types::CursorStyle::Pointer
    }
}

impl WidgetAny for ContextMenu {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
