//! `Menu` — a list of choices in a floating panel.
//!
//! Theme-agnostic by design, like [`Badge`]: a 2.0 widget takes explicit values
//! rather than reading a global theme signal, so resolving the palette and the
//! token sizes is the *kit's* job.
//!
//! ## What a menu is
//!
//! A panel with a row per choice, a fill behind the row the pointer is over,
//! and a click that picks the row and closes the panel. That is the whole
//! thing, and all three of the floem-ui kits' menus are it.
//!
//! ## Why it is an overlay, and where it goes
//!
//! A menu is a panel *over* the page: it must not push the content around, and
//! it must escape the box it is declared in. So it is asked of the host as an
//! overlay layer, the way [`Popup`](crate::popup::Popup) is, and it is placed
//! at the top-left of the box it is mounted in.
//!
//! v1 had no such decision: the panel went into an overlay at the overlay's
//! own origin, and the *caller* decided where that was by choosing the slot the
//! menu was declared in. The 2.0 menu keeps that -- the caller still chooses
//! the slot -- and the host does the arithmetic.
//!
//! ## The open flag
//!
//! A menu is opened by something else -- a button, a key -- so its state is
//! shared rather than owned: a [`SharedFlag`] the caller writes and the menu
//! reads. Choosing a row closes the menu by writing the same flag.

use std::sync::Arc;

use floom_accessibility::SemanticsRole;
use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Event, EventContext, FontWeight, LayoutRequest,
    OverlayKind, OverlayRequest, PaintContext, Point, Rect, SemanticsContext, UpdateContext, Widget,
    WidgetAny,
};
use floom_types::{Insets, PointerButton, PointerPhase, Shadow};

use floom_widgets::primitives::{Revision, SemanticsHint, emit_semantics};
use floom_widgets::{SharedFlag, read_flag, shared_flag, write_flag};

use crate::overlay_bookkeeping::OverlayHost;

/// One row of a menu.
///
/// The callback is a `Box`, the same shape the shared props carry, so a kit
/// hands its row straight over without a conversion. The rows are held behind
/// an `Arc` by the widget that owns them, because a row has to be cloneable:
/// the panel is built twice over, once for the host's layer and once for the
/// in-tree fallback.
pub struct MenuItem {
    /// The row's label.
    pub label: String,
    /// Called when the row is chosen.
    pub on_select: Option<Box<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for MenuItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuItem")
            .field("label", &self.label)
            .field(
                "on_select",
                &self.on_select.as_ref().map(|_| "Fn()"),
            )
            .finish()
    }
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            on_select: None,
        }
    }

    /// Called when the row is chosen.
    pub fn on_select(mut self, callback: impl Fn() + Send + Sync + 'static) -> Self {
        self.on_select = Some(Box::new(callback));
        self
    }
}

/// The menu's appearance.
///
/// Every field here is read by [`Menu`], and the coverage is what the three
/// kits' disagreement needs rather than what one kit's markup happened to
/// spell out. The colours are *resolved* rather than carried as an alpha over
/// a theme colour: the engine has no theme, and the one place the kits
/// genuinely disagree -- a hovered row is `on_surface`@8 in Material and
/// Fluent but `primary`@16 in Cupertino -- is a different *source* colour, not
/// a different alpha.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStyle {
    /// The panel's fill.
    pub background: Color,
    /// The rows' ink.
    pub text_color: Color,
    /// The fill behind the row the pointer is over.
    pub hover_color: Color,
    /// The panel's minimum width.
    pub min_width: f64,
    /// The panel's maximum width.
    pub max_width: f64,
    /// One row's height.
    pub item_height: f64,
    /// One row's padding at the sides.
    pub item_padding_horizontal: f64,
    /// One row's padding above and below its text.
    pub item_padding_vertical: f64,
    /// The rows' text size.
    pub item_font_size: f32,
    /// The panel's padding around the rows.
    pub padding: f64,
    /// The panel's corner radius.
    pub radius: f32,
    /// The panel's shadow.
    pub shadow: Shadow,
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self {
            background: Color::rgb(0xFF, 0xFB, 0xFE),
            text_color: Color::rgb(0x1C, 0x1B, 0x1F),
            hover_color: Color::rgba(0x1C, 0x1B, 0x1F, 8),
            min_width: 112.0,
            max_width: 280.0,
            item_height: 48.0,
            item_padding_horizontal: 12.0,
            item_padding_vertical: 8.0,
            item_font_size: 14.0,
            padding: 8.0,
            radius: 4.0,
            shadow: Shadow {
                color: Color::rgba(0, 0, 0, 24),
                blur: 12.0,
                offset_x: 0.0,
                offset_y: 0.0,
                spread: 1.0,
            },
        }
    }
}

/// A list of choices in a floating panel.
///
/// The panel is up while the shared flag says so, and choosing a row writes
/// the flag back to `false`.
pub struct Menu {
    style: MenuStyle,
    items: Vec<Arc<MenuItem>>,
    /// Whether the menu is up. Shared with whoever opens it.
    open: SharedFlag,
    surface: OverlayHost,
    revision: Revision,
}

impl std::fmt::Debug for Menu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Menu")
            .field("items", &self.items.len())
            .field("open", &read_flag(&self.open))
            .finish_non_exhaustive()
    }
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    pub fn new() -> Self {
        Self {
            style: MenuStyle::default(),
            items: Vec::new(),
            open: shared_flag(false),
            surface: OverlayHost::new(false),
            revision: Revision::default(),
        }
    }

    /// The menu's appearance.
    pub fn style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Append a row.
    pub fn item(mut self, item: MenuItem) -> Self {
        self.items.push(Arc::new(item));
        self
    }

    /// The flag that says whether the menu is up.
    ///
    /// The caller writes it; the menu reads it, and writes it back to `false`
    /// when a row is chosen.
    pub fn open_flag(&self) -> SharedFlag {
        self.open.clone()
    }

    /// Use a flag the caller already has.
    pub fn with_open_flag(mut self, open: SharedFlag) -> Self {
        self.open = open;
        self
    }

    /// Whether the menu is up.
    pub fn is_open(&self) -> bool {
        read_flag(&self.open)
    }

    /// The panel's size: the widest row, and the rows stacked.
    fn panel_size(&self) -> (f64, f64) {
        let style = self.style;
        let widest = self
            .items
            .iter()
            .map(|item| {
                let measured = floom_text::measure_text(
                    &item.label,
                    style.item_font_size,
                    f64::INFINITY,
                );
                measured.width as f64 + style.item_padding_horizontal * 2.0
            })
            .fold(0.0_f64, f64::max);
        let width = widest.clamp(style.min_width, style.max_width);
        let height = self.items.len() as f64 * style.item_height + style.padding * 2.0;
        (width, height)
    }
}

/// The panel: the fill, the rows, and the row the pointer is over.
///
/// A separate widget from [`Menu`] because the panel is what the host lays
/// out, and because the panel is what knows which row the pointer is over.
#[derive(Clone)]
struct MenuPanel {
    style: MenuStyle,
    items: Vec<Arc<MenuItem>>,
    open: SharedFlag,
    /// The row the pointer is over.
    hovered: Option<usize>,
    revision: Revision,
}

impl std::fmt::Debug for MenuPanel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuPanel")
            .field("items", &self.items.len())
            .field("hovered", &self.hovered)
            .finish_non_exhaustive()
    }
}

impl MenuPanel {
    /// The row at a point, if any.
    fn row_at(&self, bounds: Rect, point: Point) -> Option<usize> {
        let style = self.style;
        let top = bounds.y0 + style.padding;
        if point.y < top || point.y >= top + self.items.len() as f64 * style.item_height {
            return None;
        }
        let index = ((point.y - top) / style.item_height).floor() as usize;
        if index < self.items.len() {
            Some(index)
        } else {
            None
        }
    }
}

impl WidgetAny for MenuPanel {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // The inherent method wins the name, so this is that one, converted
        // into the engine's own hint.
        Some(self.semantics_hint().into())
    }
}

impl Widget for MenuPanel {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
    }

    fn event(&mut self, cx: &mut dyn EventContext, _id: ElementId) -> bool {
        let Event::Pointer(pointer) = cx.event() else {
            return false;
        };
        let bounds = cx.bounds();
        match pointer.phase {
            PointerPhase::Enter | PointerPhase::Move => {
                let over = self.row_at(bounds, pointer.position);
                if over != self.hovered {
                    self.hovered = over;
                    self.revision.bump();
                }
                true
            }
            PointerPhase::Leave | PointerPhase::Cancel => {
                if self.hovered.is_some() {
                    self.hovered = None;
                    self.revision.bump();
                }
                true
            }
            PointerPhase::Up if pointer.button == Some(PointerButton::Primary) => {
                let Some(index) = self.row_at(bounds, pointer.position) else {
                    return false;
                };
                if let Some(item) = self.items.get(index) {
                    if let Some(callback) = &item.on_select {
                        callback();
                    }
                }
                // Choosing a row closes the menu: the same flag the caller
                // opened it with.
                write_flag(&self.open, false);
                self.hovered = None;
                self.revision.bump();
                true
            }
            _ => false,
        }
    }

    /// The shadow, then the panel, then the rows.
    ///
    /// The shadow goes first because it belongs *behind* the panel, and the
    /// rows go last because they are the only thing on top of it.
    fn paint(&self, cx: &mut dyn PaintContext, _id: ElementId) {
        let bounds = cx.bounds();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let style = self.style;
        cx.draw_shadow(bounds, style.radius, &style.shadow);
        cx.fill_rounded_rect(bounds, style.radius, style.background);

        let top = bounds.y0 + style.padding;
        for (index, item) in self.items.iter().enumerate() {
            let row = Rect {
                x0: bounds.x0,
                y0: top + index as f64 * style.item_height,
                x1: bounds.x1,
                y1: top + (index + 1) as f64 * style.item_height,
            };
            if row.y0 >= bounds.y1 {
                break;
            }
            if self.hovered == Some(index) && style.hover_color.a > 0 {
                cx.fill_rect(row, style.hover_color);
            }
            // v1 put the row's text `padding_vert` below the row's top edge,
            // rather than in the middle of the row.
            cx.fill_text(
                Point::new(
                    row.x0 + style.item_padding_horizontal,
                    row.y0 + style.item_padding_vertical,
                ),
                &item.label,
                style.item_font_size,
                FontWeight::Normal,
                style.text_color,
            );
        }
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl MenuPanel {
    /// What the panel reports to assistive technology: a list of its rows.
    fn semantics_hint(&self) -> SemanticsHint {
        if self.items.is_empty() {
            return SemanticsHint::decorative();
        }
        SemanticsHint::labeled(SemanticsRole::List, self.items[0].label.clone())
    }
}

impl WidgetAny for Menu {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        // The inherent method wins the name, so this is that one, converted
        // into the engine's own hint.
        Some(self.semantics_hint().into())
    }
}

impl Widget for Menu {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
        // The menu is up exactly while the flag says so.
        let present = read_flag(&self.open);
        self.surface.set_present(present);
        let style = self.style;
        let items = self.items.clone();
        let open = self.open.clone();
        // The panel is placed at the top-left of the box the menu is mounted
        // in, and it is the menu's own size: the host lays the content out
        // against the rect it is given.
        self.surface.reconcile(cx, id, move |bounds, _viewport| {
            let width = items
                .iter()
                .map(|item| {
                    let measured = floom_text::measure_text(
                        &item.label,
                        style.item_font_size,
                        f64::INFINITY,
                    );
                    measured.width as f64 + style.item_padding_horizontal * 2.0
                })
                .fold(0.0_f64, f64::max)
                .clamp(style.min_width, style.max_width);
            let height = items.len() as f64 * style.item_height + style.padding * 2.0;
            let rect = Rect {
                x0: bounds.x0,
                y0: bounds.y0,
                x1: bounds.x0 + width,
                y1: bounds.y0 + height,
            };
            Some(OverlayRequest::new(
                OverlayKind::Popup,
                false,
                rect,
                move |_handle| {
                    Box::new(MenuPanel {
                        style,
                        items: items.clone(),
                        open: open.clone(),
                        hovered: None,
                        revision: Revision::default(),
                    })
                },
            ))
        });
    }

    fn children(&self) -> Vec<Child> {
        if !self.surface.takes_children_in_tree() {
            return Vec::new();
        }
        // No host: the panel is in the tree, at the top-left of this widget's
        // box.
        let style = self.style;
        let items = self.items.clone();
        let open = self.open.clone();
        let (width, height) = self.panel_size();
        vec![Child::new(
            ChildKey::index(0),
            LayoutRequest::new()
                .width(width as f32)
                .height(height as f32),
            move || {
                Box::new(MenuPanel {
                    style,
                    items: items.clone(),
                    open: open.clone(),
                    hovered: None,
                    revision: Revision::default(),
                })
            },
        )]
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // The panel is out of the flow either way: with a host it is in the
        // host's layer, and without one it is absolutely placed.
        Some(LayoutRequest::new().absolute_fill())
    }

    fn paint(&self, _cx: &mut dyn PaintContext, _id: ElementId) {}

    fn semantics(&self, cx: &mut dyn SemanticsContext, _id: ElementId) {
        emit_semantics(self.semantics_hint(), cx);
    }
}

impl Menu {
    /// What the menu reports to assistive technology.
    ///
    /// A menu that is not up is not there to be read, and a menu with no rows
    /// has nothing to announce.
    pub fn semantics_hint(&self) -> SemanticsHint {
        if self.items.is_empty() || !self.surface.is_shown() {
            return SemanticsHint::decorative();
        }
        SemanticsHint::labeled(SemanticsRole::List, self.items[0].label.clone())
    }
}

/// The insets a row's text sits inside, for a caller that draws its own rows.
pub fn menu_row_insets(style: &MenuStyle) -> Insets {
    Insets {
        left: style.item_padding_horizontal,
        right: style.item_padding_horizontal,
        top: style.item_padding_vertical,
        bottom: style.item_padding_vertical,
    }
}
