//! The gallery tree.
//!
//! Two tabs, which is the split Flutter's integration gallery and Slint's demos
//! use, and for the same reason: **a reading and an action are not the same kind
//! of claim.** The first tab only observes the device, so opening it has no side
//! effects and is safe on someone else's phone. The second tab vibrates,
//! shares, retints the OS bars and asks for permissions, so every row there is a
//! button the user presses deliberately.
//!
//! Both tabs render from the same [`crate::probe`] data, so a device that cannot
//! serve a capability says so in both.
//!
//! # The one place this file could have lied
//!
//! The obvious way to build a row is a name on the left and a value on the
//! right, with the absent ones greyed out. That is what this does — and the
//! greying is the whole design. There is no code path here that formats a
//! missing answer as a plausible one, because [`crate::probe`] has already
//! refused to produce one: the plugins' host arms carry no measurement
//! (`battery_state()` gives `level: None`), and a gallery that printed "100%" on
//! a laptop would be indistinguishable from one that works on a phone.
//!
//! # Two API shapes worth knowing before reading on
//!
//! **A `Child` builder is a replayable `Fn`, not a `FnOnce`.** Every closure
//! here therefore clones what it captured *inside its own body* and hands the
//! clone to any nested `move` closure; writing `move || body.clone()` in a nested
//! position would move `body` out of the enclosing `Fn` and not compile. Where a
//! subtree is genuinely static, the clone is hoisted out of the loop entirely.
//!
//! **`Container` is a leaf decoration box, not a wrapper.** It paints a
//! background and carries an `Insets`, but holds no children — so a "card" is a
//! `Stack` of an absolutely-positioned background `Container` plus the content on
//! top, and inner padding is expressed with [`Spacer`]s. (`Container::padding`
//! stores an `Insets` that nothing in `floom-app`/`floom-layout` reads yet, so
//! relying on it would silently render flush-left.)

use floom_core::{Align, Axis, Child, ChildKey, Color, LayoutRequest, WidgetAny};
use floom_platform_mobile::packages::{keyboard::soft_input, system_chrome};
use floom_plugin_file_dialog as file_dialog;
use floom_plugin_local_auth as local_auth;
use floom_plugin_location as location;
use floom_plugin_notifications as notifications;
use floom_plugin_open_url as open_url;
use floom_plugin_prefs as prefs;
use floom_plugin_share as share;
use floom_plugin_vibration as vibration;
use floom_widgets::{Button, Container, Spacer, Stack, Text};

use crate::probe::{self, Row, State};

/// Muted ink for the question column, so the values stay the loudest thing.
const DIM: Color = Color::rgb(0x6b, 0x6b, 0x6b);
/// The value column's ink — full contrast; this is what the row is *for*.
const INK: Color = Color::rgb(0x1a, 0x1a, 0x1a);
/// An absent capability: present in the table, visibly not answering.
const MUTED: Color = Color::rgb(0x9a, 0x9a, 0x9a);
/// A capability that exists but failed — kept distinct from absent, so a reader
/// can tell "this platform has no such thing" from "the call did not work".
const WARN: Color = Color::rgb(0xa8, 0x4b, 0x00);
/// Panel background, so the two columns read as columns.
const PANEL: Color = Color::rgb(0xf4, 0xf3, 0xef);
/// Page background.
const PAGE: Color = Color::rgb(0xfa, 0xf9, 0xf6);
/// The title's ink, one step down from the values.
const HEAD: Color = Color::rgb(0x33, 0x33, 0x33);
/// The hairline under a section heading.
const RULE: Color = Color::rgb(0xdd, 0xda, 0xd2);

/// Width of the name/question column, in points.
///
/// Fixed rather than intrinsic so the value column starts at the same x on
/// every row — a capability table you cannot scan down is a list, not a table.
const NAME_COLUMN: f32 = 230.0;

/// Key space for section headers, kept clear of the row indices.
///
/// `push_keyed` takes a `u64`, and a header and the row it introduces share an
/// index — so the two would collide in the reconciler's key space and one would
/// silently reuse the other's mounted widget. XOR-ing into the high bit keeps
/// them disjoint without needing a counter.
const HEADER_KEY: u64 = 1 << 40;

/// The value column's ink for a row's state.
fn ink_for(state: &State) -> Color {
    match state {
        State::Value(_) => INK,
        State::Absent => MUTED,
        State::Error(_) => WARN,
    }
}

/// Empty space, in points. `Spacer` is how this file expresses padding.
fn gap(extent: f64) -> Box<dyn WidgetAny> {
    Box::new(Spacer::new(extent))
}

/// One capability's row: its name, the question it answers, and the answer.
///
/// The name is `INK` whatever the state, because the *capability existing* is a
/// fact; only the answer's ink varies. That way a table of twelve `—` still
/// reads as twelve capabilities rather than as a broken screen.
fn capability_row(row: &'static Row) -> Box<dyn WidgetAny> {
    let name = row.name;
    let question = row.question;
    let value = row.state.text().to_string();
    let value_color = ink_for(&row.state);

    Box::new(
        Stack::new(Axis::Horizontal)
            .align(Align::Start, Align::Start)
            .gap(20.0)
            .push_layout(LayoutRequest::new().width(NAME_COLUMN), move || {
                Box::new(
                    Stack::new(Axis::Vertical)
                        .gap(1.0)
                        .push_layout(LayoutRequest::new(), move || {
                            Box::new(Text::new(name).size(15.0).color(INK))
                        })
                        .push_layout(LayoutRequest::new(), move || {
                            Box::new(Text::new(question).size(12.0).color(DIM))
                        }),
                )
            })
            .push_layout(LayoutRequest::new().fill(), move || {
                Box::new(Text::new(value.clone()).size(14.0).color(value_color))
            }),
    )
}

/// A filled card around `content`, inset by `inset` on every side.
///
/// Two `Stack`s rather than one `Container`: the outer one layers an
/// absolutely-positioned background under a content column of
/// `Spacer`/content/`Spacer`, which is the only way to get both a fill and an
/// inset out of the current widget set.
fn card(inset: f64, content: Stack) -> Box<dyn WidgetAny> {
    let content = content.clone();
    Box::new(
        Stack::new(Axis::Vertical)
            .align(Align::Stretch, Align::Stretch)
            .push_layout(LayoutRequest::new().absolute_fill(), || {
                Box::new(Container::new().background(PANEL))
            })
            .push_layout(LayoutRequest::new().fill(), move || {
                let content = content.clone();
                Box::new(
                    Stack::new(Axis::Vertical)
                        .gap(0.0)
                        .push_layout(LayoutRequest::new().height(inset as f32), move || {
                            gap(inset)
                        })
                        .push_layout(LayoutRequest::new().fill(), move || {
                            Box::new(content.clone())
                        })
                        .push_layout(LayoutRequest::new().height(inset as f32), move || {
                            gap(inset)
                        }),
                )
            }),
    )
}

/// A titled line: a heading with a line of explanation under it.
fn titled(headline: &'static str, detail: &'static str) -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(4.0)
            .push_layout(LayoutRequest::new().height(30.0), move || {
                Box::new(Text::new(headline).size(24.0).color(HEAD))
            })
            .push_layout(LayoutRequest::new().height(20.0), move || {
                Box::new(Text::new(detail).size(14.0).color(DIM))
            }),
    )
}

/// A section header inside the readings table.
///
/// Sized in points rather than left to the text's intrinsic height, because a
/// `Stack` row that grows taller than the space reserved for it would push the
/// capability it introduces one row out of alignment with its heading.
fn section_header(title: &'static str) -> Box<dyn WidgetAny> {
    Box::new(
        Stack::new(Axis::Vertical)
            .gap(2.0)
            .push_layout(LayoutRequest::new().height(22.0), move || {
                Box::new(Text::new(title).size(13.0).color(HEAD))
            })
            // A hairline under the heading, so the section reads as a block even
            // where the panel behind it is the same colour throughout.
            .push_layout(LayoutRequest::new().height(1.0), || {
                Box::new(Container::new().background(RULE))
            }),
    )
}

/// The readings tab: everything observed without side effects.
fn readings_tab() -> Child {
    let rows = probe::all();
    let count = rows.len();

    // `Box::leak` is deliberate and bounded: a `Child` builder is a replayable
    // `Fn`, so the `Row` it closes over must outlive the call. The count is the
    // number of capabilities, not of frames — the table is built once.
    //
    // Section headers come from `probe::GROUPS`, which is keyed to row *names*
    // and covered by a test, so a row inserted inside a section cannot silently
    // move a heading onto the wrong block.
    let table =
        rows.iter()
            .enumerate()
            .fold(Stack::new(Axis::Vertical).gap(10.0), |stack, (i, row)| {
                let stack = match probe::group_header_for(row.name) {
                    Some(title) => {
                        stack.push_keyed(HEADER_KEY ^ i as u64, move || section_header(title))
                    }
                    None => stack,
                };
                let row: &'static Row = Box::leak(Box::new(row.clone()));
                stack.push_keyed(i as u64, move || capability_row(row))
            });

    let banner: &'static str = if probe::native_capability_host() {
        Box::leak(
            format!("Live readings from this device — {count} capabilities probed.")
                .into_boxed_str(),
        )
    } else {
        Box::leak(
            format!(
                "No native capability host, so every OS-backed row below is honestly empty. \
                 These are not failures: a desktop has no battery, no vibrator and no \
                 UIDevice. Run the same build on Android or iOS to see {count} rows fill in."
            )
            .into_boxed_str(),
        )
    };

    let body = Stack::new(Axis::Vertical)
        .gap(18.0)
        .push_layout(LayoutRequest::new().height(40.0), move || {
            Box::new(Text::new(banner).size(14.0).color(DIM))
        })
        .push_layout(LayoutRequest::new().fill(), move || Box::new(table.clone()));

    Child::new(ChildKey::index(0), LayoutRequest::new().fill(), move || {
        card(20.0, body.clone())
    })
}

/// The actions tab: the capabilities whose value *is* the act.
///
/// Nothing here runs at startup, deliberately. This example is meant to be
/// opened on a device that is not the reader's own without buzzing it, posting
/// to it, or throwing a permission dialog at them.
fn actions_tab() -> Child {
    const NOTE: &str = "Each button calls the real API and prints the result to stdout. Nothing \
        runs automatically: a capability demo should not vibrate or post to a device that merely \
        opened the app. Actions whose OS result arrives later (a share sheet, a biometric \
        prompt) report only that the call was accepted.";

    // Two columns, so eight buttons do not become a page of scrolling. Built by
    // hand rather than with `Grid`, which has no `WidgetAny` impl and so cannot
    // be handed to a `Child` builder.
    let buttons = probe::actions().chunks(2).enumerate().fold(
        Stack::new(Axis::Vertical).gap(20.0),
        |page, (i, pair)| {
            let mut row = Stack::new(Axis::Horizontal).gap(16.0);
            for (label, id) in pair.iter().copied() {
                row = row.push_layout(LayoutRequest::new().grow(1.0), move || {
                    Box::new(
                        Stack::new(Axis::Vertical)
                            .gap(3.0)
                            .push_layout(LayoutRequest::new().height(34.0), move || {
                                Box::new(Button::new(label).on_click(move || run_action(id)))
                            })
                            // The `device::*` id is what a reader greps for, and
                            // it names the exact call the button makes — worth
                            // showing next to a label that only says "Vibrate".
                            .push_layout(LayoutRequest::new(), move || {
                                Box::new(Text::new(id).size(11.0).color(DIM))
                            }),
                    )
                });
            }
            page.push_keyed(i as u64, move || Box::new(row.clone()))
        },
    );

    let body = Stack::new(Axis::Vertical)
        .gap(18.0)
        .push_layout(LayoutRequest::new().height(60.0), move || {
            Box::new(Text::new(NOTE).size(13.0).color(DIM))
        })
        .push_layout(LayoutRequest::new().fill(), move || {
            Box::new(buttons.clone())
        });

    Child::new(ChildKey::index(0), LayoutRequest::new().fill(), move || {
        card(20.0, body.clone())
    })
}

/// Run one action and report what the OS said.
///
/// The result goes to stdout rather than into a row because several of these
/// open a system sheet whose outcome arrives asynchronously: a button that
/// reported "done" the instant it was pressed would be claiming something it
/// cannot know yet.
fn run_action(id: &str) {
    let outcome: Result<String, String> = match id {
        "device::vibrate" => vibration::vibrate(400).map(|()| "vibrated 400 ms".to_string()),
        "device::haptic" => vibration::haptic_feedback(vibration::HapticFeedback::Medium)
            .map(|()| "medium impact".to_string()),
        "device::share" => share::share_text("hello from floom-capabilities")
            .map(|()| "share sheet opened".to_string()),
        "device::authenticate" => {
            local_auth::authenticate("prove it is you").map(|()| "authenticated".to_string())
        }
        "device::request_location" => location::request_authorization()
            .map(|()| format!("now {:?}", location::authorization_status())),
        "device::request_notifications" => notifications::request_authorization()
            .map(|()| format!("now {}", notifications::are_notifications_enabled())),
        // `set_system_chrome_style` is the whole of `system_chrome`'s public
        // surface: there is no edge-to-edge toggle to ask for and no getter to
        // read back, so the button applies a concrete style and reports whether
        // the platform accepted it.
        "device::system_chrome" => {
            system_chrome::set_system_chrome_style(system_chrome::SystemChromeStyle {
                status_bar_color: Some(Color::rgb(0x1a, 0x1a, 0x1a)),
                navigation_bar_color: Some(Color::rgb(0x1a, 0x1a, 0x1a)),
                status_bar_icon_brightness: system_chrome::BarIconBrightness::Light,
                navigation_bar_icon_brightness: system_chrome::BarIconBrightness::Light,
            })
            .map(|()| "dark bars, light icons".to_string())
        }
        // Succeeds vacuously on a host: there is no soft keyboard to show, and
        // the package documents that as `Ok(())` rather than `Err` so callers
        // need no `#[cfg]`. The button therefore reports "ok" on a laptop — which
        // is a *different* fact from "the bridge is missing", and the reason the
        // id is printed under the label.
        "device::show_keyboard" => {
            soft_input::set_soft_keyboard_visible(true).map(|()| "show requested".to_string())
        }
        // The only action that changes the world outside the app on a desktop:
        // the default browser opens the repository page. `Ok` claims the OS
        // accepted the request — that is the plugin's whole contract — so the
        // report says "accepted", and the browser window is the human's proof.
        "device::open_url" => open_url::open_url("https://github.com/lapce/floom")
            .map(|()| "accepted — the default browser should be opening".to_string()),
        // The three outcomes the API promises, kept distinct in the report:
        // a path (the user chose), "cancelled" (the user's answer), and the
        // refusal (a platform whose picker needs plumbing this build does not
        // have — the phone case, spelled out rather than blurred into
        // "error").
        "device::pick_file" => match file_dialog::pick_file(&file_dialog::Options::new()) {
            Ok(Some(path)) => Ok(format!("chose {}", path.display())),
            Ok(None) => Ok("cancelled — the user's answer, not an error".to_string()),
            Err(reason) => Err(reason),
        },
        // Read, increment, write — through the OS's own store, so the value
        // the report names is the one a *future process* will read back.
        "device::count_launch" => prefs::Preferences::open(probe::PREFS_STORE)
            .and_then(|store| {
                let previous: u64 = store
                    .get("launches")
                    .and_then(|v| v.trim().parse().ok())
                    .unwrap_or(0);
                store
                    .set("launches", &(previous + 1).to_string())
                    .map(|()| (previous, previous + 1))
            })
            .map(|(previous, now)| {
                format!(
                    "launch {previous} -> {now} — recorded; the row above will read it \
                     back on the next run"
                )
            }),
        other => Err(format!("unknown action `{other}`")),
    };
    match outcome {
        Ok(ok) => println!("[capabilities] {id}: {ok}"),
        Err(e) => println!("[capabilities] {id}: unavailable — {e}"),
    }
}

/// The whole gallery: a title, both panels, and a footer naming the host.
///
/// Returns a concrete `Stack` rather than a `Box<dyn WidgetAny>`: every backend
/// hands this to `WidgetHost::mount_root`, which takes `impl WidgetAny`, and a
/// `Stack` is the one type that satisfies all of them without a cast.
pub fn gallery_root() -> Stack {
    let footer: &'static str = Box::leak(
        format!(
            "target {}/{} · native capability host: {}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            if probe::native_capability_host() {
                "yes"
            } else {
                "no"
            }
        )
        .into_boxed_str(),
    );

    let body = Stack::new(Axis::Vertical)
        .gap(18.0)
        .push_layout(LayoutRequest::new().height(54.0), || {
            titled(
                "Floom 2.0 platform capabilities",
                "The layer between a widget tree and the device it runs on.",
            )
        })
        .push_layout(LayoutRequest::new().fill(), || {
            Box::new(
                Stack::new(Axis::Vertical)
                    .gap(10.0)
                    .push_layout(LayoutRequest::new().height(18.0), || {
                        Box::new(Text::new("Readings").size(13.0).color(HEAD))
                    })
                    .push_layout(LayoutRequest::new().fill(), || readings_tab().build())
                    .push_layout(LayoutRequest::new().height(18.0), || {
                        Box::new(Text::new("Actions").size(13.0).color(HEAD))
                    })
                    .push_layout(LayoutRequest::new().fill(), || actions_tab().build()),
            )
        })
        .push_layout(LayoutRequest::new().height(16.0), move || {
            Box::new(Text::new(footer).size(12.0).color(MUTED))
        });

    let body = body.clone();
    Stack::new(Axis::Vertical)
        .align(Align::Stretch, Align::Stretch)
        .push_layout(LayoutRequest::new().absolute_fill(), || {
            Box::new(Container::new().background(PAGE))
        })
        .push_layout(LayoutRequest::new().fill(), move || {
            let body = body.clone();
            Box::new(
                Stack::new(Axis::Vertical)
                    .gap(0.0)
                    .push_layout(LayoutRequest::new().height(24.0), || gap(24.0))
                    .push_layout(LayoutRequest::new().fill(), move || Box::new(body.clone()))
                    .push_layout(LayoutRequest::new().height(24.0), || gap(24.0)),
            )
        })
}
