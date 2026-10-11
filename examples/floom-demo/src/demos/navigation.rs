//! Navigation: the containers and the page host.

use floom_core::{Axis, LayoutRequest, WidgetAny};
use floom_widgets::{
    NavDestination, NavHandle, NavHost, NavigationBar, NavigationDrawer, NavigationRail, Stack,
    Text,
};

use super::{Demo, Section};

pub(super) fn demos() -> Vec<Demo> {
    vec![
        Demo {
            id: "nav-bar",
            title: "NavigationBar",
            section: Section::Navigation,
            build: build_bar,
        },
        Demo {
            id: "nav-rail",
            title: "NavigationRail",
            section: Section::Navigation,
            build: build_rail,
        },
        Demo {
            id: "nav-drawer",
            title: "NavigationDrawer",
            section: Section::Navigation,
            build: build_drawer,
        },
        Demo {
            id: "nav-host",
            title: "NavHost",
            section: Section::Navigation,
            build: build_host,
        },
    ]
}

fn destinations() -> [NavDestination; 3] {
    [
        NavDestination::new("Home"),
        NavDestination::new("Search"),
        NavDestination::new("Profile"),
    ]
}

fn build_bar() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(320.0).height(56.0),
            || Box::new(NavigationBar::new().destinations(destinations())),
        ),
    )
}

fn build_rail() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(88.0).height(200.0),
            || Box::new(NavigationRail::new().destinations(destinations())),
        ),
    )
}

fn build_drawer() -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(220.0).height(200.0),
            || Box::new(NavigationDrawer::new().header("menu").destinations(destinations())),
        ),
    )
}

/// A page stack with two entries: the root plus one pushed page. The host
/// renders whatever its builder returns per page, so the gallery shows the
/// top page's path.
fn build_host() -> Box<dyn WidgetAny> {
    let handle = NavHandle::new("/home");
    handle.push("/profile/42");

    Box::new(
        Stack::new(Axis::Vertical).push_layout(
            LayoutRequest::new().width(240.0).height(60.0),
            move || {
                let handle = handle.clone();
                Box::new(NavHost::new(handle, |page| {
                    Box::new(Text::new(page.path.clone()))
                }))
            },
        ),
    )
}
