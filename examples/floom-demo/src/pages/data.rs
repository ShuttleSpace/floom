//! The components a reader reads.
//!
//! Every component here exists in all three design systems with the same
//! builder methods, so a sample is written once and shown three times.

use std::sync::Arc;

use floom::floom_core::{Axis, WidgetAny};
use floom::floom_types::FontWeight;
use floom::floom_widgets::{Stack, Text};
use floom_core_theme::props::datatable::DataTableColumn;
use floom_core_theme::props::menu::MenuItem;
use floom_core_theme::props::timeline::TimelineItem;
use floom_core_theme::props::treeview::TreeviewNode;
use floom_ui_composites::{CalendarDate, WidgetBuilder};

use crate::api::{ApiDoc, ApiRow};
use crate::ds;
use crate::macros::{compare_children, themed_widget};
use crate::pages::{Category, ComponentPage, Sample, page};
use crate::widgets::{compare_row, label, widget};

/// This category's components, as the key the satellite names them after and
/// the name a reader sees.
const COMPONENTS: &[(&str, &str)] = &[
    ("avatar", "Avatar"),
    ("badge", "Badge"),
    ("calendar", "Calendar"),
    ("card", "Card"),
    ("carousel", "Carousel"),
    ("chip", "Chip"),
    ("code", "Code"),
    ("data_iterator", "DataIterator"),
    ("datatable", "DataTable"),
    ("icon", "Icon"),
    ("image", "Image"),
    ("kbd", "Kbd"),
    ("list", "List"),
    ("list_item", "ListItem"),
    ("pagination", "Pagination"),
    ("sparkline", "Sparkline"),
    ("table", "Table"),
    ("text", "Text"),
    ("timeline", "Timeline"),
    ("treeview", "Treeview"),
    ("virtual_scroller", "VirtualScroller"),
];

/// This category's pages.
pub fn pages() -> Vec<ComponentPage> {
    COMPONENTS
        .iter()
        .map(|(component, name)| match *component {
            "avatar" => avatar_page(),
            "badge" => badge_page(),
            "calendar" => calendar_page(),
            "card" => card_page(),
            "carousel" => carousel_page(),
            "chip" => chip_page(),
            "code" => code_page(),
            "data_iterator" => data_iterator_page(),
            "datatable" => data_table_page(),
            "icon" => icon_page(),
            "image" => image_page(),
            "kbd" => kbd_page(),
            "list" => list_page(),
            "list_item" => list_item_page(),
            "pagination" => pagination_page(),
            "sparkline" => sparkline_page(),
            "table" => table_page(),
            "text" => text_page(),
            "timeline" => timeline_page(),
            "treeview" => treeview_page(),
            "virtual_scroller" => virtual_scroller_page(),
            _ => page(component, name, Category::DataDisplay),
        })
        .collect()
}

/// A page in this category: an introduction, its samples and its API.
fn data_page(
    component: &'static str,
    name: &'static str,
    intro: &'static str,
    samples: Vec<Sample>,
    api: ApiDoc,
) -> ComponentPage {
    let mut page = page(component, name, Category::DataDisplay);
    page.intro = intro;
    page.samples = samples;
    page.api = api;
    page
}

// -- Avatar ---------------------------------------------------------------

/// The Avatar page.
fn avatar_page() -> ComponentPage {
    data_page(
        "avatar",
        "Avatar",
        "A circle with a person's initials in it.",
        vec![
            Sample {
                title: "With initials",
                note: None,
                build: avatar_initials,
            },
            Sample {
                title: "Larger",
                note: Some("The size is the circle's edge length."),
                build: avatar_large,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The avatar, with initials.
fn avatar_initials() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Avatar,
        ds::cupertino::Avatar,
        ds::fluent::Avatar,
        |b, theme| b::new().with_theme(theme).content("RF").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The avatar, larger.
fn avatar_large() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Avatar,
        ds::cupertino::Avatar,
        ds::fluent::Avatar,
        |b, theme| b::new().with_theme(theme).content("RF").size(56.0).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Badge ----------------------------------------------------------------

/// The Badge page.
fn badge_page() -> ComponentPage {
    data_page(
        "badge",
        "Badge",
        "A small count, pinned to something else.",
        vec![
            Sample {
                title: "With a count",
                note: None,
                build: badge_count,
            },
            Sample {
                title: "A dot",
                note: Some("A dot says 'something changed' without saying how much."),
                build: badge_dot,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Badge",
                "Badge",
                "Build the engine badge this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The badge, with a count.
fn badge_count() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Badge,
        ds::cupertino::Badge,
        ds::fluent::Badge,
        |b, theme| b::new().with_theme(theme).content("7").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The badge, as a dot.
fn badge_dot() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Badge,
        ds::cupertino::Badge,
        ds::fluent::Badge,
        |b, theme| b::new().with_theme(theme).dot(true).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Calendar -------------------------------------------------------------

/// The Calendar page.
fn calendar_page() -> ComponentPage {
    data_page(
        "calendar",
        "Calendar",
        "A month, with its days.",
        vec![
            Sample {
                title: "A month",
                note: None,
                build: calendar_month,
            },
            Sample {
                title: "With week numbers",
                note: Some("The week number is the week of the year, counted from the first."),
                build: calendar_weeks,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(CalendarDate) + Send + Sync",
                "CalendarDate",
                "Called with the chosen date when the reader picks a day.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "value",
                    "mut self -> Self",
                    "CalendarDate",
                    "The date the calendar is showing.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The calendar, showing a month.
fn calendar_month() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Calendar,
        ds::cupertino::Calendar,
        ds::fluent::Calendar,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(CalendarDate {
                    year: 2026,
                    month: 11,
                    day: 1,
                })
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The calendar, with week numbers.
fn calendar_weeks() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Calendar,
        ds::cupertino::Calendar,
        ds::fluent::Calendar,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .value(CalendarDate {
                    year: 2026,
                    month: 11,
                    day: 1,
                })
                .show_week_numbers(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Card -----------------------------------------------------------------

/// The Card page.
fn card_page() -> ComponentPage {
    data_page(
        "card",
        "Card",
        "A raised surface that groups one thing with its details.",
        vec![
            Sample {
                title: "Flat",
                note: None,
                build: card_flat,
            },
            Sample {
                title: "Raised",
                note: Some("The elevation is how far the card lifts off the page."),
                build: card_raised,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![
                ApiRow::new(
                    "surface",
                    "&self -> floom_widgets::Surface",
                    "Surface",
                    "The card's surface; attach content to it.",
                ),
                ApiRow::new(
                    "padding",
                    "&self -> Insets",
                    "Insets",
                    "The padding between the card's edge and its content.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The card, flat.
fn card_flat() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Card,
        ds::cupertino::Card,
        ds::fluent::Card,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .surface()
                .push(|| Box::new(Text::new("A card")))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The card, raised.
fn card_raised() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Card,
        ds::cupertino::Card,
        ds::fluent::Card,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .elevation(3)
                .surface()
                .push(|| Box::new(Text::new("A card")))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Carousel -------------------------------------------------------------

/// The Carousel page.
fn carousel_page() -> ComponentPage {
    data_page(
        "carousel",
        "Carousel",
        "A row of slides the reader moves through.",
        vec![
            Sample {
                title: "Three slides",
                note: None,
                build: carousel_slides,
            },
            Sample {
                title: "With dots and arrows",
                note: Some("The dots are the pages; the arrows move between them."),
                build: carousel_chrome,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the new slide's index when it changes.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "index",
                    "&self -> usize",
                    "0",
                    "The slide on show right now.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The carousel, with three slides.
fn carousel_slides() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Carousel,
        ds::cupertino::Carousel,
        ds::fluent::Carousel,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .slides(vec![
                    label(
                        "one",
                        "First",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                    label(
                        "two",
                        "Second",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                    label(
                        "three",
                        "Third",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The carousel, with its dots and arrows.
fn carousel_chrome() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Carousel,
        ds::cupertino::Carousel,
        ds::fluent::Carousel,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .slides(vec![
                    label(
                        "one",
                        "First",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                    label(
                        "two",
                        "Second",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                    label(
                        "three",
                        "Third",
                        13.0,
                        FontWeight::Normal,
                        crate::widgets::INK,
                    ),
                ])
                .height(160.0)
                .show_dots(true)
                .show_arrows(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Chip -----------------------------------------------------------------

/// The Chip page.
fn chip_page() -> ComponentPage {
    data_page(
        "chip",
        "Chip",
        "A small rounded label: a tag, a filter, a choice.",
        vec![
            Sample {
                title: "Plain and selected",
                note: None,
                build: chip_states,
            },
            Sample {
                title: "Disabled",
                note: Some("A disabled chip keeps its label; it just stops responding."),
                build: chip_disabled,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + 'static",
                "()",
                "Called when the chip is pressed.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::Chip",
                "Chip",
                "Build the engine chip this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The chip, plain and selected.
fn chip_states() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Chip,
        ds::cupertino::Chip,
        ds::fluent::Chip,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "plain",
                    themed_widget!(b, theme, |c| c.label("Rust").view()),
                ))
                .child(widget(
                    "selected",
                    themed_widget!(b, theme, |c| c.label("Rust").selected(true).view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The chip, disabled.
fn chip_disabled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Chip,
        ds::cupertino::Chip,
        ds::fluent::Chip,
        |b, theme| {
            Stack::new(Axis::Vertical)
                .gap(10.0)
                .child(widget(
                    "plain",
                    themed_widget!(b, theme, |c| c.label("Rust").disabled(true).view()),
                ))
                .child(widget(
                    "selected",
                    themed_widget!(b, theme, |c| c
                        .label("Rust")
                        .selected(true)
                        .disabled(true)
                        .view()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Code -----------------------------------------------------------------

/// The Code page.
fn code_page() -> ComponentPage {
    data_page(
        "code",
        "Code",
        "A snippet of code, set in a monospaced face.",
        vec![
            Sample {
                title: "A snippet",
                note: None,
                build: code_snippet,
            },
            Sample {
                title: "Not monospaced",
                note: Some("Monospaced is the default; the switch turns it off."),
                build: code_proportional,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The code snippet, monospaced.
fn code_snippet() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Code,
        ds::cupertino::Code,
        ds::fluent::Code,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .content("let answer = 42;")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The code snippet, in the reader's own face.
fn code_proportional() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Code,
        ds::cupertino::Code,
        ds::fluent::Code,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .content("let answer = 42;")
                .mono(false)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- DataIterator ---------------------------------------------------------

/// The DataIterator page.
fn data_iterator_page() -> ComponentPage {
    data_page(
        "data_iterator",
        "DataIterator",
        "A list of rows with a header and a footer around them.",
        vec![
            Sample {
                title: "Rows with a header",
                note: None,
                build: data_iterator_header,
            },
            Sample {
                title: "Divided, with a footer",
                note: Some("Divided puts a rule between the rows."),
                build: data_iterator_footer,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The data iterator, with a header.
fn data_iterator_header() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DataIterator,
        ds::cupertino::DataIterator,
        ds::fluent::DataIterator,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .header(|| Box::new(Text::new("Recent")))
                .item(|| Box::new(Text::new("One")))
                .item(|| Box::new(Text::new("Two")))
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The data iterator, divided, with a footer.
fn data_iterator_footer() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::DataIterator,
        ds::cupertino::DataIterator,
        ds::fluent::DataIterator,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .header(|| Box::new(Text::new("Recent")))
                .item(|| Box::new(Text::new("One")))
                .item(|| Box::new(Text::new("Two")))
                .footer(|| Box::new(Text::new("3 items")))
                .divided(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- DataTable ------------------------------------------------------------

/// The DataTable page.
fn data_table_page() -> ComponentPage {
    data_page(
        "datatable",
        "DataTable",
        "A table of rows and columns, with a header row.",
        vec![
            Sample {
                title: "Three columns",
                note: None,
                build: data_table_plain,
            },
            Sample {
                title: "Striped and dense",
                note: Some("Striped tints alternate rows; dense packs them closer."),
                build: data_table_striped,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_row_click",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the row's index when a row is clicked.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The data table, plain.
fn data_table_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Datatable,
        ds::cupertino::Datatable,
        ds::fluent::Datatable,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .columns(vec![
                    DataTableColumn {
                        title: "Component".to_string(),
                        ..Default::default()
                    },
                    DataTableColumn {
                        title: "Design systems".to_string(),
                        ..Default::default()
                    },
                ])
                .rows(vec![
                    vec!["Button".to_string(), "3".to_string()],
                    vec!["Checkbox".to_string(), "3".to_string()],
                    vec!["Counter".to_string(), "3".to_string()],
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The data table, striped and dense.
fn data_table_striped() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Datatable,
        ds::cupertino::Datatable,
        ds::fluent::Datatable,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .columns(vec![
                    DataTableColumn {
                        title: "Component".to_string(),
                        ..Default::default()
                    },
                    DataTableColumn {
                        title: "Design systems".to_string(),
                        ..Default::default()
                    },
                ])
                .rows(vec![
                    vec!["Button".to_string(), "3".to_string()],
                    vec!["Checkbox".to_string(), "3".to_string()],
                    vec!["Counter".to_string(), "3".to_string()],
                ])
                .striped(true)
                .dense(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Icon -----------------------------------------------------------------

/// The Icon page.
fn icon_page() -> ComponentPage {
    data_page(
        "icon",
        "Icon",
        "A glyph: a picture of one thing, drawn rather than written.",
        vec![
            Sample {
                title: "Some glyphs",
                note: None,
                build: icon_glyphs,
            },
            Sample {
                title: "Larger, and named",
                note: Some("A labelled icon has an accessible name; a decorative one does not."),
                build: icon_labelled,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![
                ApiRow::new(
                    "resolve",
                    "&self -> floom_widgets::VectorIcon",
                    "VectorIcon",
                    "Build the engine vector icon this component renders.",
                ),
                ApiRow::new(
                    "path_data",
                    "&self -> &'static str",
                    "\"\"",
                    "The glyph's path data.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The icons, a few of them.
fn icon_glyphs() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(16.0)
                .child(widget(
                    "home",
                    themed_widget!(b, theme, "home", |c| c.resolve()),
                ))
                .child(widget(
                    "search",
                    themed_widget!(b, theme, "search", |c| c.resolve()),
                ))
                .child(widget(
                    "settings",
                    themed_widget!(b, theme, "settings", |c| c.resolve()),
                ))
                .child(widget(
                    "star",
                    themed_widget!(b, theme, "star", |c| c.resolve()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The icons, larger and named.
fn icon_labelled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Icon,
        ds::cupertino::Icon,
        ds::fluent::Icon,
        |b, theme| {
            Stack::new(Axis::Horizontal)
                .gap(16.0)
                .child(widget(
                    "named",
                    themed_widget!(b, theme, "star", |c| c
                        .size(32.0)
                        .label("Favourite")
                        .resolve()),
                ))
                .child(widget(
                    "decorative",
                    themed_widget!(b, theme, "star", |c| c
                        .size(32.0)
                        .decorative(true)
                        .resolve()),
                ))
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Image ----------------------------------------------------------------

/// The Image page.
fn image_page() -> ComponentPage {
    data_page(
        "image",
        "Image",
        "A picture.",
        vec![
            Sample {
                title: "A source",
                note: Some(
                    "The source is a path the host resolves; the alt text is the picture's name.",
                ),
                build: image_source,
            },
            Sample {
                title: "Sized and rounded",
                note: None,
                build: image_sized,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The image, with a source.
fn image_source() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Image,
        ds::cupertino::Image,
        ds::fluent::Image,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .src("logo.png")
                .alt("The Floom logo")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The image, sized and rounded.
fn image_sized() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Image,
        ds::cupertino::Image,
        ds::fluent::Image,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .src("logo.png")
                .alt("The Floom logo")
                .width(120.0)
                .height(80.0)
                .rounded(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Kbd ------------------------------------------------------------------

/// The Kbd page.
fn kbd_page() -> ComponentPage {
    data_page(
        "kbd",
        "Kbd",
        "A key on a keyboard, set off from the prose around it.",
        vec![
            Sample {
                title: "A key",
                note: None,
                build: kbd_key,
            },
            Sample {
                title: "Not monospaced",
                note: Some("Monospaced is the default; the switch turns it off."),
                build: kbd_proportional,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The key.
fn kbd_key() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Kbd,
        ds::cupertino::Kbd,
        ds::fluent::Kbd,
        |b, theme| b::new().with_theme(theme).content("Ctrl").view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The key, in the reader's own face.
fn kbd_proportional() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Kbd,
        ds::cupertino::Kbd,
        ds::fluent::Kbd,
        |b, theme| b::new()
            .with_theme(theme)
            .content("Ctrl")
            .mono(false)
            .view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- List -----------------------------------------------------------------

/// The List page.
fn list_page() -> ComponentPage {
    data_page(
        "list",
        "List",
        "A row of choices, each one a thing the reader can pick.",
        vec![
            Sample {
                title: "Two items",
                note: None,
                build: list_two,
            },
            Sample {
                title: "Built from a table",
                note: Some("A list can be built from items rather than one at a time."),
                build: list_from_items,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_select",
                "impl Fn() + Send + Sync",
                "()",
                "Called when an item is picked.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The list, with two items.
fn list_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::List,
        ds::cupertino::List,
        ds::fluent::List,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .item("Rust", || {})
                .item("Floom", || {})
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The list, built from a table of items.
fn list_from_items() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::List,
        ds::cupertino::List,
        ds::fluent::List,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .items(vec![
                    MenuItem {
                        label: "Rust".to_string(),
                        on_select: None,
                    },
                    MenuItem {
                        label: "Floom".to_string(),
                        on_select: None,
                    },
                    MenuItem {
                        label: "Widgets".to_string(),
                        on_select: None,
                    },
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- ListItem -------------------------------------------------------------

/// The ListItem page.
fn list_item_page() -> ComponentPage {
    data_page(
        "list_item",
        "ListItem",
        "One row of a list: a title, and whatever else it needs.",
        vec![
            Sample {
                title: "A title and a subtitle",
                note: None,
                build: list_item_plain,
            },
            Sample {
                title: "With a leading icon, and disabled",
                note: Some("A leading icon sits before the title."),
                build: list_item_leading,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_click",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the row is pressed.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_ui_composites::ListItemRow",
                "ListItemRow",
                "Build the engine row this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The list item, with a title and a subtitle.
fn list_item_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListItem,
        ds::cupertino::ListItem,
        ds::fluent::ListItem,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Settings")
                .subtitle("Everything about this device")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The list item, with a leading icon and disabled.
fn list_item_leading() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::ListItem,
        ds::cupertino::ListItem,
        ds::fluent::ListItem,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .title("Unavailable")
                .subtitle("Not right now")
                .leading(|| Box::new(Text::new("*")))
                .disabled(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Pagination -----------------------------------------------------------

/// The Pagination page.
fn pagination_page() -> ComponentPage {
    data_page(
        "pagination",
        "Pagination",
        "The pages of a long list, and the one the reader is on.",
        vec![
            Sample {
                title: "Five pages",
                note: None,
                build: pagination_five,
            },
            Sample {
                title: "On the third page",
                note: Some("The current page is the one the reader is on."),
                build: pagination_third,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_change",
                "impl Fn(usize) + Send + Sync",
                "usize",
                "Called with the new page's index when it changes.",
            )],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The pagination, over five pages.
fn pagination_five() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Pagination,
        ds::cupertino::Pagination,
        ds::fluent::Pagination,
        |b, theme| b::new().with_theme(theme).total(5).current(0).view()
    );
    Box::new(compare_row((m, c, f)))
}

/// The pagination, on the third page.
fn pagination_third() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Pagination,
        ds::cupertino::Pagination,
        ds::fluent::Pagination,
        |b, theme| b::new().with_theme(theme).total(8).current(2).view()
    );
    Box::new(compare_row((m, c, f)))
}

// -- Sparkline ------------------------------------------------------------

/// The Sparkline page.
fn sparkline_page() -> ComponentPage {
    data_page(
        "sparkline",
        "Sparkline",
        "A chart with no axes: the shape of a series, at a glance.",
        vec![
            Sample {
                title: "A line",
                note: None,
                build: sparkline_line,
            },
            Sample {
                title: "Filled, and wider",
                note: Some("A filled sparkline tints the area under the line."),
                build: sparkline_filled,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The sparkline, as a line.
fn sparkline_line() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Sparkline,
        ds::cupertino::Sparkline,
        ds::fluent::Sparkline,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .values(vec![4.0, 7.0, 3.0, 9.0, 6.0, 11.0, 8.0])
                .width(140.0)
                .height(48.0)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The sparkline, filled.
fn sparkline_filled() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Sparkline,
        ds::cupertino::Sparkline,
        ds::fluent::Sparkline,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .values(vec![4.0, 7.0, 3.0, 9.0, 6.0, 11.0, 8.0])
                .width(200.0)
                .height(64.0)
                .line_width(2.0)
                .fill(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Table ----------------------------------------------------------------

/// The Table page.
fn table_page() -> ComponentPage {
    data_page(
        "table",
        "Table",
        "Rows and columns of plain text.",
        vec![
            Sample {
                title: "Headers and rows",
                note: None,
                build: table_plain,
            },
            Sample {
                title: "Striped and dense",
                note: Some("Striped tints alternate rows; dense packs them closer."),
                build: table_striped,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Surface",
                "Surface",
                "Build the engine surface this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The table, plain.
fn table_plain() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Table,
        ds::cupertino::Table,
        ds::fluent::Table,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .headers(vec!["Component".to_string(), "Count".to_string()])
                .rows(vec![
                    vec!["Button".to_string(), "3".to_string()],
                    vec!["Checkbox".to_string(), "3".to_string()],
                    vec!["Counter".to_string(), "3".to_string()],
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The table, striped and dense.
fn table_striped() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Table,
        ds::cupertino::Table,
        ds::fluent::Table,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .headers(vec!["Component".to_string(), "Count".to_string()])
                .rows(vec![
                    vec!["Button".to_string(), "3".to_string()],
                    vec!["Checkbox".to_string(), "3".to_string()],
                    vec!["Counter".to_string(), "3".to_string()],
                ])
                .striped(true)
                .dense(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Text -----------------------------------------------------------------

/// The Text page.
fn text_page() -> ComponentPage {
    data_page(
        "text",
        "Text",
        "A run of prose, in the reader's own face.",
        vec![
            Sample {
                title: "A sentence",
                note: None,
                build: text_sentence,
            },
            Sample {
                title: "Clamped to two lines",
                note: Some("The clamp is where the text stops and an ellipsis takes over."),
                build: text_clamped,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Text",
                "Text",
                "Build the engine text this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The text, one sentence.
fn text_sentence() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Text,
        ds::cupertino::Text,
        ds::fluent::Text,
        |b, theme| {
            b::new("A gallery of every component, in three design systems.")
                .with_theme(theme)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The text, clamped to two lines.
fn text_clamped() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Text,
        ds::cupertino::Text,
        ds::fluent::Text,
        |b, theme| {
            b::new("A gallery of every component the satellite provides, shown in Material, in Cupertino and in Fluent, so that the three can be read side by side.")
                .with_theme(theme)
                .max_lines(2)
                .selectable(true)
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Timeline -------------------------------------------------------------

/// The Timeline page.
fn timeline_page() -> ComponentPage {
    data_page(
        "timeline",
        "Timeline",
        "A list of events, in the order they happened.",
        vec![
            Sample {
                title: "Two events",
                note: None,
                build: timeline_two,
            },
            Sample {
                title: "Three events",
                note: Some("The description is the supporting line under the title."),
                build: timeline_three,
            },
        ],
        ApiDoc {
            events: vec![],
            methods: vec![ApiRow::new(
                "view",
                "self -> floom_widgets::Stack",
                "Stack",
                "Build the engine stack this component renders.",
            )],
            ..Default::default()
        },
    )
}

/// The timeline, with two events.
fn timeline_two() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Timeline,
        ds::cupertino::Timeline,
        ds::fluent::Timeline,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .item("Started", "The first thing that happened")
                .item("Finished", "The last thing that happened")
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The timeline, with three events, built from a table.
fn timeline_three() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Timeline,
        ds::cupertino::Timeline,
        ds::fluent::Timeline,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .items(vec![
                    TimelineItem {
                        title: "Started".to_string(),
                        description: "The first thing that happened".to_string(),
                    },
                    TimelineItem {
                        title: "Paused".to_string(),
                        description: "Something interrupted it".to_string(),
                    },
                    TimelineItem {
                        title: "Finished".to_string(),
                        description: "The last thing that happened".to_string(),
                    },
                ])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- Treeview -------------------------------------------------------------

/// The Treeview page.
fn treeview_page() -> ComponentPage {
    data_page(
        "treeview",
        "Treeview",
        "A tree of things that contain other things.",
        vec![
            Sample {
                title: "A tree",
                note: None,
                build: treeview_tree,
            },
            Sample {
                title: "An expanded tree",
                note: Some("A node's children are indented beneath it."),
                build: treeview_expanded,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_node_click",
                "impl Fn(String) + Send + Sync",
                "String",
                "Called with the node's label when a node is clicked.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_ui_composites::TreeView",
                    "TreeView",
                    "Build the engine tree this component renders.",
                ),
                ApiRow::new(
                    "tree",
                    "&self -> floom_ui_composites::SharedTree",
                    "SharedTree",
                    "The tree's state, shared with whoever is reading it.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The tree, with one node open.
fn treeview_tree() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Treeview,
        ds::cupertino::Treeview,
        ds::fluent::Treeview,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .nodes(vec![TreeviewNode {
                    label: "crates".to_string(),
                    icon: None,
                    children: vec![TreeviewNode {
                        label: "floom-core".to_string(),
                        icon: None,
                        children: Vec::new(),
                        expanded: false,
                    }],
                    expanded: false,
                }])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The tree, with the top node open.
fn treeview_expanded() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::Treeview,
        ds::cupertino::Treeview,
        ds::fluent::Treeview,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .nodes(vec![TreeviewNode {
                    label: "crates".to_string(),
                    icon: None,
                    children: vec![TreeviewNode {
                        label: "floom-core".to_string(),
                        icon: None,
                        children: vec![TreeviewNode {
                            label: "compose.rs".to_string(),
                            icon: None,
                            children: Vec::new(),
                            expanded: false,
                        }],
                        expanded: true,
                    }],
                    expanded: true,
                }])
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

// -- VirtualScroller ------------------------------------------------------

/// The VirtualScroller page.
fn virtual_scroller_page() -> ComponentPage {
    data_page(
        "virtual_scroller",
        "VirtualScroller",
        "A long list that only draws the rows on show.",
        vec![
            Sample {
                title: "Many rows",
                note: Some("The rows are built on demand; only the ones on show are drawn."),
                build: virtual_scroller_many,
            },
            Sample {
                title: "Taller rows",
                note: None,
                build: virtual_scroller_tall,
            },
        ],
        ApiDoc {
            events: vec![ApiRow::new(
                "on_reach_end",
                "impl Fn() + Send + Sync",
                "()",
                "Called when the reader scrolls to the end.",
            )],
            methods: vec![
                ApiRow::new(
                    "view",
                    "self -> floom_widgets::Surface",
                    "Surface",
                    "Build the engine surface this component renders.",
                ),
                ApiRow::new(
                    "item_height",
                    "mut self -> Self",
                    "32.0",
                    "How tall one row is, which is how the list knows where to scroll.",
                ),
            ],
            ..Default::default()
        },
    )
}

/// The virtual scroller, over many rows.
fn virtual_scroller_many() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::VirtualScroller,
        ds::cupertino::VirtualScroller,
        ds::fluent::VirtualScroller,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .item_height(32.0)
                .height(160.0)
                .items((0..40).map(row_builder).collect())
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// The virtual scroller, over taller rows.
fn virtual_scroller_tall() -> Box<dyn WidgetAny> {
    let (m, c, f) = compare_children!(
        ds::material::VirtualScroller,
        ds::cupertino::VirtualScroller,
        ds::fluent::VirtualScroller,
        |b, theme| {
            b::new()
                .with_theme(theme)
                .item_height(48.0)
                .height(200.0)
                .row_gap(4.0)
                .overscan(4)
                .items((0..40).map(row_builder).collect())
                .view()
        }
    );
    Box::new(compare_row((m, c, f)))
}

/// One row of the virtual scroller: a builder that draws it when it is on
/// show. The row is built on demand, so the builder has to be replayable --
/// hence the clone of the text on every call.
fn row_builder(index: usize) -> WidgetBuilder {
    let text = format!("Row {index}");
    Arc::new(move || -> Box<dyn WidgetAny> { Box::new(Text::new(text.clone())) })
}
