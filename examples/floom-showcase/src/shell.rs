//! The shell: a retained widget tree whose keyed child is rebuilt on route change.
//!
//! The shell owns the `Router<AppRoute>`. The router is widget-free, so when
//! `Router::current` changes the shell marks its subtree dirty and Floom
//! rebuilds only the keyed child — the page subtree survives untouched while
//! the route stays put. Page state that must survive lives in `AppState`.

use std::sync::{Arc, Mutex};

use floom::floom_core::{
    Align, Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, SemanticsHint,
    UpdateContext, Widget, WidgetAny,
};
use floom::floom_types::{Color, Edges, FontWeight, Insets};
use floom::floom_widgets::{Scroll, Stack, Surface};
use floom_signal::{SignalGet, SignalUpdate};

use crate::pages::page_for;
use crate::routes::{AppRoute, NavIntent};
use crate::state::AppState;
use crate::widgets::{ACCENT, BORDER, SIDEBAR};

/// Surface that owns the whole window: top bar, body row, status bar.
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
                .child(crate::widgets::label(
                    "path",
                    route.path(),
                    12.0,
                    FontWeight::Normal,
                    crate::widgets::FAINT,
                ));

            if can_pop {
                let pending = pending.clone();
                right = right.child(crate::widgets::button_child(
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
                .child(crate::widgets::label(
                    "brand",
                    "Floom",
                    18.0,
                    FontWeight::Bold,
                    ACCENT,
                ))
                .child(crate::widgets::label(
                    "product",
                    "Showcase",
                    15.0,
                    FontWeight::Normal,
                    crate::widgets::FAINT,
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
            column = column.child(crate::widgets::section_label("overview", "OVERVIEW"));
            column = column.child(crate::widgets::nav_item(
                "home",
                AppRoute::Home,
                route,
                pending.clone(),
            ));
            column = column.child(crate::widgets::section_label("demos", "DEMOS"));
            for item in [
                AppRoute::Components,
                AppRoute::Signals,
                AppRoute::Kits,
                AppRoute::Editor,
                AppRoute::Devtools,
            ] {
                column = column.child(crate::widgets::nav_item(
                    item.path(),
                    item,
                    route,
                    pending.clone(),
                ));
            }
            column = column.child(crate::widgets::spacer("fill", 8.0));
            column = column.child(crate::widgets::label(
                "hint",
                "F12 opens devtools",
                11.0,
                FontWeight::Normal,
                crate::widgets::FAINT,
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
                .child(crate::widgets::signal_text(
                    "clicks",
                    Arc::new(move || format!("clicks {}", live.clicks.get())),
                    11.0,
                    crate::widgets::FAINT,
                ))
                .child(crate::widgets::label(
                    "meta",
                    meta,
                    11.0,
                    FontWeight::Normal,
                    crate::widgets::FAINT,
                ));
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
                                    .child(crate::widgets::label(
                                        "route",
                                        route.path(),
                                        11.0,
                                        FontWeight::Normal,
                                        crate::widgets::FAINT,
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

/// The shell widget: owns the router and rebuilds its keyed child on route
/// change.
pub struct ShowcaseShell {
    router: floom_navigation::Router<AppRoute>,
    pub rendered: AppRoute,
    pub pending: Arc<Mutex<Option<NavIntent>>>,
    state: Arc<AppState>,
}

impl ShowcaseShell {
    pub fn new(state: Arc<AppState>) -> Self {
        Self::new_at(state, AppRoute::Home)
    }

    pub fn new_at(state: Arc<AppState>, route: AppRoute) -> Self {
        state.route.set(route.id() as u8);
        Self {
            router: floom_navigation::Router::new(route).with_limit(50),
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
