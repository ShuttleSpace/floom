//! The shell: a rail of every component on the left, the selected component's
//! page on the right.
//!
//! The shell owns the selection. The rail's entries set a pending selection,
//! the shell takes it in `update` and marks its children dirty, and the keyed
//! child is rebuilt for the newly selected page -- the same pattern the
//! showcase's router uses, for the same reason: a widget tree is rebuilt
//! rather than kept, so the state that has to survive a page change lives
//! here rather than in the page.

use std::sync::{Arc, Mutex};

use floom::floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, SemanticsHint,
    UpdateContext, Widget, WidgetAny,
};
use floom::floom_types::{Color, Edges, FontWeight, Insets};
use floom::floom_widgets::{Button, Scroll, Stack, Surface};
use floom_signal::{SignalGet, SignalUpdate, SyncBaseSignal};

use crate::pages::{Category, ComponentPage};
use crate::widgets::{self, ACCENT, BORDER, FAINT, INK, MUTED, SIDEBAR};

/// The size of the gallery window.
pub const WINDOW: floom::floom_types::Size = floom::floom_types::Size::new(1280.0, 860.0);

/// The gallery shell: the rail and the detail pane.
pub struct GalleryShell {
    pages: Arc<Vec<ComponentPage>>,
    selected: usize,
    pending: Arc<Mutex<Option<usize>>>,
    dark: Arc<SyncBaseSignal<bool>>,
    /// The dark state the shell last saw, so it can tell when it changed.
    dark_seen: bool,
}

impl GalleryShell {
    /// The whole catalogue, with the first page selected.
    pub fn new() -> Self {
        Self::showing(crate::registry::pages(), 0)
    }

    /// A shell over a given set of pages, with one of them selected.
    pub fn showing(pages: Vec<ComponentPage>, selected: usize) -> Self {
        let selected = selected.min(pages.len().saturating_sub(1));
        Self {
            pages: Arc::new(pages),
            selected,
            pending: Arc::new(Mutex::new(None)),
            dark: Arc::new(SyncBaseSignal::new(false)),
            dark_seen: false,
        }
    }
}

impl Default for GalleryShell {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for GalleryShell {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let mut changed = false;

        // A pending selection only matters when it differs from the one we hold
        // and stays within the catalogue; both conditions guard the update.
        if let Some(index) = self.pending.lock().unwrap().take()
            && index != self.selected
            && index < self.pages.len()
        {
            self.selected = index;
            changed = true;
        }

        // Dark mode is a global setting, but the widgets bake their resolved
        // colours when they are built -- so a change has to rebuild the tree
        // for the new colours to reach it.
        let dark = self.dark.get();
        if dark != self.dark_seen {
            self.dark_seen = dark;
            changed = true;
        }

        if changed {
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let pages = self.pages.clone();
        let selected = self.selected;
        let pending = self.pending.clone();
        let dark = self.dark.clone();

        // The key carries the selection, so the subtree is rebuilt when the
        // selection changes and survives untouched while it does not.
        vec![Child::new(
            ChildKey::keyed(selected as u64),
            LayoutRequest::new().width_pct(100.0).height_pct(100.0),
            move || {
                Box::new(build_shell(
                    pages.clone(),
                    selected,
                    pending.clone(),
                    dark.clone(),
                ))
            },
        )]
    }
}

impl WidgetAny for GalleryShell {
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

/// The whole window: a header, then the rail and the detail pane.
fn build_shell(
    pages: Arc<Vec<ComponentPage>>,
    selected: usize,
    pending: Arc<Mutex<Option<usize>>>,
    dark: Arc<SyncBaseSignal<bool>>,
) -> Surface {
    Surface::new()
        .background(Color::WHITE)
        .request(LayoutRequest::new().width_pct(100.0).height_pct(100.0))
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width_pct(100.0).height_pct(100.0),
            move || {
                Box::new(
                    Stack::new(Axis::Vertical)
                        .child(header(pages.len(), dark.clone()))
                        .child(body_row(pages.clone(), selected, pending.clone())),
                )
            },
        ))
}

/// The header: what this is, and how much of it there is.
fn header(count: usize, dark: Arc<SyncBaseSignal<bool>>) -> Child {
    Child::new(
        ChildKey::from("header"),
        LayoutRequest::new().width_pct(100.0).height(58.0),
        move || {
            let brand = Stack::new(Axis::Horizontal)
                .gap(7.0)
                .align(Align::Start, Align::Center)
                .child(widgets::label(
                    "brand",
                    "Floom",
                    18.0,
                    FontWeight::Bold,
                    ACCENT,
                ))
                .child(widgets::label(
                    "product",
                    "Component gallery",
                    15.0,
                    FontWeight::Normal,
                    FAINT,
                ));

            let toggle_dark = dark.clone();
            let right = Stack::new(Axis::Horizontal)
                .gap(14.0)
                .align(Align::Start, Align::Center)
                .child(widgets::label(
                    "count",
                    format!("{count} components"),
                    12.0,
                    FontWeight::Normal,
                    MUTED,
                ))
                .child(Child::new(
                    ChildKey::from("dark-toggle"),
                    LayoutRequest::new().width(96.0).height(30.0),
                    move || {
                        let toggle_dark = toggle_dark.clone();
                        Box::new(Button::new("Dark").radius(6.0).on_click(move || {
                            let next = !toggle_dark.get();
                            toggle_dark.set(next);
                            floom_core_theme::global_theme::set_dark(next);
                        }))
                    },
                ));

            let brand = Arc::new(brand);
            let right = Arc::new(right);
            let brand_slot = brand.clone();
            let right_slot = right.clone();

            Box::new(
                Surface::new()
                    .background(Color::WHITE)
                    .border_edges(1.0, BORDER, Edges::BOTTOM)
                    .padding(Insets {
                        top: 0.0,
                        right: 22.0,
                        bottom: 0.0,
                        left: 22.0,
                    })
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || {
                            Box::new(
                                Stack::new(Axis::Horizontal)
                                    .align(Align::SpaceBetween, Align::Center)
                                    .child(Child::new(
                                        ChildKey::from("brand-slot"),
                                        LayoutRequest::new(),
                                        {
                                            let brand = brand_slot.clone();
                                            move || Box::new((*brand).clone())
                                        },
                                    ))
                                    .child(Child::new(
                                        ChildKey::from("right-slot"),
                                        LayoutRequest::new(),
                                        {
                                            let right = right_slot.clone();
                                            move || Box::new((*right).clone())
                                        },
                                    )),
                            )
                        },
                    )),
            )
        },
    )
}

/// The rail and the detail pane, side by side.
fn body_row(
    pages: Arc<Vec<ComponentPage>>,
    selected: usize,
    pending: Arc<Mutex<Option<usize>>>,
) -> Child {
    Child::new(
        ChildKey::from("body"),
        LayoutRequest::new().width_pct(100.0).fill(),
        move || {
            Box::new(
                Stack::new(Axis::Horizontal)
                    .child(rail(pages.clone(), selected, pending.clone()))
                    .child(detail(pages.clone(), selected)),
            )
        },
    )
}

/// The rail: every page, grouped by category.
fn rail(
    pages: Arc<Vec<ComponentPage>>,
    selected: usize,
    pending: Arc<Mutex<Option<usize>>>,
) -> Child {
    Child::new(
        ChildKey::from("rail"),
        LayoutRequest::new()
            .width(240.0)
            .min_width(240.0)
            .height_pct(100.0),
        move || {
            let mut column = Stack::new(Axis::Vertical).gap(2.0);
            let mut category = None;

            for (index, page) in pages.iter().enumerate() {
                if category != Some(page.category) {
                    category = Some(page.category);
                    column = column.child(widgets::section_label(
                        page.category.slug(),
                        page.category.label(),
                    ));
                }
                let entry_pending = pending.clone();
                column = column.child(widgets::nav_item(
                    page.component,
                    page.name,
                    index == selected,
                    move || {
                        *entry_pending.lock().unwrap() = Some(index);
                    },
                ));
            }
            column = column.child(widgets::spacer("rail-fill", 8.0));

            let column = Arc::new(column);
            let column_slot = column.clone();

            Box::new(
                Surface::new()
                    .background(SIDEBAR)
                    .border_edges(1.0, BORDER, Edges::RIGHT)
                    .padding(Insets {
                        top: 16.0,
                        right: 12.0,
                        bottom: 16.0,
                        left: 12.0,
                    })
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || {
                            Box::new(Scroll::new().child(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().width_pct(100.0),
                                {
                                    let column = column_slot.clone();
                                    move || Box::new((*column).clone())
                                },
                            )))
                        },
                    )),
            )
        },
    )
}

/// The detail pane: the selected page.
fn detail(pages: Arc<Vec<ComponentPage>>, selected: usize) -> Child {
    Child::new(
        ChildKey::from("detail"),
        LayoutRequest::new().fill().min_width(0.0).height_pct(100.0),
        move || {
            let page = pages
                .get(selected)
                .cloned()
                .unwrap_or_else(|| pages[0].clone());

            Box::new(
                Surface::new()
                    .background(Color::WHITE)
                    .request(LayoutRequest::new().min_width(0.0))
                    .padding(Insets {
                        top: 24.0,
                        right: 28.0,
                        bottom: 24.0,
                        left: 28.0,
                    })
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || {
                            Box::new(Scroll::new().child(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().width_pct(100.0),
                                {
                                    let page = page.clone();
                                    move || Box::new(page_body(&page))
                                },
                            )))
                        },
                    )),
            )
        },
    )
}

/// One page: its title, its introduction, its samples and its API tables.
fn page_body(page: &ComponentPage) -> Stack {
    let mut column = Stack::new(Axis::Vertical).gap(16.0);

    column = column.child(widgets::label(
        "page-title",
        page.name,
        22.0,
        FontWeight::Bold,
        INK,
    ));
    if !page.intro.is_empty() {
        column = column.child(widgets::paragraph("page-intro", page.intro, 13.0, MUTED));
    }

    for (index, sample) in page.samples.iter().enumerate() {
        column = column.child(widgets::section_label(sample.title, sample.title));
        if let Some(note) = sample.note {
            column = column.child(widgets::paragraph("sample-note", note, 12.0, MUTED));
        }
        let build = sample.build;
        let key = format!("sample-{index}");
        column = column.child(Child::new(
            ChildKey::from(key.as_str()),
            LayoutRequest::new().width_pct(100.0),
            move || (build)(),
        ));
    }

    if !page.api.props.is_empty() {
        column = column.child(Child::new(
            ChildKey::from("props-table"),
            LayoutRequest::new().width_pct(100.0),
            {
                let rows = page.api.props.clone();
                move || Box::new(widgets::table("props", "Props", &rows))
            },
        ));
    }
    if !page.api.events.is_empty() {
        column = column.child(Child::new(
            ChildKey::from("events-table"),
            LayoutRequest::new().width_pct(100.0),
            {
                let rows = page.api.events.clone();
                move || Box::new(widgets::table("events", "Events", &rows))
            },
        ));
    }
    if !page.api.methods.is_empty() {
        column = column.child(Child::new(
            ChildKey::from("methods-table"),
            LayoutRequest::new().width_pct(100.0),
            {
                let rows = page.api.methods.clone();
                move || Box::new(widgets::table("methods", "Methods", &rows))
            },
        ));
    }

    column
}

/// The categories the gallery's pages fall into, in rail order.
pub fn categories() -> Vec<Category> {
    Category::ALL.to_vec()
}
