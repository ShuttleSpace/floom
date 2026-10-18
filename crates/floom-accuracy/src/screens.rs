//! The reference gallery (Task 27.1): nine representative screens built from
//! real widgets as a pure function of a token table.
//!
//! Every screen shares one probed anatomy — a card (radius, fill, padding),
//! a title and body run, screen-specific rows, and a primary/secondary action
//! pair (accent fill, touch target, sibling gap) — so one measurement routine
//! works for all of them and the screens differ only the way real screens
//! differ: in their rows.

use floom_accessibility::SemanticsRole;
use floom_core::{Align, Axis, Child, ChildKey, LayoutRequest, WidgetAny};
use floom_types::{Color, Size};
use floom_widgets::{Container, Spacer, Stack, Switch, Text};

use crate::labeled::Labeled;
use crate::tokens::ScreenTokens;

/// The window every gallery screen is drawn in. The reference contracts
/// specify the same dimensions.
pub fn screen_size() -> Size {
    Size::new(360.0, 640.0)
}

/// Gap between a card's title and body runs.
const INNER_GAP: f64 = 6.0;

/// The screens the plan asks the gallery to cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Screen {
    Settings,
    Dashboard,
    Feed,
    Editor,
    MediaList,
    Form,
    Dialog,
    Navigation,
    Webview,
}

impl Screen {
    pub const ALL: [Screen; 9] = [
        Screen::Settings,
        Screen::Dashboard,
        Screen::Feed,
        Screen::Editor,
        Screen::MediaList,
        Screen::Form,
        Screen::Dialog,
        Screen::Navigation,
        Screen::Webview,
    ];

    /// The card's title — also the accessibility label the contract names.
    pub fn title(self) -> &'static str {
        match self {
            Screen::Settings => "Settings",
            Screen::Dashboard => "Dashboard",
            Screen::Feed => "Feed",
            Screen::Editor => "Editor",
            Screen::MediaList => "Library",
            Screen::Form => "Create account",
            Screen::Dialog => "Confirm delete",
            Screen::Navigation => "Navigation",
            Screen::Webview => "Browser",
        }
    }

    pub(crate) fn body(self) -> &'static str {
        match self {
            Screen::Settings => "Preferences sync across your devices.",
            Screen::Dashboard => "Weekly overview.",
            Screen::Feed => "Top stories today.",
            Screen::Editor => "Draft autosaved just now.",
            Screen::MediaList => "3 albums, 42 tracks.",
            Screen::Form => "All fields are required.",
            Screen::Dialog => "This action cannot be undone.",
            Screen::Navigation => "Drawer and content pane.",
            Screen::Webview => "External content placeholder.",
        }
    }

    /// The labels the screen's semantics must publish.
    pub fn expected_labels(self) -> Vec<&'static str> {
        let mut labels = vec![
            self.title(),
            self.body(),
            self.primary_action(),
            self.secondary_action(),
        ];
        labels.extend(self.extra_rows());
        labels
    }

    pub(crate) fn primary_action(self) -> &'static str {
        match self {
            Screen::Settings | Screen::Navigation => "Save",
            Screen::Dashboard => "Export",
            Screen::Feed => "Like",
            Screen::Editor => "Publish",
            Screen::MediaList => "Play",
            Screen::Form => "Submit",
            Screen::Dialog => "Confirm",
            Screen::Webview => "Open",
        }
    }

    pub(crate) fn secondary_action(self) -> &'static str {
        match self {
            Screen::Settings | Screen::Navigation => "Cancel",
            Screen::Dashboard => "Dismiss",
            Screen::Feed => "Share",
            Screen::Editor => "Discard",
            Screen::MediaList => "Queue",
            Screen::Form => "Reset",
            Screen::Dialog => "Cancel",
            Screen::Webview => "Dismiss",
        }
    }

    fn extra_rows(self) -> Vec<&'static str> {
        match self {
            Screen::Settings => vec!["Enable sync", "Show notifications"],
            Screen::Dashboard => vec!["Revenue", "12,400"],
            Screen::Feed => vec!["Post one", "Post two", "Post three"],
            Screen::Editor | Screen::Form | Screen::Dialog => vec![],
            Screen::MediaList => vec!["Track one", "Track two", "Track three"],
            Screen::Navigation => vec!["Home", "Library", "Settings nav"],
            Screen::Webview => vec!["Web content", "External content placeholder."],
        }
    }

    /// Why the webview screen has no web content: the gap is the Phase 17 OS
    /// providers, not an oversight of the gallery.
    pub fn placeholder_note(self) -> &'static str {
        match self {
            Screen::Webview => {
                "Real web content needs the Phase 17 OS providers; this screen \
                 mounts a labelled placeholder so the layout and semantics are \
                 still exercised."
            }
            _ => "not a placeholder",
        }
    }

    /// Builds the screen from a token table — this *is* the implementation
    /// under test.
    ///
    /// Three layers, bottom to top: the surface the root paints, an optional
    /// scrim (the dialog), and the content pane (optionally split with a
    /// drawer, the navigation screen).
    pub fn build(self, tokens: &ScreenTokens) -> Stack {
        let t = tokens.clone();

        // The content column: card, rows, then the action pair. The gap probe
        // reads the space between the two action boxes, so they are adjacent.
        let mut content = Stack::new(Axis::Vertical).gap(t.gap as f32);
        content = content.child(card_child(&t, self.title(), self.body()));
        for row in self.rows(&t) {
            content = content.child(row);
        }
        content = content.child(action_child(
            self.primary_action(),
            &t,
            t.accent,
            t.on_accent,
        ));
        content = content.child(action_child(self.secondary_action(), &t, t.card, t.text));

        // The pane: navigation splits the content column with a drawer.
        let pane: Stack = match self {
            Screen::Navigation => {
                let mut drawer = Stack::new(Axis::Vertical).gap(t.gap as f32);
                for label in ["Home", "Library", "Settings nav"] {
                    drawer = drawer.child(text_child(label, t.body_size, t.text));
                }
                let mut split = Stack::new(Axis::Horizontal).gap(t.gap as f32);
                split = split.push_layout(LayoutRequest::new().width(96.0), move || {
                    Box::new(drawer.clone())
                });
                split = split.push_layout(LayoutRequest::new().fill(), move || {
                    Box::new(content.clone())
                });
                split
            }
            _ => content,
        };

        // The layer above the surface: the dialog dims its host first.
        let layer = match self {
            Screen::Dialog => Stack::new(Axis::Vertical)
                .push_layout(LayoutRequest::new().absolute_fill(), {
                    move || Box::new(Container::new().background(Color::rgba(0, 0, 0, 0x66)))
                })
                .push_layout(LayoutRequest::new().fill(), move || Box::new(pane.clone())),
            _ => Stack::new(Axis::Vertical)
                .push_layout(LayoutRequest::new().fill(), move || Box::new(pane.clone())),
        };

        Stack::new(Axis::Vertical)
            .push_layout(LayoutRequest::new().absolute_fill(), {
                let surface = t.surface;
                move || Box::new(Container::new().background(surface))
            })
            .push_layout(LayoutRequest::new().fill(), move || Box::new(layer.clone()))
    }

    /// The screen-specific rows between the card and the action pair.
    fn rows(self, t: &ScreenTokens) -> Vec<Child> {
        let t = t.clone();
        match self {
            Screen::Settings => vec![
                control_child("enable-sync", move || {
                    Box::new(Switch::new("Enable sync").on(true))
                }),
                // This row was a `Checkbox` until that moved to
                // `floom-ui-composites`, which lives in the `floom-ui`
                // workspace the root one excludes and which this
                // measurement harness cannot depend on. `Switch` is the
                // same role -- a labelled two-state control that publishes
                // `CHECKED` -- and `extra_rows` still names the label, so
                // the assertion that every screen publishes its labels is
                // testing the same thing it was.
                control_child("notifications", move || {
                    Box::new(Switch::new("Show notifications"))
                }),
            ],
            Screen::Dashboard => vec![
                text_child("Revenue", t.body_size, t.text),
                text_child("12,400", t.title_size, t.text),
            ],
            Screen::Feed => vec![
                text_child("Post one", t.body_size, t.text),
                text_child("Post two", t.body_size, t.text),
                text_child("Post three", t.body_size, t.text),
            ],
            Screen::MediaList => vec![
                text_child("Track one", t.body_size, t.text),
                text_child("Track two", t.body_size, t.text),
                text_child("Track three", t.body_size, t.text),
            ],
            Screen::Editor | Screen::Form | Screen::Dialog | Screen::Navigation => vec![],
            Screen::Webview => vec![webview_child(&t)],
        }
    }
}

/// A plain text row as a child slot.
fn text_child(label: &'static str, size: f32, color: Color) -> Child {
    Child::new(ChildKey::from(label), LayoutRequest::new(), move || {
        Box::new(Text::new(label).size(size).color(color))
    })
}

/// A switch/checkbox row.
fn control_child(
    key: &'static str,
    build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
) -> Child {
    Child::new(ChildKey::from(key), LayoutRequest::new(), build)
}

/// The webview placeholder: a labelled group with no painted box of its own,
/// so it cannot be mistaken for a card by the geometry probes.
fn webview_child(t: &ScreenTokens) -> Child {
    let t = t.clone();
    Child::new("web-content", LayoutRequest::new(), move || {
        let t = t.clone();
        Box::new(Labeled::new(SemanticsRole::Group, "Web content").child(
            LayoutRequest::new().fill(),
            move || {
                let t = t.clone();
                Box::new(Stack::new(Axis::Vertical).push(move || {
                    Box::new(
                        Text::new("External content placeholder.")
                            .size(t.body_size)
                            .color(t.text),
                    )
                }))
            },
        ))
    })
}

/// The card: a painted box (radius + fill, found geometrically by the
/// evaluation) with a padded title/body run laid over it via an absolute-fill
/// background — the engine's in-tree overlay vocabulary.
fn card_child(t: &ScreenTokens, title: &str, body: &str) -> Child {
    let title_lh = floom_text::measure_text(title, t.title_size, f64::INFINITY).height as f32;
    let body_lh = floom_text::measure_text(body, t.body_size, f64::INFINITY).height as f32;
    let card_h = t.padding as f32 * 2.0 + title_lh + INNER_GAP as f32 + body_lh;
    let padding = t.padding;
    let (title, body) = (title.to_string(), body.to_string());
    let t0 = t.clone();
    Child::new("card", LayoutRequest::new().height(card_h), move || {
        let t = t0.clone();
        Box::new(
            Stack::new(Axis::Vertical)
                .push_layout(LayoutRequest::new().absolute_fill(), {
                    let t = t.clone();
                    move || Box::new(Container::new().background(t.card).radius(t.radius))
                })
                .push({
                    move || Box::new(Spacer::new(padding).axis(floom_widgets::Axis::Vertical))
                })
                .push({
                    let t = t.clone();
                    let title = title.clone();
                    let body = body.clone();
                    move || {
                        let t = t.clone();
                        let title = title.clone();
                        let body = body.clone();
                        Box::new(
                            Stack::new(Axis::Horizontal)
                                .push({
                                    let t = t.clone();
                                    move || Box::new(Spacer::new(t.padding))
                                })
                                .push({
                                    let t = t.clone();
                                    let title = title.clone();
                                    let body = body.clone();
                                    move || {
                                        let t = t.clone();
                                        let title = title.clone();
                                        Box::new(
                                            Stack::new(Axis::Vertical)
                                                .push({
                                                    let t = t.clone();
                                                    let title = title.clone();
                                                    move || {
                                                        Box::new(
                                                            Text::new(title.clone())
                                                                .size(t.title_size)
                                                                .color(t.text),
                                                        )
                                                    }
                                                })
                                                .push({
                                                    move || {
                                                        Box::new(
                                                            Spacer::new(INNER_GAP).axis(
                                                                floom_widgets::Axis::Vertical,
                                                            ),
                                                        )
                                                    }
                                                })
                                                .push({
                                                    let t = t.clone();
                                                    let body = body.clone();
                                                    move || {
                                                        Box::new(
                                                            Text::new(body.clone())
                                                                .size(t.body_size)
                                                                .color(t.text),
                                                        )
                                                    }
                                                }),
                                        )
                                    }
                                })
                                .push({
                                    let t = t.clone();
                                    move || Box::new(Spacer::new(t.padding))
                                }),
                        )
                    }
                })
                .push({
                    move || Box::new(Spacer::new(padding).axis(floom_widgets::Axis::Vertical))
                }),
        )
    })
}

/// An action box: a labelled `Button`-role region with a fixed height (the
/// touch target), an absolute-fill painted box, and centred text.
fn action_child(label: &str, t: &ScreenTokens, fill: Color, text_color: Color) -> Child {
    let label = label.to_string();
    let height = t.control_height;
    let body_size = t.body_size;
    Child::new(
        ChildKey::from(label.as_str()),
        LayoutRequest::new().height(height),
        move || {
            let label = label.clone();
            Box::new(
                Labeled::new(SemanticsRole::Button, label.clone())
                    .request(LayoutRequest::new().height(height))
                    .child(LayoutRequest::new().fill(), move || {
                        let label = label.clone();
                        Box::new(
                            Stack::new(Axis::Horizontal)
                                .align(Align::Center, Align::Center)
                                .push_layout(LayoutRequest::new().absolute_fill(), {
                                    move || Box::new(Container::new().background(fill))
                                })
                                .push(move || {
                                    Box::new(
                                        Text::new(label.clone()).size(body_size).color(text_color),
                                    )
                                }),
                        )
                    }),
            )
        },
    )
}
