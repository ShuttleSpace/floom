//! The components that arrange the page around the content.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times. A few
//! of them are shells -- `App`, `Main` and `Window` wrap a whole region rather
//! than a single control -- and `Divider` ends on `divider()` rather than
//! `view()`, because it resolves straight to an engine primitive.

use std::sync::Arc;

use floom::floom_core::WidgetAny;
use floom::floom_widgets::Text;
use floom_core_theme::props::divider::DividerOrientation;
use floom_core_theme::props::navigation_drawer::NavigationDrawerSide;

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::{compare_children, compare_with};
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, widget};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("app", "App"),
    ("app_bar", "AppBar"),
    ("bottom_navigation", "BottomNavigation"),
    ("breadcrumbs", "Breadcrumbs"),
    ("container", "Container"),
    ("divider", "Divider"),
    ("footer", "Footer"),
    ("grid", "Grid"),
    ("hover", "Hover"),
    ("main", "Main"),
    ("navigation_drawer", "NavigationDrawer"),
    ("parallax", "Parallax"),
    ("responsive", "Responsive"),
    ("slide_group", "SlideGroup"),
    ("spacer", "Spacer"),
    ("system_bar", "SystemBar"),
    ("tabs", "Tabs"),
    ("toolbar", "Toolbar"),
    ("window", "Window"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| match *component {
            "app" => app_page(),
            "app_bar" => app_bar_page(),
            "bottom_navigation" => bottom_navigation_page(),
            "breadcrumbs" => breadcrumbs_page(),
            "container" => container_page(),
            "divider" => divider_page(),
            "footer" => footer_page(),
            "grid" => grid_page(),
            "hover" => hover_page(),
            "main" => main_page(),
            "navigation_drawer" => navigation_drawer_page(),
            "parallax" => parallax_page(),
            "responsive" => responsive_page(),
            "slide_group" => slide_group_page(),
            "spacer" => spacer_page(),
            "system_bar" => system_bar_page(),
            "tabs" => tabs_page(),
            "toolbar" => toolbar_page(),
            "window" => window_page(),
            _ => page(component, name, Category::Layout),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn layout_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::Layout);
    page.intro = intro;
    page.samples = samples;
    page.api = api;
    page
}

/// The `view` terminal, as an API method row.
fn view_method(sig: &'static str, ret: &'static str) -> Vec<ApiRow> {
    vec![ApiRow::new(
        "view",
        sig,
        ret,
        "Build the engine node this component renders.",
    )]
}

/// A short piece of placeholder content, as a `Child`.
fn content(key: &'static str, text: &'static str) -> floom::floom_core::Child {
    widget(key, move || Box::new(Text::new(text.to_string())))
}

// -- App ------------------------------------------------------------------

/// The App page.
fn app_page() -> ComponentPage {
    layout_page(
        "app",
        "App",
        "The root shell: it fills the viewport and carries the system bar, the \
         app bar and the main region. A reader mounts it once, at the top of \
         the tree, and hands it the screen's body through `child`.",
        vec![
            Sample {
                title: "With a body",
                note: Some("App is the shell; the area you see is the whole app frame."),
                build: app_with_body,
            },
            Sample {
                title: "Empty",
                note: None,
                build: app_empty,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// The app, with a body handed through `child`.
fn app_with_body() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::App,
        ds::cupertino::App,
        ds::fluent::App,
        |b, theme| b::new()
            .with_theme(theme)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The app, with no body -- just the shell background.
fn app_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::App,
        ds::cupertino::App,
        ds::fluent::App,
        |b, theme| b::new().with_theme(theme).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- AppBar ---------------------------------------------------------------

/// The AppBar page.
fn app_bar_page() -> ComponentPage {
    layout_page(
        "app_bar",
        "AppBar",
        "The bar across the top of a screen, with a title, an optional leading \
         glyph, and trailing actions.",
        vec![
            Sample {
                title: "With a title",
                note: None,
                build: app_bar_title,
            },
            Sample {
                title: "With leading and actions",
                note: None,
                build: app_bar_actions,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// The app bar, with a title.
fn app_bar_title() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::AppBar,
        ds::cupertino::AppBar,
        ds::fluent::AppBar,
        |b, theme| b::new().with_theme(theme).title("Settings").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The app bar, with a leading glyph and actions.
fn app_bar_actions() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::AppBar,
        ds::cupertino::AppBar,
        ds::fluent::AppBar,
        |b, theme| b::new()
            .with_theme(theme)
            .title("Settings")
            .leading(content("lead", "≡"))
            .actions(vec![content("act", "Save")])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- BottomNavigation -----------------------------------------------------

/// The BottomNavigation page.
fn bottom_navigation_page() -> ComponentPage {
    layout_page(
        "bottom_navigation",
        "BottomNavigation",
        "A bar of destinations along the bottom edge of the screen. Each \
         destination carries an icon and a label, and selecting one reports \
         its index.",
        vec![
            Sample {
                title: "Two destinations",
                note: None,
                build: bottom_nav_two,
            },
            Sample {
                title: "Second selected",
                note: Some("`selected` picks the open destination on first paint."),
                build: bottom_nav_selected,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the chosen destination's index when the reader taps it.",
            )],
            methods: view_method(
                "self -> floom_ui_composites::BottomNavigationBar",
                "BottomNavigationBar",
            ),
            ..Default::default()
        },
    )
}

/// Two destinations, the first selected.
fn bottom_nav_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::BottomNavigation,
        ds::cupertino::BottomNavigation,
        ds::fluent::BottomNavigation,
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, x, theme| {
            let icon = |name: &'static str| {
                let t = theme.clone();
                Arc::new(move || -> Box<dyn WidgetAny> {
                    Box::new(x::new(name).with_theme(t.clone()).resolve())
                })
            };
            b::new()
                .with_theme(theme.clone())
                .labels(vec!["Home".into(), "Search".into()])
                .icons(vec![icon("home"), icon("search")])
                .selected(0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// Two destinations, the second selected.
fn bottom_nav_selected() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_with!(
        ds::material::BottomNavigation,
        ds::cupertino::BottomNavigation,
        ds::fluent::BottomNavigation,
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, x, theme| {
            let icon = |name: &'static str| {
                let t = theme.clone();
                Arc::new(move || -> Box<dyn WidgetAny> {
                    Box::new(x::new(name).with_theme(t.clone()).resolve())
                })
            };
            b::new()
                .with_theme(theme.clone())
                .labels(vec!["Home".into(), "Search".into()])
                .icons(vec![icon("home"), icon("search")])
                .selected(1)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Breadcrumbs ----------------------------------------------------------

/// The Breadcrumbs page.
fn breadcrumbs_page() -> ComponentPage {
    layout_page(
        "breadcrumbs",
        "Breadcrumbs",
        "A trail of links showing where the current page sits in a hierarchy. \
         Tapping a crumb reports its index.",
        vec![
            Sample {
                title: "A short trail",
                note: None,
                build: breadcrumbs_short,
            },
            Sample {
                title: "A longer trail",
                note: None,
                build: breadcrumbs_long,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the tapped crumb's index.",
            )],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A short trail.
fn breadcrumbs_short() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Breadcrumbs,
        ds::cupertino::Breadcrumbs,
        ds::fluent::Breadcrumbs,
        |b, theme| b::new()
            .with_theme(theme)
            .items(vec!["Home".into(), "Docs".into(), "Buttons".into()])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A longer trail.
fn breadcrumbs_long() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Breadcrumbs,
        ds::cupertino::Breadcrumbs,
        ds::fluent::Breadcrumbs,
        |b, theme| b::new()
            .with_theme(theme)
            .items(vec!["A".into(), "B".into(), "C".into(), "D".into(),])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Container ------------------------------------------------------------

/// The Container page.
fn container_page() -> ComponentPage {
    layout_page(
        "container",
        "Container",
        "A centred column that caps its width so content stays readable on wide \
         screens, and can run edge to edge when fluid.",
        vec![
            Sample {
                title: "Fixed width",
                note: None,
                build: container_fixed,
            },
            Sample {
                title: "Fluid",
                note: Some("`fluid` lets the container run to the screen edge."),
                build: container_fluid,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// A fixed-width, padded container.
fn container_fixed() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Container,
        ds::cupertino::Container,
        ds::fluent::Container,
        |b, theme| b::new()
            .with_theme(theme)
            .max_width(480.0)
            .padding(16.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A fluid container.
fn container_fluid() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Container,
        ds::cupertino::Container,
        ds::fluent::Container,
        |b, theme| b::new()
            .with_theme(theme)
            .fluid(true)
            .padding(24.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The content shown inside a container sample.
fn content_inside() -> floom::floom_widgets::Text {
    floom::floom_widgets::Text::new("Content goes here".to_string())
}

// -- Divider --------------------------------------------------------------

/// The Divider page.
fn divider_page() -> ComponentPage {
    layout_page(
        "divider",
        "Divider",
        "A thin rule that separates content. It resolves straight to an engine \
         divider through `divider()` rather than `view()`.",
        vec![
            Sample {
                title: "Horizontal",
                note: None,
                build: divider_horizontal,
            },
            Sample {
                title: "Vertical",
                note: Some("The vertical divider needs height from its container to show."),
                build: divider_vertical,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "divider",
                "&self -> floom_widgets::Divider",
                "Divider",
                "Build the engine divider this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// A horizontal divider.
fn divider_horizontal() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Divider,
        ds::cupertino::Divider,
        ds::fluent::Divider,
        |b, theme| b::new()
            .with_theme(theme)
            .orientation(DividerOrientation::Horizontal)
            .divider()
    );
    Box::new(compare_row((m, c, f)))
}

/// A vertical divider.
fn divider_vertical() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Divider,
        ds::cupertino::Divider,
        ds::fluent::Divider,
        |b, theme| b::new()
            .with_theme(theme)
            .orientation(DividerOrientation::Vertical)
            .divider()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Footer ---------------------------------------------------------------

/// The Footer page.
fn footer_page() -> ComponentPage {
    layout_page(
        "footer",
        "Footer",
        "A bar pinned to the bottom of the screen, with an optional top border.",
        vec![
            Sample {
                title: "Bordered",
                note: None,
                build: footer_bordered,
            },
            Sample {
                title: "Plain",
                note: None,
                build: footer_plain,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A bordered footer with content.
fn footer_bordered() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Footer,
        ds::cupertino::Footer,
        ds::fluent::Footer,
        |b, theme| b::new()
            .with_theme(theme)
            .bordered(true)
            .pad(12.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A plain footer with content.
fn footer_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Footer,
        ds::cupertino::Footer,
        ds::fluent::Footer,
        |b, theme| b::new()
            .with_theme(theme)
            .height(40.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Grid -----------------------------------------------------------------

/// The Grid page.
fn grid_page() -> ComponentPage {
    layout_page(
        "grid",
        "Grid",
        "A fixed-column grid. Its `child` is a factory: the same builder fills \
         every cell, so the grid lays out as many cells as it is given room for.",
        vec![
            Sample {
                title: "Three columns",
                note: None,
                build: grid_three,
            },
            Sample {
                title: "Two columns with a row gap",
                note: None,
                build: grid_two,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method(
                "self -> floom_widgets::Grid<(u64, CellBuilder)>",
                "Grid<(u64, CellBuilder)>",
            ),
            ..Default::default()
        },
    )
}

/// A three-column grid.
fn grid_three() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Grid,
        ds::cupertino::Grid,
        ds::fluent::Grid,
        |b, theme| b::new()
            .with_theme(theme)
            .cols(3)
            .gap(8.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A two-column grid with a row gap.
fn grid_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Grid,
        ds::cupertino::Grid,
        ds::fluent::Grid,
        |b, theme| b::new()
            .with_theme(theme)
            .cols(2)
            .gap(8.0)
            .row_gap(16.0)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Hover ----------------------------------------------------------------

/// The Hover page.
fn hover_page() -> ComponentPage {
    layout_page(
        "hover",
        "Hover",
        "A wrapper that reports when the pointer is over its child, so the child \
         can restyle itself on hover. The samples show the resting state; hover \
         a cell in a live browser to see the highlighted state.",
        vec![
            Sample {
                title: "At rest",
                note: Some("Hover the cell to reveal its highlighted state."),
                build: hover_rest,
            },
            Sample {
                title: "With a handler",
                note: Some("`on_hover_change` fires with the new hovered state."),
                build: hover_handler,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_hover_change",
                "impl Fn(bool) + Send + Sync",
                "bool",
                "Called with the new hovered state when the pointer enters or leaves.",
            )],
            methods: view_method("self -> floom_ui_composites::HoverBox", "HoverBox"),
            ..Default::default()
        },
    )
}

/// A hover wrapper at rest.
fn hover_rest() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Hover,
        ds::cupertino::Hover,
        ds::fluent::Hover,
        |b, theme| b::new()
            .with_theme(theme)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A hover wrapper with a change handler.
fn hover_handler() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Hover,
        ds::cupertino::Hover,
        ds::fluent::Hover,
        |b, theme| b::new()
            .with_theme(theme)
            .on_hover_change(|_state: bool| {})
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Main -----------------------------------------------------------------

/// The Main page.
fn main_page() -> ComponentPage {
    layout_page(
        "main",
        "Main",
        "The scrollable content region of an app, wrapped by `App`. It takes a \
         single child -- the body of the screen -- rather than a builder.",
        vec![
            Sample {
                title: "With content",
                note: Some("Main is a shell: it wraps the screen's body."),
                build: main_content,
            },
            Sample {
                title: "With a panel",
                note: None,
                build: main_panel,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// Main wrapping a line of content.
fn main_content() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Main,
        ds::cupertino::Main,
        ds::fluent::Main,
        |b, theme| b::new()
            .with_theme(theme)
            .child(content("body", "Main content"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// Main wrapping a panel.
fn main_panel() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Main,
        ds::cupertino::Main,
        ds::fluent::Main,
        |b, theme| b::new()
            .with_theme(theme)
            .child(content("body", "A page body"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- NavigationDrawer -----------------------------------------------------

/// The NavigationDrawer page.
fn navigation_drawer_page() -> ComponentPage {
    layout_page(
        "navigation_drawer",
        "NavigationDrawer",
        "A panel that slides in from an edge to hold navigation. It is revealed \
         with `open` (or `open_flag` / `with_open_flag`); the samples show the \
         closed and open states side by side.",
        vec![
            Sample {
                title: "Closed",
                note: Some("Reveal it with `open(true)` or an `open_flag`."),
                build: drawer_closed,
            },
            Sample {
                title: "Open on the right",
                note: None,
                build: drawer_open,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_ui_composites::Overlay", "Overlay"),
            ..Default::default()
        },
    )
}

/// A closed drawer.
fn drawer_closed() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::NavigationDrawer,
        ds::cupertino::NavigationDrawer,
        ds::fluent::NavigationDrawer,
        |b, theme| b::new()
            .with_theme(theme)
            .open(false)
            .header(content("head", "Menu"))
            .child(content("body", "Links"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// An open, right-side drawer.
fn drawer_open() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::NavigationDrawer,
        ds::cupertino::NavigationDrawer,
        ds::fluent::NavigationDrawer,
        |b, theme| b::new()
            .with_theme(theme)
            .open(true)
            .side(NavigationDrawerSide::Right)
            .width(220.0)
            .header(content("head", "Menu"))
            .child(content("body", "Links"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Parallax -------------------------------------------------------------

/// The Parallax page.
fn parallax_page() -> ComponentPage {
    layout_page(
        "parallax",
        "Parallax",
        "A layer that scrolls slower than its container, for depth. It is driven \
         by a scroll signal; the samples show it at scroll offset zero.",
        vec![
            Sample {
                title: "Gentle",
                note: Some("Shown at scroll offset 0; `factor` sets how far it lags."),
                build: parallax_gentle,
            },
            Sample {
                title: "Strong",
                note: None,
                build: parallax_strong,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method(
                "self -> floom_ui_composites::ParallaxLayer",
                "ParallaxLayer",
            ),
            ..Default::default()
        },
    )
}

/// A gentle parallax layer.
fn parallax_gentle() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Parallax,
        ds::cupertino::Parallax,
        ds::fluent::Parallax,
        |b, theme| b::new()
            .with_theme(theme)
            .factor(0.4)
            .child(content("layer", "Background"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A stronger parallax layer.
fn parallax_strong() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Parallax,
        ds::cupertino::Parallax,
        ds::fluent::Parallax,
        |b, theme| b::new()
            .with_theme(theme)
            .factor(0.8)
            .child(content("layer", "Background"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Responsive -----------------------------------------------------------

/// The Responsive page.
fn responsive_page() -> ComponentPage {
    layout_page(
        "responsive",
        "Responsive",
        "A box that keeps an aspect ratio, so its content never stretches. \
         `new` takes the ratio; this page shows a square and a wide box.",
        vec![
            Sample {
                title: "Square",
                note: None,
                build: responsive_square,
            },
            Sample {
                title: "Wide",
                note: Some("A 16:9 box keeps its letterbox shape as the screen changes."),
                build: responsive_wide,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A square responsive box.
fn responsive_square() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Responsive,
        ds::cupertino::Responsive,
        ds::fluent::Responsive,
        |b, theme| b::new(1.0)
            .with_theme(theme)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A wide responsive box.
fn responsive_wide() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Responsive,
        ds::cupertino::Responsive,
        ds::fluent::Responsive,
        |b, theme| b::new(16.0 / 9.0)
            .with_theme(theme)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- SlideGroup -----------------------------------------------------------

/// The SlideGroup page.
fn slide_group_page() -> ComponentPage {
    layout_page(
        "slide_group",
        "SlideGroup",
        "A row of items the reader swipes or arrows through, one shown at a \
         time. Selecting an item reports its index.",
        vec![
            Sample {
                title: "Three items",
                note: Some("`show_arrows` adds the previous and next controls."),
                build: slide_group_three,
            },
            Sample {
                title: "Mandatory, second shown",
                note: Some("`mandatory` stops the group from settling between items."),
                build: slide_group_mandatory,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the shown item's index when the reader moves.",
            )],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// A slide group of three items with arrows.
fn slide_group_three() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SlideGroup,
        ds::cupertino::SlideGroup,
        ds::fluent::SlideGroup,
        |b, theme| b::new()
            .with_theme(theme)
            .show_arrows(true)
            .items(vec![
                content("a", "One"),
                content("b", "Two"),
                content("c", "Three"),
            ])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A mandatory slide group showing the second item.
fn slide_group_mandatory() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SlideGroup,
        ds::cupertino::SlideGroup,
        ds::fluent::SlideGroup,
        |b, theme| b::new()
            .with_theme(theme)
            .mandatory(true)
            .selected(1)
            .items(vec![
                content("a", "One"),
                content("b", "Two"),
                content("c", "Three"),
            ])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Spacer ---------------------------------------------------------------

/// The Spacer page.
fn spacer_page() -> ComponentPage {
    layout_page(
        "spacer",
        "Spacer",
        "Empty space that pushes siblings apart. It paints nothing of its own, \
         so it shows as a gap between other things rather than as a thing.",
        vec![
            Sample {
                title: "Default",
                note: None,
                build: spacer_default,
            },
            Sample {
                title: "Growing",
                note: Some("`grow` lets the spacer claim spare room in its axis."),
                build: spacer_grow,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Spacer", "Spacer"),
            ..Default::default()
        },
    )
}

/// A default spacer.
fn spacer_default() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Spacer,
        ds::cupertino::Spacer,
        ds::fluent::Spacer,
        |b, theme| b::new().with_theme(theme).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A growing spacer.
fn spacer_grow() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Spacer,
        ds::cupertino::Spacer,
        ds::fluent::Spacer,
        |b, theme| b::new().with_theme(theme).grow(1.0).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- SystemBar ------------------------------------------------------------

/// The SystemBar page.
fn system_bar_page() -> ComponentPage {
    layout_page(
        "system_bar",
        "SystemBar",
        "A thin bar at the very top of the screen, holding status like time and \
         signal. It can render light text on a dark fill or dark text on a light \
         one.",
        vec![
            Sample {
                title: "Dark",
                note: None,
                build: system_bar_dark,
            },
            Sample {
                title: "Light",
                note: None,
                build: system_bar_light,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A dark system bar.
fn system_bar_dark() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SystemBar,
        ds::cupertino::SystemBar,
        ds::fluent::SystemBar,
        |b, theme| b::new()
            .with_theme(theme)
            .dark(true)
            .height(28.0)
            .child(content("status", "9:41"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A light system bar.
fn system_bar_light() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SystemBar,
        ds::cupertino::SystemBar,
        ds::fluent::SystemBar,
        |b, theme| b::new()
            .with_theme(theme)
            .dark(false)
            .height(28.0)
            .child(content("status", "9:41"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Tabs -----------------------------------------------------------------

/// The Tabs page.
fn tabs_page() -> ComponentPage {
    layout_page(
        "tabs",
        "Tabs",
        "A row of tab labels that select one panel at a time. Selecting a tab \
         reports its index.",
        vec![
            Sample {
                title: "Three tabs",
                note: None,
                build: tabs_three,
            },
            Sample {
                title: "Two tabs",
                note: None,
                build: tabs_two,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the selected tab's index when the reader picks one.",
            )],
            methods: view_method("self -> floom_ui_composites::TabsBar", "TabsBar"),
            ..Default::default()
        },
    )
}

/// Three tabs, the first selected.
fn tabs_three() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Tabs,
        ds::cupertino::Tabs,
        ds::fluent::Tabs,
        |b, theme| b::new()
            .with_theme(theme)
            .tabs(vec!["One".into(), "Two".into(), "Three".into()])
            .selected(0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// Two tabs, the first selected.
fn tabs_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Tabs,
        ds::cupertino::Tabs,
        ds::fluent::Tabs,
        |b, theme| b::new()
            .with_theme(theme)
            .tabs(vec!["Alpha".into(), "Beta".into()])
            .selected(0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Toolbar --------------------------------------------------------------

/// The Toolbar page.
fn toolbar_page() -> ComponentPage {
    layout_page(
        "toolbar",
        "Toolbar",
        "A horizontal bar of controls -- a leading glyph, a title and a row of \
         actions -- that sits above content.",
        vec![
            Sample {
                title: "With a title",
                note: None,
                build: toolbar_title,
            },
            Sample {
                title: "With actions",
                note: None,
                build: toolbar_actions,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A toolbar with a title.
fn toolbar_title() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Toolbar,
        ds::cupertino::Toolbar,
        ds::fluent::Toolbar,
        |b, theme| b::new()
            .with_theme(theme)
            .title("Editor")
            .leading("menu")
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A toolbar with an action.
fn toolbar_actions() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Toolbar,
        ds::cupertino::Toolbar,
        ds::fluent::Toolbar,
        |b, theme| b::new()
            .with_theme(theme)
            .title("Editor")
            .leading("menu")
            .actions(|| Box::new(floom::floom_widgets::Text::new("Save".to_string())))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Window ---------------------------------------------------------------

/// The Window page.
fn window_page() -> ComponentPage {
    layout_page(
        "window",
        "Window",
        "A carousel of full panels the reader pages through with arrows or dots. \
         Moving between panels reports the shown index.",
        vec![
            Sample {
                title: "Three panes with arrows",
                note: None,
                build: window_arrows,
            },
            Sample {
                title: "With dots",
                note: Some("`show_dots` swaps the arrows for a row of position dots."),
                build: window_dots,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the shown pane's index when the reader pages.",
            )],
            methods: view_method("self -> floom_ui_composites::WindowViewer", "WindowViewer"),
            ..Default::default()
        },
    )
}

/// A window with three panes and arrows.
fn window_arrows() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Window,
        ds::cupertino::Window,
        ds::fluent::Window,
        |b, theme| b::new()
            .with_theme(theme)
            .show_arrows(true)
            .selected(0)
            .items(vec![
                content("a", "Slide 1"),
                content("b", "Slide 2"),
                content("c", "Slide 3"),
            ])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// A window with three panes and dots.
fn window_dots() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Window,
        ds::cupertino::Window,
        ds::fluent::Window,
        |b, theme| b::new()
            .with_theme(theme)
            .show_arrows(false)
            .show_dots(true)
            .items(vec![
                content("a", "Slide 1"),
                content("b", "Slide 2"),
                content("c", "Slide 3"),
            ])
            .view()
    );
    Box::new(compare_row((m, c, f)))
}
