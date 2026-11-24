use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{
    Child, ChildKey, Color, CreateContext, ElementId, Inset, LayoutRequest, PaintContext,
    UpdateContext, Widget,
};
use floom_types::ClipShape;
use floom_widgets::primitives::Revision;
use floom_widgets::{Align, Axis, Container, Pressable, Stack, Surface, Text};

/// The key the page on show is mounted under. The index is part of it, so a
/// new page is a new slot and the old one is unmounted.
const PAGE_KEY: u64 = 0x5700_0000;
/// The key the previous arrow is mounted under.
const PREVIOUS_ARROW_KEY: u64 = 0x5700_1000;
/// The key the next arrow is mounted under.
const NEXT_ARROW_KEY: u64 = 0x5700_2000;
/// The key the dots are mounted under. The index is part of it, so the dots
/// are rebuilt -- and so the active one changes colour -- when the page does.
const DOTS_KEY: u64 = 0x5700_3000;

/// The viewer's appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowViewerStyle {
    /// The box's fill.
    pub surface: Color,
    /// The arrow glyphs' ink.
    pub on_surface: Color,
    /// Reported by v1's style, but the paint never used it.
    pub on_surface_variant: Color,
    /// The active dot's fill.
    pub primary: Color,
    /// The inactive dots' fill.
    pub dot_inactive_color: Color,
    /// The box's corner radius.
    pub border_radius: f32,
    /// The box's height.
    pub height: f32,
    /// The dots' diameter.
    pub dot_size: f32,
    /// The token's arrow size.
    pub arrow_size: f32,
    /// The arrow glyph's size: 0.6 of the token's arrow size.
    pub arrow_font_size: f32,
    /// The width of the box the arrow glyph sits in.
    pub arrow_width: f32,
    /// The gap between the dots.
    pub dot_gap: f32,
    /// The outline's thickness.
    pub border_width: f32,
    /// The outline's colour.
    pub border_color: Color,
    /// How far the dots sit above the bottom edge, in per cent of the height.
    pub dots_inset_pct: f32,
}

impl Default for WindowViewerStyle {
    fn default() -> Self {
        Self {
            surface: Color::rgb(0xFF, 0xFF, 0xFF),
            on_surface: Color::rgb(0x1C, 0x1B, 0x1F),
            on_surface_variant: Color::rgb(0x49, 0x45, 0x55),
            primary: Color::rgb(0x67, 0x50, 0xA4),
            dot_inactive_color: Color::rgb(0xE7, 0xE0, 0xEC),
            border_radius: 0.0,
            height: 200.0,
            dot_size: 6.0,
            arrow_size: 24.0,
            arrow_font_size: 14.4,
            arrow_width: 40.0,
            dot_gap: 6.0,
            border_width: 1.0,
            border_color: Color::rgb(0xE7, 0xE0, 0xEC),
            dots_inset_pct: 2.0,
        }
    }
}

/// The viewer: the box, the page on show, the arrows and the dots.
///
/// The shape v1's `Stack::new(children)` had, in a widget that can say which
/// page is on show: 2.0 mounts a tree once, so a widget whose children
/// depend on a value that changes has to say so, and re-declare itself when
/// it does.
pub struct WindowViewer {
    /// The box: its fill, its radius and its outline.
    decoration: Container,
    /// The pages, as 2.0 slots.
    pages: Vec<Child>,
    /// The page on show. Shared with the arrows and the dots, which move it.
    index: Arc<AtomicUsize>,
    /// The page the children were last declared for.
    declared: usize,
    show_arrows: bool,
    show_dots: bool,
    continuous: bool,
    on_change: Option<Arc<dyn Fn(usize) + Send + Sync + 'static>>,
    style: WindowViewerStyle,
    revision: Revision,
}

impl WindowViewer {
    pub fn new() -> Self {
        Self {
            decoration: Container::new(),
            pages: Vec::new(),
            index: Arc::new(AtomicUsize::new(0)),
            declared: usize::MAX,
            show_arrows: false,
            show_dots: false,
            continuous: false,
            on_change: None,
            style: WindowViewerStyle::default(),
            revision: Revision::default(),
        }
    }

    /// The viewer's appearance.
    pub fn style(mut self, style: WindowViewerStyle) -> Self {
        // The box's own decoration follows the style rather than being set a
        // second time.
        self.decoration = Container::new()
            .background(style.surface)
            .radius(style.border_radius)
            .border(style.border_width, style.border_color);
        self.style = style;
        self
    }

    /// The pages, as 2.0 slots.
    pub fn pages(mut self, pages: Vec<Child>) -> Self {
        self.pages = pages;
        self
    }

    /// The counter the page on show lives in.
    pub fn shared_index(mut self, index: Arc<AtomicUsize>) -> Self {
        self.index = index;
        self
    }

    /// Whether the arrows that move the page are shown.
    pub fn show_arrows(mut self, show: bool) -> Self {
        self.show_arrows = show;
        self
    }

    /// Whether the dots that move the page are shown.
    pub fn show_dots(mut self, show: bool) -> Self {
        self.show_dots = show;
        self
    }

    /// Whether the arrows wrap around at the ends.
    pub fn continuous(mut self, continuous: bool) -> Self {
        self.continuous = continuous;
        self
    }

    /// Called with the page's index whenever the page moves.
    pub fn on_change(mut self, callback: impl Fn(usize) + Send + Sync + 'static) -> Self {
        self.on_change = Some(Arc::new(callback));
        self
    }
}

impl WindowViewer {
    /// The page on show.
    pub fn index(&self) -> usize {
        self.current()
    }

    /// The page on show, clamped to the pages there are.
    fn current(&self) -> usize {
        let count = self.pages.len();
        if count == 0 {
            return 0;
        }
        self.index.load(Ordering::SeqCst).min(count - 1)
    }

    /// The arrow at one edge of the box: a glyph, and the click that moves the
    /// page on.
    fn arrow(&self, next: bool) -> Child {
        let style = self.style;
        let shared = self.index.clone();
        let count = self.pages.len();
        let continuous = self.continuous;
        let on_change = self.on_change.clone();
        let glyph = if next { "›" } else { "‹" };
        let ink = style.on_surface;
        let font_size = style.arrow_font_size;

        let mut request = LayoutRequest::new()
            .width(style.arrow_width)
            .height_pct(100.0)
            // v1's `.items_center()` on the arrow's box.
            .main_align(Align::Center)
            .cross_align(Align::Center)
            .absolute_fill();
        request.inset = Some(if next {
            Inset {
                left: None,
                top: Some(0.0),
                right: Some(0.0),
                bottom: None,
            }
        } else {
            Inset {
                left: Some(0.0),
                top: Some(0.0),
                right: None,
                bottom: None,
            }
        });

        Child::new(
            ChildKey::keyed(if next {
                NEXT_ARROW_KEY
            } else {
                PREVIOUS_ARROW_KEY
            }),
            request,
            move || {
                // The slot is a replayed `Fn`, so the handles it hands to the
                // click are copies of its own.
                let shared = shared.clone();
                let on_change = on_change.clone();
                Box::new(
                    Pressable::new(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new(),
                        move || Box::new(Text::new(glyph).size(font_size).color(ink)),
                    ))
                    .on_click(move || {
                        let current = shared.load(Ordering::SeqCst);
                        let target = if next {
                            if current + 1 >= count {
                                if continuous {
                                    0
                                } else {
                                    count - 1
                                }
                            } else {
                                current + 1
                            }
                        } else if current == 0 {
                            if continuous {
                                count - 1
                            } else {
                                0
                            }
                        } else {
                            current - 1
                        };
                        shared.store(target, Ordering::SeqCst);
                        if let Some(cb) = &on_change {
                            cb(target);
                        }
                    }),
                )
            },
        )
    }

    /// The dots along the bottom of the box: one per page, the active one
    /// filled with the primary.
    fn dots(&self, index: usize) -> Child {
        let style = self.style;
        let count = self.pages.len();
        let mut row = Stack::new(Axis::Horizontal)
            // v1's `.justify_center().items_center()` on the row.
            .align(Align::Center, Align::Center);

        for dot in 0..count {
            let active = dot == index;
            let fill = if active {
                style.primary
            } else {
                style.dot_inactive_color
            };
            let size = style.dot_size;
            let radius = style.dot_size / 2.0;
            let shared = self.index.clone();
            let on_change = self.on_change.clone();
            row = row.push_layout(
                // v1's `.margin(dot_gap / 2.0)` on each dot.
                LayoutRequest::new().margin_horiz(f64::from(style.dot_gap / 2.0)),
                move || {
                    // The slot is a replayed `Fn`, so the handles it hands to
                    // the click are copies of its own.
                    let shared = shared.clone();
                    let on_change = on_change.clone();
                    Box::new(
                        Pressable::new(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().width(size).height(size),
                            move || Box::new(Surface::new().background(fill).radius(radius)),
                        ))
                        .on_click(move || {
                            shared.store(dot, Ordering::SeqCst);
                            if let Some(cb) = &on_change {
                                cb(dot);
                            }
                        }),
                    )
                },
            );
        }

        let mut request = LayoutRequest::new()
            .height(style.dot_size * 2.0)
            // v1's `.items_center().justify_center()` on the dots' box.
            .main_align(Align::Center)
            .cross_align(Align::Center)
            .absolute_fill();
        request.inset = Some(Inset {
            left: Some(0.0),
            top: None,
            right: Some(0.0),
            // v1's `inset_bottom_pct(2.0)`: a percentage of the box's height.
            bottom: Some(self.style.height * style.dots_inset_pct / 100.0),
        });

        Child::new(
            // The index is part of the key, so the dots are rebuilt -- and the
            // active one changes colour -- when the page changes.
            ChildKey::keyed(DOTS_KEY + index as u64),
            request,
            move || Box::new(row.clone()),
        )
    }
}

impl Widget for WindowViewer {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    /// The page moved: the children have to be declared again, because the
    /// page on show is a different slot and the dots are a different colour.
    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.revision.sync(cx, id);
        let current = self.current();
        if current != self.declared {
            self.declared = current;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let index = self.current();
        let mut out = Vec::new();

        // The page on show, and only the page on show: 2.0 has no `hide()`.
        if let Some(page) = self.pages.get(index) {
            out.push(Child {
                // The index is part of the key, so the page that was on show
                // is unmounted and the new one mounted.
                key: ChildKey::keyed(PAGE_KEY + index as u64),
                // v1's page was absolute, at the box's top left, and as big as
                // the box.
                layout: LayoutRequest::new().absolute_fill(),
                build: page.build.clone(),
                shared_key: page.shared_key,
                name: page.name,
            });
        }

        if self.show_arrows && self.pages.len() > 1 {
            out.push(self.arrow(false));
            out.push(self.arrow(true));
        }

        if self.show_dots && !self.pages.is_empty() {
            out.push(self.dots(index));
        }

        out
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        // v1's box: the full width of its parent, and the height.
        Some(
            LayoutRequest::new()
                .width_pct(100.0)
                .height(self.style.height),
        )
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.decoration.paint(cx, id);
    }

    /// v1 clipped the box's contents to the box.
    fn clip(&self) -> Option<ClipShape> {
        Some(ClipShape::RoundedBox {
            radius: f64::from(self.style.border_radius),
        })
    }
}

impl floom_core::WidgetAny for WindowViewer {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    /// The viewer publishes nothing of its own: the page inside it, and the
    /// arrows and the dots, are what speak.
    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}
