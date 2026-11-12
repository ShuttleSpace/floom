//! The components that move, progress, and provide context to their children.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times. The six
//! providers are service layers with no visual of their own; the four
//! transitions swap a subtree; and the two progress indicators and the skeleton
//! loader draw.

use std::collections::HashMap;

use floom::floom_core::WidgetAny;
use floom::floom_widgets::Text;

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::compare_children;
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, widget};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("date_adapter_provider", "DateAdapterProvider"),
    ("defaults_provider", "DefaultsProvider"),
    ("display_provider", "DisplayProvider"),
    ("expand_transition", "ExpandTransition"),
    ("fade_transition", "FadeTransition"),
    ("icons_provider", "IconsProvider"),
    ("infinite_scroller", "InfiniteScroller"),
    ("locale_provider", "LocaleProvider"),
    ("progress_circular", "ProgressCircular"),
    ("progress_linear", "ProgressLinear"),
    ("pull_to_refresh", "PullToRefresh"),
    ("scale_transition", "ScaleTransition"),
    ("skeleton_loader", "SkeletonLoader"),
    ("slide_transition", "SlideTransition"),
    ("theme_provider", "ThemeProvider"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| match *component {
            "date_adapter_provider" => date_adapter_provider_page(),
            "defaults_provider" => defaults_provider_page(),
            "display_provider" => display_provider_page(),
            "expand_transition" => expand_transition_page(),
            "fade_transition" => fade_transition_page(),
            "icons_provider" => icons_provider_page(),
            "infinite_scroller" => infinite_scroller_page(),
            "locale_provider" => locale_provider_page(),
            "progress_circular" => progress_circular_page(),
            "progress_linear" => progress_linear_page(),
            "pull_to_refresh" => pull_to_refresh_page(),
            "scale_transition" => scale_transition_page(),
            "skeleton_loader" => skeleton_loader_page(),
            "slide_transition" => slide_transition_page(),
            "theme_provider" => theme_provider_page(),
            _ => page(component, name, Category::Motion),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn motion_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::Motion);
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

/// The content shown inside a provider or scroller sample.
fn content_inside() -> Text {
    Text::new("Content".to_string())
}

/// A small map of defaults, for the defaults provider sample.
fn defaults_map() -> HashMap<String, String> {
    HashMap::from([("density".to_string(), "comfortable".to_string())])
}

/// A small map of icons, for the icons provider sample.
fn icons_map() -> HashMap<String, String> {
    HashMap::from([("home".to_string(), "M0 0 L10 10".to_string())])
}

// -- DateAdapterProvider --------------------------------------------------

/// The DateAdapterProvider page.
fn date_adapter_provider_page() -> ComponentPage {
    motion_page(
        "date_adapter_provider",
        "DateAdapterProvider",
        "A service layer that hands a date adapter to every descendant. It draws \
         nothing of its own; the sample shows the content it wraps.",
        vec![
            Sample {
                title: "With a child",
                note: Some("Provides a date adapter to its descendants; no visual of its own."),
                build: date_adapter_provider_child,
            },
            Sample {
                title: "Empty",
                note: Some("Provides a date adapter to its descendants; no visual of its own."),
                build: date_adapter_provider_empty,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The date adapter provider, with a child.
fn date_adapter_provider_child() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DateAdapterProvider,
        ds::cupertino::DateAdapterProvider,
        ds::fluent::DateAdapterProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .adapter("default")
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The date adapter provider, with no child.
fn date_adapter_provider_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DateAdapterProvider,
        ds::cupertino::DateAdapterProvider,
        ds::fluent::DateAdapterProvider,
        |b, theme| b::new().with_theme(theme).adapter("default").view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- DefaultsProvider -----------------------------------------------------

/// The DefaultsProvider page.
fn defaults_provider_page() -> ComponentPage {
    motion_page(
        "defaults_provider",
        "DefaultsProvider",
        "A service layer that overrides component defaults for every \
         descendant. It draws nothing of its own; the sample shows the content \
         it wraps.",
        vec![
            Sample {
                title: "With a child",
                note: Some("Overrides defaults for its descendants; no visual of its own."),
                build: defaults_provider_child,
            },
            Sample {
                title: "Empty",
                note: Some("Overrides defaults for its descendants; no visual of its own."),
                build: defaults_provider_empty,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The defaults provider, with a child.
fn defaults_provider_child() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DefaultsProvider,
        ds::cupertino::DefaultsProvider,
        ds::fluent::DefaultsProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .defaults(defaults_map())
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The defaults provider, with no child.
fn defaults_provider_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DefaultsProvider,
        ds::cupertino::DefaultsProvider,
        ds::fluent::DefaultsProvider,
        |b, theme| b::new().with_theme(theme).defaults(defaults_map()).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- DisplayProvider ------------------------------------------------------

/// The DisplayProvider page.
fn display_provider_page() -> ComponentPage {
    motion_page(
        "display_provider",
        "DisplayProvider",
        "A service layer that sets the display density (comfortable or dense) for \
         every descendant. It draws nothing of its own; the sample shows the \
         content it wraps.",
        vec![
            Sample {
                title: "Comfortable",
                note: Some("Sets display density for its descendants; no visual of its own."),
                build: display_provider_comfortable,
            },
            Sample {
                title: "Dense",
                note: Some("Sets display density for its descendants; no visual of its own."),
                build: display_provider_dense,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The display provider, comfortable.
fn display_provider_comfortable() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DisplayProvider,
        ds::cupertino::DisplayProvider,
        ds::fluent::DisplayProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .dense(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The display provider, dense.
fn display_provider_dense() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DisplayProvider,
        ds::cupertino::DisplayProvider,
        ds::fluent::DisplayProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .dense(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- ExpandTransition -----------------------------------------------------

/// The ExpandTransition page.
fn expand_transition_page() -> ComponentPage {
    motion_page(
        "expand_transition",
        "ExpandTransition",
        "A transition that grows and shrinks its child in place, for disclosure \
         and expansion. The sample sets `show` so the child is present.",
        vec![
            Sample {
                title: "Shown",
                note: Some("With `show(true)` the child is present; `show(false)` collapses it."),
                build: expand_transition_shown,
            },
            Sample {
                title: "Hidden",
                note: Some("With `show(false)` the child is absent until `show` flips."),
                build: expand_transition_hidden,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Conditional", "Conditional"),
            ..Default::default()
        },
    )
}

/// The expand transition, shown.
fn expand_transition_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ExpandTransition,
        ds::cupertino::ExpandTransition,
        ds::fluent::ExpandTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The expand transition, hidden.
fn expand_transition_hidden() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ExpandTransition,
        ds::cupertino::ExpandTransition,
        ds::fluent::ExpandTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- FadeTransition -------------------------------------------------------

/// The FadeTransition page.
fn fade_transition_page() -> ComponentPage {
    motion_page(
        "fade_transition",
        "FadeTransition",
        "A transition that fades its child in and out, for appearance and \
         disappearance. The sample sets `show` so the child is present.",
        vec![
            Sample {
                title: "Shown",
                note: Some("With `show(true)` the child is present; `show(false)` fades it out."),
                build: fade_transition_shown,
            },
            Sample {
                title: "Hidden",
                note: Some("With `show(false)` the child is absent until `show` flips."),
                build: fade_transition_hidden,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Conditional", "Conditional"),
            ..Default::default()
        },
    )
}

/// The fade transition, shown.
fn fade_transition_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::FadeTransition,
        ds::cupertino::FadeTransition,
        ds::fluent::FadeTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The fade transition, hidden.
fn fade_transition_hidden() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::FadeTransition,
        ds::cupertino::FadeTransition,
        ds::fluent::FadeTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- IconsProvider --------------------------------------------------------

/// The IconsProvider page.
fn icons_provider_page() -> ComponentPage {
    motion_page(
        "icons_provider",
        "IconsProvider",
        "A service layer that supplies icon paths to every descendant. It draws \
         nothing of its own; the sample shows the content it wraps.",
        vec![
            Sample {
                title: "With a child",
                note: Some("Supplies icon paths to its descendants; no visual of its own."),
                build: icons_provider_child,
            },
            Sample {
                title: "Empty",
                note: Some("Supplies icon paths to its descendants; no visual of its own."),
                build: icons_provider_empty,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The icons provider, with a child.
fn icons_provider_child() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::IconsProvider,
        ds::cupertino::IconsProvider,
        ds::fluent::IconsProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .icons(icons_map())
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The icons provider, with no child.
fn icons_provider_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::IconsProvider,
        ds::cupertino::IconsProvider,
        ds::fluent::IconsProvider,
        |b, theme| b::new().with_theme(theme).icons(icons_map()).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- InfiniteScroller -----------------------------------------------------

/// The InfiniteScroller page.
fn infinite_scroller_page() -> ComponentPage {
    motion_page(
        "infinite_scroller",
        "InfiniteScroller",
        "A scrolling surface that asks for more content as the reader nears the \
         bottom. The sample shows the resting state; scroll to the end to trigger \
         `on_load_more`.",
        vec![
            Sample {
                title: "Resting",
                note: Some("Scroll to the bottom to load more; shown at rest."),
                build: infinite_scroller_resting,
            },
            Sample {
                title: "Loading",
                note: Some("`loading(true)` shows the pending spinner."),
                build: infinite_scroller_loading,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_load_more",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader scrolls near the bottom and more is wanted.",
            )],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// The infinite scroller, at rest.
fn infinite_scroller_resting() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::InfiniteScroller,
        ds::cupertino::InfiniteScroller,
        ds::fluent::InfiniteScroller,
        |b, theme| b::new()
            .with_theme(theme)
            .viewport(160.0)
            .child(content("body", "Scrolling content"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The infinite scroller, loading.
fn infinite_scroller_loading() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::InfiniteScroller,
        ds::cupertino::InfiniteScroller,
        ds::fluent::InfiniteScroller,
        |b, theme| b::new()
            .with_theme(theme)
            .viewport(160.0)
            .loading(true)
            .child(content("body", "Scrolling content"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- LocaleProvider -------------------------------------------------------

/// The LocaleProvider page.
fn locale_provider_page() -> ComponentPage {
    motion_page(
        "locale_provider",
        "LocaleProvider",
        "A service layer that sets the locale for every descendant. It draws \
         nothing of its own; the sample shows the content it wraps.",
        vec![
            Sample {
                title: "With a child",
                note: Some("Sets the locale for its descendants; no visual of its own."),
                build: locale_provider_child,
            },
            Sample {
                title: "Empty",
                note: Some("Sets the locale for its descendants; no visual of its own."),
                build: locale_provider_empty,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The locale provider, with a child.
fn locale_provider_child() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::LocaleProvider,
        ds::cupertino::LocaleProvider,
        ds::fluent::LocaleProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .locale("en-US")
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The locale provider, with no child.
fn locale_provider_empty() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::LocaleProvider,
        ds::cupertino::LocaleProvider,
        ds::fluent::LocaleProvider,
        |b, theme| b::new().with_theme(theme).locale("en-US").view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- ProgressCircular -----------------------------------------------------

/// The ProgressCircular page.
fn progress_circular_page() -> ComponentPage {
    motion_page(
        "progress_circular",
        "ProgressCircular",
        "A circular progress indicator, either a determinate fraction or an \
         indeterminate spin.",
        vec![
            Sample {
                title: "Determinate",
                note: None,
                build: progress_circular_determinate,
            },
            Sample {
                title: "Indeterminate",
                note: None,
                build: progress_circular_indeterminate,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method(
                "self -> floom_widgets::CircularProgress",
                "CircularProgress",
            ),
            ..Default::default()
        },
    )
}

/// A determinate circular progress.
fn progress_circular_determinate() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ProgressCircular,
        ds::cupertino::ProgressCircular,
        ds::fluent::ProgressCircular,
        |b, theme| b::new().with_theme(theme).value(0.6).size(36.0).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// An indeterminate circular progress.
fn progress_circular_indeterminate() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ProgressCircular,
        ds::cupertino::ProgressCircular,
        ds::fluent::ProgressCircular,
        |b, theme| b::new()
            .with_theme(theme)
            .indeterminate(true)
            .size(36.0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- ProgressLinear -------------------------------------------------------

/// The ProgressLinear page.
fn progress_linear_page() -> ComponentPage {
    motion_page(
        "progress_linear",
        "ProgressLinear",
        "A linear progress indicator, either a determinate fraction or an \
         indeterminate bar.",
        vec![
            Sample {
                title: "Determinate",
                note: None,
                build: progress_linear_determinate,
            },
            Sample {
                title: "Indeterminate",
                note: None,
                build: progress_linear_indeterminate,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::LinearProgress", "LinearProgress"),
            ..Default::default()
        },
    )
}

/// A determinate linear progress.
fn progress_linear_determinate() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ProgressLinear,
        ds::cupertino::ProgressLinear,
        ds::fluent::ProgressLinear,
        |b, theme| b::new().with_theme(theme).value(0.6).height(4.0).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// An indeterminate linear progress.
fn progress_linear_indeterminate() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ProgressLinear,
        ds::cupertino::ProgressLinear,
        ds::fluent::ProgressLinear,
        |b, theme| b::new()
            .with_theme(theme)
            .indeterminate(true)
            .height(4.0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- PullToRefresh --------------------------------------------------------

/// The PullToRefresh page.
fn pull_to_refresh_page() -> ComponentPage {
    motion_page(
        "pull_to_refresh",
        "PullToRefresh",
        "A scrolling surface that reloads when the reader pulls past the top. The \
         sample shows the resting state; pull down to trigger `on_refresh`.",
        vec![
            Sample {
                title: "Resting",
                note: Some("Pull down past the threshold to refresh; shown at rest."),
                build: pull_to_refresh_resting,
            },
            Sample {
                title: "Refreshing",
                note: Some("`loading(true)` shows the refreshing spinner."),
                build: pull_to_refresh_loading,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_refresh",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader pulls past the threshold to refresh.",
            )],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// The pull-to-refresh, at rest.
fn pull_to_refresh_resting() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::PullToRefresh,
        ds::cupertino::PullToRefresh,
        ds::fluent::PullToRefresh,
        |b, theme| b::new()
            .with_theme(theme)
            .child(content("body", "List content"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The pull-to-refresh, refreshing.
fn pull_to_refresh_loading() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::PullToRefresh,
        ds::cupertino::PullToRefresh,
        ds::fluent::PullToRefresh,
        |b, theme| b::new()
            .with_theme(theme)
            .loading(true)
            .child(content("body", "List content"))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- ScaleTransition ------------------------------------------------------

/// The ScaleTransition page.
fn scale_transition_page() -> ComponentPage {
    motion_page(
        "scale_transition",
        "ScaleTransition",
        "A transition that scales its child up and down, for entrance and exit. \
         The sample sets `show` so the child is present.",
        vec![
            Sample {
                title: "Shown",
                note: Some("With `show(true)` the child is present; `show(false)` scales it away."),
                build: scale_transition_shown,
            },
            Sample {
                title: "Hidden",
                note: Some("With `show(false)` the child is absent until `show` flips."),
                build: scale_transition_hidden,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Conditional", "Conditional"),
            ..Default::default()
        },
    )
}

/// The scale transition, shown.
fn scale_transition_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ScaleTransition,
        ds::cupertino::ScaleTransition,
        ds::fluent::ScaleTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The scale transition, hidden.
fn scale_transition_hidden() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ScaleTransition,
        ds::cupertino::ScaleTransition,
        ds::fluent::ScaleTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- SkeletonLoader -------------------------------------------------------

/// The SkeletonLoader page.
fn skeleton_loader_page() -> ComponentPage {
    motion_page(
        "skeleton_loader",
        "SkeletonLoader",
        "A placeholder that mimics the shape of loading content, so the layout \
         does not jump when the real content arrives.",
        vec![
            Sample {
                title: "Three lines",
                note: None,
                build: skeleton_loader_lines,
            },
            Sample {
                title: "Animated",
                note: Some("`animated(true)` pulses the placeholder."),
                build: skeleton_loader_animated,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Surface", "Surface"),
            ..Default::default()
        },
    )
}

/// A skeleton of three lines.
fn skeleton_loader_lines() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SkeletonLoader,
        ds::cupertino::SkeletonLoader,
        ds::fluent::SkeletonLoader,
        |b, theme| b::new()
            .with_theme(theme)
            .lines(3)
            .width(220.0)
            .height(16.0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// An animated skeleton of two lines.
fn skeleton_loader_animated() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SkeletonLoader,
        ds::cupertino::SkeletonLoader,
        ds::fluent::SkeletonLoader,
        |b, theme| b::new()
            .with_theme(theme)
            .animated(true)
            .lines(2)
            .width(220.0)
            .height(16.0)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- SlideTransition ------------------------------------------------------

/// The SlideTransition page.
fn slide_transition_page() -> ComponentPage {
    motion_page(
        "slide_transition",
        "SlideTransition",
        "A transition that slides its child in from an edge and out to one, for \
         panels and sheets. The sample sets `show` so the child is present.",
        vec![
            Sample {
                title: "Shown",
                note: Some("With `show(true)` the child is present; `show(false)` slides it out."),
                build: slide_transition_shown,
            },
            Sample {
                title: "Hidden",
                note: Some("With `show(false)` the child is absent until `show` flips."),
                build: slide_transition_hidden,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Conditional", "Conditional"),
            ..Default::default()
        },
    )
}

/// The slide transition, shown.
fn slide_transition_shown() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SlideTransition,
        ds::cupertino::SlideTransition,
        ds::fluent::SlideTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The slide transition, hidden.
fn slide_transition_hidden() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::SlideTransition,
        ds::cupertino::SlideTransition,
        ds::fluent::SlideTransition,
        |b, theme| b::new()
            .with_theme(theme)
            .show(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- ThemeProvider --------------------------------------------------------

/// The ThemeProvider page.
fn theme_provider_page() -> ComponentPage {
    motion_page(
        "theme_provider",
        "ThemeProvider",
        "A service layer that sets the dark mode for every descendant. It draws \
         nothing of its own; the sample shows the content it wraps.",
        vec![
            Sample {
                title: "Light",
                note: Some("Sets dark mode for its descendants; no visual of its own."),
                build: theme_provider_light,
            },
            Sample {
                title: "Dark",
                note: Some("Sets dark mode for its descendants; no visual of its own."),
                build: theme_provider_dark,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: view_method("self -> floom_widgets::Stack", "Stack"),
            ..Default::default()
        },
    )
}

/// The theme provider, light.
fn theme_provider_light() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ThemeProvider,
        ds::cupertino::ThemeProvider,
        ds::fluent::ThemeProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .dark(false)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The theme provider, dark.
fn theme_provider_dark() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ThemeProvider,
        ds::cupertino::ThemeProvider,
        ds::fluent::ThemeProvider,
        |b, theme| b::new()
            .with_theme(theme)
            .dark(true)
            .child(|| Box::new(content_inside()))
            .view()
    );
    Box::new(compare_row((m, c, f)))
}
