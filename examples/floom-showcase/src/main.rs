//! The Floom 2.0 showcase: one application over the whole stack.
//!
//! Run with `cargo run -p floom-showcase`.
//!
//! - `/components` mounts the engine's Tier-0 widgets directly.
//! - `/signals` drives state through `floom-signal`'s `SyncBaseSignal`.
//! - `/kits` mounts the ported `floom-ui` material components.
//! - `/editor` mounts `floom-editor`'s `EditorWidget`.
//! - `/devtools` explains the panel that F12 opens (the entry is
//!   `run_desktop_with_devtools`, so it is wired for the whole app).
//!
//! Navigation goes through `floom-navigation`'s `Router`. That crate is
//! widget-free, so the shell owns the router and re-declares its subtree when
//! `Router::current` changes.

use std::sync::{Arc, Mutex};

use floom::floom_core::{
    CreateContext, ElementId, EventContext, PaintContext, SemanticsContext, SemanticsHint,
    UpdateContext, Widget, WidgetAny,
};
use floom::floom_types::{Edges, FontWeight, Insets};
use floom::floom_widgets::{Scroll, Surface};
use floom::prelude::*;
use floom_core_theme::props::button::ButtonVariant;
use floom_editor::EditorWidget;
use floom_navigation::{Route, Router};
use floom_signal::{SignalGet, SignalUpdate, SyncBaseSignal};
use floom_widget_md_button::MdButton;
use floom_widget_md_checkbox::MdCheckbox;
use floom_widget_md_text::MaterialText;

const WINDOW: Size = Size::new(1180.0, 820.0);

const INK: Color = Color::rgb(0x1A, 0x1D, 0x23);
const MUTED: Color = Color::rgb(0x5C, 0x63, 0x70);
const FAINT: Color = Color::rgb(0x8A, 0x91, 0x9E);
const ACCENT: Color = Color::rgb(0x2B, 0x6C, 0xB0);
const SIDEBAR: Color = Color::rgb(0xF4, 0xF6, 0xF8);
const BORDER: Color = Color::rgb(0xE1, 0xE5, 0xEB);

const SAMPLE: &str = "\
Floom 2.0 — editor

This page mounts floom-editor's document model:
  - a rope-backed TextDocument
  - transactions whose inverse records undo
  - selection highlight and a caret drawn by the widget

Type here. Mod+Z undoes, Mod+Y redoes.
Backspace and Delete edit around the caret.
Clicking the editor focuses it; keys go to the focused element.";

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AppRoute {
    Home,
    Components,
    Signals,
    Kits,
    Editor,
    Devtools,
}

impl AppRoute {
    const ALL: [Self; 6] = [
        Self::Home,
        Self::Components,
        Self::Signals,
        Self::Kits,
        Self::Editor,
        Self::Devtools,
    ];

    fn path(self) -> &'static str {
        match self {
            Self::Home => "/",
            Self::Components => "/components",
            Self::Signals => "/signals",
            Self::Kits => "/kits",
            Self::Editor => "/editor",
            Self::Devtools => "/devtools",
        }
    }

    fn nav_label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Components => "Components",
            Self::Signals => "Signals",
            Self::Kits => "Kits",
            Self::Editor => "Editor",
            Self::Devtools => "Devtools",
        }
    }

    fn id(self) -> u64 {
        match self {
            Self::Home => 0,
            Self::Components => 1,
            Self::Signals => 2,
            Self::Kits => 3,
            Self::Editor => 4,
            Self::Devtools => 5,
        }
    }
}

impl Route for AppRoute {
    fn encode(&self) -> String {
        self.path().to_string()
    }

    fn decode(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|route| route.path() == path)
    }
}

#[derive(Clone, Copy)]
enum NavIntent {
    Go(AppRoute),
    Back,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// State that has to outlive a page rebuild.
///
/// `SyncBaseSignal` (not the `Rc`-based `RwSignal`) is the one that crosses
/// `Child::new`'s `Send + Sync` builder boundary; the editor is behind a
/// `Mutex` for the same reason.
struct AppState {
    clicks: SyncBaseSignal<i32>,
    bright: SyncBaseSignal<bool>,
    signal_count: SyncBaseSignal<i32>,
    signal_flag: SyncBaseSignal<bool>,
    kit_clicks: SyncBaseSignal<i32>,
    kit_checked: SyncBaseSignal<bool>,
    route: SyncBaseSignal<u8>,
    editor: Mutex<Arc<Mutex<EditorWidget>>>,
}

impl AppState {
    fn new() -> Self {
        Self {
            clicks: SyncBaseSignal::new(0),
            bright: SyncBaseSignal::new(true),
            signal_count: SyncBaseSignal::new(3),
            signal_flag: SyncBaseSignal::new(true),
            kit_clicks: SyncBaseSignal::new(0),
            kit_checked: SyncBaseSignal::new(true),
            route: SyncBaseSignal::new(AppRoute::Home.id() as u8),
            editor: Mutex::new(Arc::new(Mutex::new(EditorWidget::from_text(SAMPLE)))),
        }
    }

    fn editor(&self) -> Arc<Mutex<EditorWidget>> {
        self.editor.lock().unwrap().clone()
    }
}

// ---------------------------------------------------------------------------
// Shell
// ---------------------------------------------------------------------------

struct ShowcaseShell {
    router: Router<AppRoute>,
    rendered: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
    state: Arc<AppState>,
}

impl ShowcaseShell {
    fn new(state: Arc<AppState>) -> Self {
        Self::new_at(state, AppRoute::Home)
    }

    fn new_at(state: Arc<AppState>, route: AppRoute) -> Self {
        state.route.set(route.id() as u8);
        Self {
            router: Router::new(route).with_limit(50),
            rendered: route,
            pending: Arc::new(Mutex::new(None)),
            state,
        }
    }
}

impl Widget for ShowcaseShell {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        let mut changed = false;
        if let Some(intent) = self.pending.lock().unwrap().take() {
            match intent {
                NavIntent::Go(route) => {
                    if *self.router.current() != route {
                        self.router.push(route);
                        changed = true;
                    }
                }
                NavIntent::Back => {
                    if self.router.pop().is_some() {
                        changed = true;
                    }
                }
            }
        }
        if *self.router.current() != self.rendered {
            self.rendered = *self.router.current();
            self.state.route.set(self.rendered.id() as u8);
            changed = true;
        }
        if changed {
            cx.mark_children_dirty(id);
        }
    }

    // The key carries the route, so the shell subtree is rebuilt on navigation
    // and survives untouched while the route stays put. The page state that has
    // to survive lives in `AppState`.
    fn children(&self) -> Vec<Child> {
        let route = self.rendered;
        let can_pop = self.router.can_pop();
        let depth = self.router.depth();
        let pending = self.pending.clone();
        let state = self.state.clone();
        vec![Child::new(
            ChildKey::keyed(route.id()),
            LayoutRequest::new().width_pct(100.0).height_pct(100.0),
            move || {
                Box::new(build_shell(
                    route,
                    can_pop,
                    depth,
                    pending.clone(),
                    state.clone(),
                ))
            },
        )]
    }
}

impl WidgetAny for ShowcaseShell {
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

fn build_shell(
    route: AppRoute,
    can_pop: bool,
    depth: usize,
    pending: Arc<Mutex<Option<NavIntent>>>,
    state: Arc<AppState>,
) -> Surface {
    let body_pending = pending.clone();
    let body_state = state.clone();
    let status_state = state.clone();

    Surface::new()
        .background(Color::WHITE)
        .request(LayoutRequest::new().width_pct(100.0).height_pct(100.0))
        .child(Child::new(
            ChildKey::index(0),
            LayoutRequest::new().width_pct(100.0).height_pct(100.0),
            move || {
                Box::new(
                    Stack::new(Axis::Vertical)
                        .child(top_bar(route, can_pop, pending.clone()))
                        .child(body_row(route, body_pending.clone(), body_state.clone()))
                        .child(status_bar(route, depth, status_state.clone())),
                )
            },
        ))
}

fn top_bar(route: AppRoute, can_pop: bool, pending: Arc<Mutex<Option<NavIntent>>>) -> Child {
    Child::new(
        ChildKey::from("top-bar"),
        LayoutRequest::new().width_pct(100.0).height(58.0),
        move || {
            let mut right = Stack::new(Axis::Horizontal)
                .gap(12.0)
                .align(Align::Start, Align::Center)
                .child(label("path", route.path(), 12.0, FontWeight::Normal, FAINT));

            if can_pop {
                let pending = pending.clone();
                right = right.child(button_child(
                    "back",
                    "Back",
                    LayoutRequest::new().width(78.0).height(30.0),
                    move || {
                        *pending.lock().unwrap() = Some(NavIntent::Back);
                    },
                ));
            }

            let right = Arc::new(right);
            let brand = Stack::new(Axis::Horizontal)
                .gap(7.0)
                .align(Align::Start, Align::Center)
                .child(label("brand", "Floom", 18.0, FontWeight::Bold, ACCENT))
                .child(label(
                    "product",
                    "Showcase",
                    15.0,
                    FontWeight::Normal,
                    FAINT,
                ));
            let brand = Arc::new(brand);
            let right_slot = right.clone();
            let brand_slot = brand.clone();

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

fn body_row(
    route: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
    state: Arc<AppState>,
) -> Child {
    Child::new(
        ChildKey::from("body"),
        LayoutRequest::new().width_pct(100.0).fill(),
        move || {
            Box::new(
                Stack::new(Axis::Horizontal)
                    .child(sidebar(route, pending.clone()))
                    .child(content_area(route, pending.clone(), state.clone())),
            )
        },
    )
}

fn sidebar(route: AppRoute, pending: Arc<Mutex<Option<NavIntent>>>) -> Child {
    Child::new(
        ChildKey::from("sidebar"),
        LayoutRequest::new()
            .width(226.0)
            .min_width(226.0)
            .height_pct(100.0),
        move || {
            let mut column = Stack::new(Axis::Vertical).gap(4.0);
            column = column.child(section_label("overview", "OVERVIEW"));
            column = column.child(nav_item("home", AppRoute::Home, route, pending.clone()));
            column = column.child(section_label("demos", "DEMOS"));
            for item in [
                AppRoute::Components,
                AppRoute::Signals,
                AppRoute::Kits,
                AppRoute::Editor,
                AppRoute::Devtools,
            ] {
                column = column.child(nav_item(item.path(), item, route, pending.clone()));
            }
            column = column.child(spacer("fill", 8.0));
            column = column.child(label(
                "hint",
                "F12 opens devtools",
                11.0,
                FontWeight::Normal,
                FAINT,
            ));

            let column = Arc::new(column);
            let column_slot = column.clone();

            Box::new(
                Surface::new()
                    .background(SIDEBAR)
                    .border_edges(1.0, BORDER, Edges::RIGHT)
                    .padding(Insets {
                        top: 18.0,
                        right: 14.0,
                        bottom: 18.0,
                        left: 14.0,
                    })
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || Box::new((*column_slot).clone()),
                    )),
            )
        },
    )
}

fn nav_item(
    key: &'static str,
    item: AppRoute,
    current: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
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

fn content_area(
    route: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
    state: Arc<AppState>,
) -> Child {
    Child::new(
        ChildKey::from("content"),
        LayoutRequest::new().fill().min_width(0.0).height_pct(100.0),
        move || {
            let page_pending = pending.clone();
            let page_state = state.clone();
            Box::new(
                Surface::new()
                    .background(Color::WHITE)
                    // The slot's `min_width(0.0)` is merged against the
                    // widget's own request, so the box has to repeat it or the
                    // merge clears it and the page's max-content wins.
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
                            let page_pending = page_pending.clone();
                            let page_state = page_state.clone();
                            Box::new(Scroll::new().child(Child::new(
                                ChildKey::index(0),
                                LayoutRequest::new().width_pct(100.0),
                                move || {
                                    Box::new(page_for(
                                        route,
                                        page_state.clone(),
                                        page_pending.clone(),
                                    ))
                                },
                            )))
                        },
                    )),
            )
        },
    )
}

fn status_bar(route: AppRoute, depth: usize, state: Arc<AppState>) -> Child {
    Child::new(
        ChildKey::from("status"),
        LayoutRequest::new().width_pct(100.0).height(28.0),
        move || {
            let meta = format!("floom 2.0  ·  material kit  ·  history {}", depth);
            let live = state.clone();
            let right = Stack::new(Axis::Horizontal)
                .gap(14.0)
                .align(Align::Start, Align::Center)
                .child(signal_text(
                    "clicks",
                    Arc::new(move || format!("clicks {}", live.clicks.get())),
                    11.0,
                    FAINT,
                ))
                .child(label("meta", meta, 11.0, FontWeight::Normal, FAINT));
            let right = Arc::new(right);
            Box::new(
                Surface::new()
                    .background(SIDEBAR)
                    .border_edges(1.0, BORDER, Edges::TOP)
                    .padding_horiz(16.0)
                    .child(Child::new(
                        ChildKey::index(0),
                        LayoutRequest::new().width_pct(100.0).height_pct(100.0),
                        move || {
                            Box::new(
                                Stack::new(Axis::Horizontal)
                                    .align(Align::SpaceBetween, Align::Center)
                                    .child(label(
                                        "route",
                                        route.path(),
                                        11.0,
                                        FontWeight::Normal,
                                        FAINT,
                                    ))
                                    .child(Child::new(
                                        ChildKey::from("right"),
                                        LayoutRequest::new(),
                                        {
                                            let right = right.clone();
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

// ---------------------------------------------------------------------------
// Pages
// ---------------------------------------------------------------------------

fn page_for(
    route: AppRoute,
    state: Arc<AppState>,
    pending: Arc<Mutex<Option<NavIntent>>>,
) -> Stack {
    match route {
        AppRoute::Home => page_home(state, pending),
        AppRoute::Components => page_components(state),
        AppRoute::Signals => page_signals(state),
        AppRoute::Kits => page_kits(state),
        AppRoute::Editor => page_editor(state),
        AppRoute::Devtools => page_devtools(state),
    }
}

fn base_page(title: &str, subtitle: &str) -> Stack {
    Stack::new(Axis::Vertical)
        .gap(9.0)
        .child(label("page-title", title, 28.0, FontWeight::SemiBold, INK))
        .child(paragraph(
            "page-subtitle",
            subtitle,
            14.0,
            FontWeight::Normal,
            MUTED,
        ))
        .child(spacer("page-gap", 10.0))
}

fn page_home(state: Arc<AppState>, pending: Arc<Mutex<Option<NavIntent>>>) -> Stack {
    let _ = &state;
    let mut page = base_page(
        "Floom 2.0 Showcase",
        "One window over the engine, the reactive core, the ported kits, and the editor.",
    );

    page = page.child(paragraph(
        "intro",
        "Every page renders with the same retained widget tree and the same router.",
        13.0,
        FontWeight::Normal,
        FAINT,
    ));
    page = page.child(spacer("cards-gap", 8.0));

    let mut row1 = Stack::new(Axis::Horizontal).gap(16.0);
    row1 = row1.child(home_card(
        "card-components",
        "Components",
        "Button, Switch, Progress, Spinner, Divider and Text, mounted straight from floom-widgets.",
        AppRoute::Components,
        pending.clone(),
    ));
    row1 = row1.child(home_card(
        "card-signals",
        "Signals",
        "SyncBaseSignal counters and toggles, read back by a widget that tracks the value.",
        AppRoute::Signals,
        pending.clone(),
    ));
    page = page.child(Child::new(
        ChildKey::from("row1"),
        LayoutRequest::new().width_pct(100.0).height(168.0),
        move || Box::new(row1.clone()),
    ));

    let mut row2 = Stack::new(Axis::Horizontal).gap(16.0);
    row2 = row2.child(home_card(
        "card-kits",
        "Kits",
        "The ported material adapters from the floom-ui satellite workspace.",
        AppRoute::Kits,
        pending.clone(),
    ));
    row2 = row2.child(home_card(
        "card-editor",
        "Editor",
        "floom-editor's TextDocument, transactions, undo and caret, in one widget.",
        AppRoute::Editor,
        pending.clone(),
    ));
    page = page.child(Child::new(
        ChildKey::from("row2"),
        LayoutRequest::new().width_pct(100.0).height(168.0),
        move || Box::new(row2.clone()),
    ));

    let devtools_pending = pending.clone();
    page = page.child(card(
        "card-devtools",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .align(Align::SpaceBetween, Align::Start)
            .child(label("t", "Devtools", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "Press F12 to open the live element tree; Ctrl+Shift+C arms the pick.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(go_button(
                "open",
                "Open",
                AppRoute::Devtools,
                devtools_pending,
            )),
    ));

    page
}

fn home_card(
    key: &'static str,
    title: &'static str,
    blurb: &'static str,
    route: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
) -> Child {
    let body = Stack::new(Axis::Vertical)
        .gap(6.0)
        .align(Align::SpaceBetween, Align::Start)
        .child(label("t", title, 16.0, FontWeight::SemiBold, INK))
        .child(paragraph("b", blurb, 13.0, FontWeight::Normal, MUTED))
        .child(go_button("open", "Open", route, pending));
    let mut child = card(key, 168.0, body);
    // Half of a two-up row: a zero basis plus grow gives both cards the same
    // width. `fill()` alone keeps each card's content-sized basis, so the
    // longer blurb would make its card wider than its neighbour.
    child.layout = LayoutRequest::new()
        .width(0.0)
        .fill()
        .min_width(0.0)
        .height(168.0);
    child
}

fn page_components(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Components",
        "The engine's Tier-0 widgets, mounted directly with no kit layer in between.",
    );

    let clicks = state.clone();
    let click_count = state.clone();
    let clicks_2 = state.clone();
    let click_count_2 = state.clone();

    let buttons = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(button_child(
            "primary",
            "Primary",
            LayoutRequest::new().width(104.0).height(36.0),
            {
                let state = clicks.clone();
                move || {
                    state.clicks.update(|n| *n += 1);
                }
            },
        ))
        .child(button_child(
            "secondary",
            "Secondary",
            LayoutRequest::new().width(112.0).height(36.0),
            {
                let state = clicks_2.clone();
                move || {
                    state.clicks.update(|n| *n += 1);
                }
            },
        ))
        .child(disabled_button("disabled", "Disabled"));

    page = page.child(card(
        "buttons",
        176.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Buttons", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "The two live buttons share a counter; the readout is a signal-backed widget.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("buttons-row"),
                LayoutRequest::new().height(36.0),
                move || Box::new(buttons.clone()),
            ))
            .child(signal_text(
                "clicks-readout",
                {
                    let state = click_count.clone();
                    Arc::new(move || format!("clicks = {}", state.clicks.get()))
                },
                13.0,
                INK,
            )),
    ));

    let switch_state = state.clone();
    let bright_read = state.clone();
    let bright_read_2 = state.clone();
    let _ = click_count_2;

    let controls = Stack::new(Axis::Horizontal)
        .gap(22.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("switch"),
            LayoutRequest::new().height(28.0),
            move || {
                let state = switch_state.clone();
                Box::new(
                    Switch::new("Wi-Fi")
                        .on(state.bright.get())
                        .on_toggle(move |on| state.bright.set(on)),
                )
            },
        ))
        .child(signal_text(
            "bright-readout",
            {
                let state = bright_read.clone();
                Arc::new(move || {
                    format!("Wi-Fi is {}", if state.bright.get() { "on" } else { "off" })
                })
            },
            13.0,
            MUTED,
        ))
        .child(Child::new(
            ChildKey::from("progress"),
            LayoutRequest::new().width(190.0).height(8.0),
            move || Box::new(Progress::determinate(0.42)),
        ))
        .child(Child::new(
            ChildKey::from("progress-busy"),
            LayoutRequest::new().width(190.0).height(8.0),
            move || Box::new(Progress::indeterminate()),
        ))
        .child(Child::new(
            ChildKey::from("spinner"),
            LayoutRequest::new().width(26.0).height(26.0),
            move || Box::new(Spinner::new().size(24.0).color(ACCENT).track(BORDER)),
        ));

    page = page.child(card(
        "controls",
        168.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "Toggles, progress and motion",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b",
                "Switch toggles in place; the two progress bars and the spinner animate themselves.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("controls-row"),
                LayoutRequest::new().height(36.0),
                move || Box::new(controls.clone()),
            ))
            .child(signal_text(
                "bright-readout-slot",
                {
                    let state = bright_read_2.clone();
                    Arc::new(move || {
                        format!(
                            "wired to the same signal: {}",
                            if state.bright.get() { "on" } else { "off" }
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page = page.child(card(
        "text",
        176.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label("h", "Typography", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "body",
                "Text is a leaf with an intrinsic measure: the box is the run, not a magic number.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "mono",
                "Family and weight are part of the measure as well as the paint.",
                13.0,
                FontWeight::Medium,
                INK,
            ))
            .child(Child::new(
                ChildKey::from("divider"),
                LayoutRequest::new().width_pct(100.0).height(1.0),
                move || Box::new(Divider::new().color(BORDER)),
            ))
            .child(paragraph(
                "after",
                "Divider above: a rule whose colour is a value, not a theme lookup.",
                12.0,
                FontWeight::Normal,
                FAINT,
            )),
    ));

    page
}

fn page_signals(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Signals",
        "floom-signal's SyncBaseSignal: the thread-safe variant, which is the one a page can hand to a child builder.",
    );

    let counter_row = {
        let dec = state.clone();
        let inc = state.clone();
        let reset = state.clone();
        Stack::new(Axis::Horizontal)
            .gap(10.0)
            .align(Align::Start, Align::Center)
            .child(button_child(
                "dec",
                "-1",
                LayoutRequest::new().width(60.0).height(34.0),
                move || {
                    dec.signal_count.update(|n| *n -= 1);
                },
            ))
            .child(signal_text(
                "value",
                {
                    let state = state.clone();
                    Arc::new(move || format!("value = {}", state.signal_count.get()))
                },
                18.0,
                INK,
            ))
            .child(button_child(
                "inc",
                "+1",
                LayoutRequest::new().width(60.0).height(34.0),
                move || {
                    inc.signal_count.update(|n| *n += 1);
                },
            ))
            .child(button_child(
                "reset",
                "Reset",
                LayoutRequest::new().width(76.0).height(34.0),
                move || {
                    reset.signal_count.set(3);
                },
            ))
    };

    let parity = {
        let state = state.clone();
        Arc::new(move || {
            let n = state.signal_count.get();
            format!(
                "read back every frame: {} is {}",
                n,
                if n % 2 == 0 { "even" } else { "odd" }
            )
        })
    };

    page = page.child(card(
        "counter",
        170.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Counter", 16.0, FontWeight::SemiBold, INK))
            .child(paragraph(
                "b",
                "The buttons write the signal; the readout widget asks for the current value in update.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("counter-row"),
                LayoutRequest::new().height(40.0),
                move || Box::new(counter_row.clone()),
            ))
            .child(signal_text("parity", parity, 12.0, FAINT)),
    ));

    let toggle_state = state.clone();
    let toggle_read = state.clone();
    let toggles = Stack::new(Axis::Horizontal)
        .gap(14.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("flag"),
            LayoutRequest::new().height(28.0),
            move || {
                let state = toggle_state.clone();
                Box::new(
                    Switch::new("Feature flag")
                        .on(state.signal_flag.get())
                        .on_toggle(move |on| state.signal_flag.set(on)),
                )
            },
        ))
        .child(signal_text(
            "flag-readout",
            {
                let state = toggle_read.clone();
                Arc::new(move || {
                    format!(
                        "feature = {}",
                        if state.signal_flag.get() {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    )
                })
            },
            13.0,
            INK,
        ));

    page = page.child(card(
        "toggle",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Start)
            .child(label("t", "Toggle", 16.0, FontWeight::SemiBold, INK))
            .child(Child::new(
                ChildKey::from("toggle-row"),
                LayoutRequest::new().height(32.0),
                move || Box::new(toggles.clone()),
            )),
    ));

    page = page.child(card(
        "why",
        148.0,
        Stack::new(Axis::Vertical)
            .gap(6.0)
            .child(label(
                "t",
                "Why SyncBaseSignal",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b1",
                "Child::new builders are Fn + Send + Sync, so an Rc-backed RwSignal cannot be captured by one.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "b2",
                "SyncBaseSignal stores its value in the sync runtime and can be cloned into those builders.",
                13.0,
                FontWeight::Normal,
                MUTED,
            )),
    ));

    page
}

fn page_kits(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Kits",
        "Only the ported material crates are imported: md-text, md-button and md-checkbox.",
    );

    page = page.child(paragraph(
        "note",
        "The floom-ui satellite workspace has 300+ members; each is its own crate, so a demo names the ones it shows.",
        12.0,
        FontWeight::Normal,
        FAINT,
    ));

    page = page.child(card(
        "text",
        128.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label(
                "t",
                "floom-widget-md-text",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(Child::new(
                ChildKey::from("material-text"),
                LayoutRequest::new(),
                move || {
                    Box::new(
                        MaterialText::new("Material body text, resolved from the theme").view(),
                    )
                },
            )),
    ));

    let kit_clicks = state.clone();
    let kit_read = state.clone();
    let kit_buttons = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(Child::new(
            ChildKey::from("kit-primary"),
            LayoutRequest::new().height(40.0),
            {
                let state = kit_clicks.clone();
                move || {
                    let state = state.clone();
                    Box::new(
                        MdButton::new()
                            .label("Primary")
                            .on_click(move || {
                                state.kit_clicks.update(|n| *n += 1);
                            })
                            .view(),
                    )
                }
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-secondary"),
            LayoutRequest::new().height(40.0),
            {
                let state = kit_clicks.clone();
                move || {
                    let state = state.clone();
                    Box::new(
                        MdButton::new()
                            .label("Secondary")
                            .variant(ButtonVariant::Secondary)
                            .on_click(move || {
                                state.kit_clicks.update(|n| *n += 1);
                            })
                            .view(),
                    )
                }
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-ghost"),
            LayoutRequest::new().height(40.0),
            move || {
                Box::new(
                    MdButton::new()
                        .label("Ghost")
                        .variant(ButtonVariant::Ghost)
                        .view(),
                )
            },
        ))
        .child(Child::new(
            ChildKey::from("kit-disabled"),
            LayoutRequest::new().height(40.0),
            move || Box::new(MdButton::new().label("Disabled").disabled(true).view()),
        ));

    page = page.child(card(
        "buttons",
        180.0,
        Stack::new(Axis::Vertical)
            .gap(12.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "floom-widget-md-button",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "b",
                "Four variants from the same adapter; the primary and secondary share a counter.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("kit-buttons-row"),
                LayoutRequest::new().height(40.0),
                move || Box::new(kit_buttons.clone()),
            ))
            .child(signal_text(
                "kit-readout",
                {
                    let state = kit_read.clone();
                    Arc::new(move || format!("kit clicks = {}", state.kit_clicks.get()))
                },
                12.0,
                FAINT,
            )),
    ));

    let checkbox_state = state.clone();
    let checkbox_read = state.clone();
    let checkbox = Child::new(
        ChildKey::from("checkbox"),
        LayoutRequest::new().height(28.0),
        move || {
            let state = checkbox_state.clone();
            Box::new(
                MdCheckbox::new("Receive updates")
                    .checked(state.kit_checked.get())
                    .on_change(move |checked| state.kit_checked.set(checked))
                    .view(),
            )
        },
    );

    page = page.child(card(
        "checkbox",
        132.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Start)
            .child(label(
                "t",
                "floom-widget-md-checkbox",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(checkbox)
            .child(signal_text(
                "check-readout",
                {
                    let state = checkbox_read.clone();
                    Arc::new(move || {
                        format!(
                            "checked = {}",
                            if state.kit_checked.get() {
                                "true"
                            } else {
                                "false"
                            }
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page
}

fn page_editor(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Editor",
        "floom-editor's document model, mounted as a widget: rope, transactions, undo, selection and caret.",
    );

    let editor = state.editor();

    let undo_editor = editor.clone();
    let redo_editor = editor.clone();
    let reset_editor = editor.clone();

    let toolbar = Stack::new(Axis::Horizontal)
        .gap(10.0)
        .align(Align::Start, Align::Center)
        .child(button_child(
            "undo",
            "Undo",
            LayoutRequest::new().width(86.0).height(34.0),
            move || {
                undo_editor.lock().unwrap().undo();
            },
        ))
        .child(button_child(
            "redo",
            "Redo",
            LayoutRequest::new().width(86.0).height(34.0),
            move || {
                redo_editor.lock().unwrap().redo();
            },
        ))
        .child(button_child(
            "reset",
            "Reset sample",
            LayoutRequest::new().width(118.0).height(34.0),
            move || {
                reset_editor.lock().unwrap().set_text(SAMPLE);
            },
        ));

    page = page.child(paragraph(
        "hint",
        "Click the editor to focus it. Mod+Z / Mod+Y, Enter, Backspace and Delete are handled by the widget.",
        13.0,
        FontWeight::Normal,
        MUTED,
    ));

    page = page.child(Child::new(
        ChildKey::from("toolbar"),
        LayoutRequest::new().width_pct(100.0).height(36.0),
        move || Box::new(toolbar.clone()),
    ));

    page = page.child(card(
        "editor",
        470.0,
        Stack::new(Axis::Vertical)
            .gap(10.0)
            .align(Align::Start, Align::Stretch)
            .child(label(
                "label",
                "sample.txt",
                12.0,
                FontWeight::Medium,
                FAINT,
            ))
            .child(Child::new(
                ChildKey::from("editor-slot"),
                LayoutRequest::new().width_pct(100.0).height(410.0),
                move || Box::new(SharedEditor::new(editor.clone())),
            )),
    ));

    page
}

fn page_devtools(state: Arc<AppState>) -> Stack {
    let mut page = base_page(
        "Devtools",
        "The panel is attached by run_desktop_with_devtools, so it is wired for the whole app.",
    );

    page = page.child(card(
        "shortcuts",
        250.0,
        Stack::new(Axis::Vertical)
            .gap(8.0)
            .child(label(
                "t",
                "Shortcuts",
                16.0,
                FontWeight::SemiBold,
                INK,
            ))
            .child(paragraph(
                "f12",
                "F12  (or Cmd/Ctrl+Shift+I)  —  show or hide the panel",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "pick",
                "Ctrl+Shift+C  —  arm the pick, then click any element to inspect it",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(paragraph(
                "escape",
                "Escape  —  close the panel, and only while it is open",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(Child::new(
                ChildKey::from("divider"),
                LayoutRequest::new().width_pct(100.0).height(1.0),
                move || Box::new(Divider::new().color(BORDER)),
            ))
            .child(paragraph(
                "readout",
                "The panel's readout is answered from this app's own state: route, engine clicks, signal value and kit clicks.",
                13.0,
                FontWeight::Normal,
                MUTED,
            ))
            .child(signal_text(
                "live",
                {
                    let state = state.clone();
                    Arc::new(move || {
                        format!(
                            "live: route {}, engine clicks {}, signal {}",
                            AppRoute::ALL
                                .into_iter()
                                .find(|r| r.id() as u8 == state.route.get())
                                .map(|r| r.path())
                                .unwrap_or("/"),
                            state.clicks.get(),
                            state.signal_count.get()
                        )
                    })
                },
                12.0,
                FAINT,
            )),
    ));

    page
}

// ---------------------------------------------------------------------------
// Small building blocks
// ---------------------------------------------------------------------------

fn label(
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
fn paragraph(
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

fn section_label(key: &'static str, content: &'static str) -> Child {
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

fn spacer(key: &'static str, height: f64) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width_pct(100.0).height(height as f32),
        move || Box::new(Spacer::new(height)),
    )
}

fn card(key: &'static str, height: f32, body: Stack) -> Child {
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

fn button_child(
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

fn disabled_button(key: &'static str, text: &'static str) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width(104.0).height(36.0),
        move || Box::new(Button::new(text).radius(6.0).disabled(true)),
    )
}

fn go_button(
    key: &'static str,
    text: &'static str,
    route: AppRoute,
    pending: Arc<Mutex<Option<NavIntent>>>,
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

fn signal_text(
    key: &'static str,
    read: Arc<dyn Fn() -> String + Send + Sync>,
    size: f32,
    color: Color,
) -> Child {
    Child::new(ChildKey::from(key), LayoutRequest::new(), move || {
        Box::new(SignalText::new(read.clone()).size(size).color(color))
    })
}

// ---------------------------------------------------------------------------
// Signal-backed text
// ---------------------------------------------------------------------------

/// A `Text` whose content comes from a signal.
///
/// A `Text` child is rebuilt only when its parent re-declares children, so this
/// parent polls the signal in `update` and marks its child dirty on a change.
/// The host updates every node each frame, which is what makes the read cheap.
struct SignalText {
    read: Arc<dyn Fn() -> String + Send + Sync>,
    last: String,
    size: f32,
    color: Color,
}

impl SignalText {
    fn new(read: Arc<dyn Fn() -> String + Send + Sync>) -> Self {
        Self {
            read,
            last: String::new(),
            size: 13.0,
            color: INK,
        }
    }

    fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    fn color(mut self, color: Color) -> Self {
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

// ---------------------------------------------------------------------------
// Shared editor
// ---------------------------------------------------------------------------

/// Delegates to an `EditorWidget` that outlives the page's widget values.
struct SharedEditor {
    inner: Arc<Mutex<EditorWidget>>,
}

impl SharedEditor {
    fn new(inner: Arc<Mutex<EditorWidget>>) -> Self {
        Self { inner }
    }
}

impl Widget for SharedEditor {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        self.inner.lock().unwrap().create(cx)
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        self.inner.lock().unwrap().update(cx, id);
    }

    fn layout_request(&self) -> Option<LayoutRequest> {
        self.inner.lock().unwrap().layout_request()
    }

    fn paint_style(&self) -> Option<floom::floom_core::PaintStyle> {
        self.inner.lock().unwrap().paint_style()
    }

    fn event(&mut self, cx: &mut dyn EventContext, id: ElementId) -> bool {
        self.inner.lock().unwrap().event(cx, id)
    }

    fn paint(&self, cx: &mut dyn PaintContext, id: ElementId) {
        self.inner.lock().unwrap().paint(cx, id);
    }

    fn semantics(&self, cx: &mut dyn SemanticsContext, id: ElementId) {
        self.inner.lock().unwrap().semantics(cx, id);
    }
}

impl WidgetAny for SharedEditor {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<SemanticsHint> {
        self.inner.lock().unwrap().semantics_hint()
    }
}

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

/// Mounts and pumps every route without a window, for the golden-example gate.
fn run_headless() {
    let mut total = 0;
    for route in AppRoute::ALL {
        let state = Arc::new(AppState::new());
        let mut host = WidgetHost::new();
        host.set_size(WINDOW);
        host.mount_root(LayoutStyle::default(), ShowcaseShell::new_at(state, route));
        let mut painted = 0;
        for _ in 0..3 {
            painted = host.frame().painted;
        }
        assert!(painted > 0, "{} painted no widgets", route.path());
        total += painted;
        println!("{:<12} painted {}", route.path(), painted);
    }
    // The router is the one piece with no unit test in an engine crate: it is
    // the shell's own wiring, so exercise a push and a pop through a real host.
    let state = Arc::new(AppState::new());
    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    let root = host.mount_root(LayoutStyle::default(), ShowcaseShell::new(state));
    host.frame();
    for route in [
        AppRoute::Components,
        AppRoute::Kits,
        AppRoute::Editor,
        AppRoute::Devtools,
    ] {
        if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
            *shell.pending.lock().unwrap() = Some(NavIntent::Go(route));
        }
        host.frame();
        let rendered = host
            .widget::<ShowcaseShell>(root)
            .map(|shell| shell.rendered)
            .expect("the shell is the root");
        assert_eq!(rendered, route, "router did not reach {}", route.path());
    }
    if let Some(shell) = host.widget_mut::<ShowcaseShell>(root) {
        *shell.pending.lock().unwrap() = Some(NavIntent::Back);
    }
    host.frame();
    let rendered = host
        .widget::<ShowcaseShell>(root)
        .map(|shell| shell.rendered)
        .expect("the shell is the root");
    assert_eq!(rendered, AppRoute::Editor, "Back did not pop to /editor");

    println!(
        "showcase: {} routes mounted, {} paints total; router push/back ok",
        AppRoute::ALL.len(),
        total
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--headless") {
        run_headless();
        return;
    }

    let state = Arc::new(AppState::new());
    let shell_state = state.clone();
    let readout_state = state.clone();

    let result = floom::run_desktop_with_devtools(
        "Floom 2.0 — Showcase",
        WINDOW,
        move |host: &mut WidgetHost| {
            host.mount_root(
                LayoutStyle::default(),
                ShowcaseShell::new(shell_state.clone()),
            );
        },
        move |_panel| {
            let route = AppRoute::ALL
                .into_iter()
                .find(|r| r.id() as u8 == readout_state.route.get())
                .map(|r| r.path())
                .unwrap_or("/");
            vec![
                floom::devtools::ReadoutRow::heading("showcase"),
                floom::devtools::ReadoutRow::new("route", route),
                floom::devtools::ReadoutRow::new(
                    "engine clicks",
                    readout_state.clicks.get().to_string(),
                ),
                floom::devtools::ReadoutRow::new(
                    "signal value",
                    readout_state.signal_count.get().to_string(),
                ),
                floom::devtools::ReadoutRow::new(
                    "kit clicks",
                    readout_state.kit_clicks.get().to_string(),
                ),
            ]
        },
    );

    if let Err(err) = result {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}
