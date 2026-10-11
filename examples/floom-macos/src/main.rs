//! Floom 2.0 on macOS — the desktop analogue of `examples/floom-android`.
//!
//! `floom-window` already proves the stack runs in a real window; what this
//! binary adds is the **device-side evidence** `docs/gates/macos.md` needs: a
//! tree you can click, a text field you can type into through whichever input
//! method is selected, and a clipboard round-trip whose other end is the system
//! pasteboard rather than a `String` in this process. Everything it learns is
//! reported on stderr, so a spot-check is a log read rather than a reading of
//! the source.
//!
//! Run it with:
//!
//! ```sh
//! cargo run -p floom-macos --features real
//! ```
//!
//! The whole real-window stack is behind the `real` feature, so the default
//! workspace build never compiles winit or wgpu.
//!
//! ## What the probe prints, and why each line exists
//!
//! | line | what it makes observable |
//! |---|---|
//! | `GPU adapter` | the presenter is the GPU one, not a software fallback |
//! | `widget tree mounted` | the tree is addressable, not merely started |
//! | `button logical=…` | where the app believes it drew the button, so a click can be aimed rather than guessed |
//! | `text field focused` | typed characters have a destination |
//! | `pointer -> button click` | input reached a widget through the real event loop |
//! | `clipboard …` | the system pasteboard accepted and returned text |
//! | `text field text=/preedit=` | the input method's characters reached the widget |

#![cfg(feature = "real")]

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, Ordering};
use std::thread;
use std::time::Duration;

use floom_app::{AppRunner, NodeId, WidgetHost};
use floom_core::{Axis, Color, LayoutRequest};
use floom_layout::LayoutStyle;
use floom_platform::{Clipboard, Ime, WindowConfig};
use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floom_renderer_vello::VelloGpuPresenter;
use floom_types::{ElementId, Size, WindowId};
use floom_widgets::{Button, Stack, Text, TextField};

/// Text written to the clipboard by the startup probe and read back both
/// immediately and after the first click. Distinctive on purpose: it has to be
/// recognisable in a log *and* unmistakable when pasted into another app by
/// hand, and it must not collide with whatever the person copied before.
const CLIPBOARD_PROBE: &str = "floom-macos clipboard probe 3e7b";

/// The application: an [`AppRunner`] plus the small amount of UI state the
/// probes need. The runner *is* the app once it is wired to a platform and a
/// presenter; what is left here is bookkeeping for what gets printed once.
struct MacWindowApp {
    runner: AppRunner<VelloGpuPresenter>,
    clicked: Arc<AtomicI32>,
    /// Last click count reported, so one tap prints one line.
    logged: i32,
    /// The counter label, resolved once the first frame has mounted the tree.
    label: Option<NodeId>,
    /// The text field, resolved and focused by the startup probe.
    field: Option<NodeId>,
    /// The realised window, learned from `window_created`.
    window: Option<WindowId>,
    /// The startup probe runs once.
    probed: bool,
    /// The focused element last reported, so a focus *move* prints one line
    /// (and so "the first focus request worked" is visible rather than assumed).
    last_focus: Option<ElementId>,
    /// Last field state printed, so a change prints once rather than every
    /// frame (an idle app would otherwise repeat itself forever).
    last_text: String,
    last_preedit: String,
    last_composing: bool,
}

impl WinitApp for MacWindowApp {
    fn window_created(&mut self, _platform: &mut WinitPlatform, window: WindowId) {
        self.window = Some(window);
    }

    fn events(&mut self, platform: &mut WinitPlatform) {
        // The frame hook: drain the platform queue, then report whatever moved.
        self.runner.pump(platform);
        self.sync_label(platform);
        self.sync_text_field();
        self.sync_focus();
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        // Drain once more so redraw requests queued during `events` are honoured
        // before the loop blocks. Nothing here calls `request_frame`: the runner
        // asks for a frame when the tree is dirty, and requesting one every time
        // the loop was about to wait is exactly how a loop stops parking.
        self.runner.pump(platform);
        self.sync_label(platform);
        self.sync_focus();
        self.probe_platform_services(platform);
    }
}

impl MacWindowApp {
    /// One-shot proof that the OS-facing services are live: clipboard, input
    /// method, focus.
    ///
    /// Each of these can be implemented in a way unit tests cannot tell apart
    /// from working — a `String` field answers `get_text` correctly forever. The
    /// only evidence is doing it against the real thing, which is what this is.
    fn probe_platform_services(&mut self, platform: &mut WinitPlatform) {
        if self.probed || self.label.is_none() {
            return;
        }
        self.probed = true;

        // Clipboard: write to the **system** pasteboard, then read it back. On
        // macOS that round-trip leaves this process (`NSPasteboard`); anything
        // the app could have faked would look identical here, which is why the
        // interesting half is reading the same string from *another* app's point
        // of view during the spot-check.
        let wrote = platform.set_text(CLIPBOARD_PROBE);
        let read_back = platform.get_text();
        eprintln!(
            "floom-macos: clipboard write={wrote} read_back={read_back:?} matched={}",
            read_back.as_deref() == Some(CLIPBOARD_PROBE)
        );

        // Input method: ask the OS to let text input through. This is what makes
        // a Chinese/Japanese/Korean composition produce `ImePreedit` instead of
        // six silent `KeyDown`s.
        if let Some(window) = self.window {
            platform.set_enabled(window, true);
        }

        // Focus the text field, so characters have somewhere to land. Without
        // this a spot-check types into nothing, and "nothing happened" is
        // indistinguishable from a broken input bridge.
        if let Some(field) = find_text_field(&self.runner) {
            let element = self.runner.host().node_id(field);
            self.runner.host_mut().set_focus(element);
            self.field = Some(field);
            eprintln!("floom-macos: text field focused");
        }

        // Tell the input method where the caret is, so its candidate window
        // lands next to the text. Reported rather than asserted: only a real
        // composition can prove the coordinates were right.
        if let (Some(window), Some(field)) = (self.window, self.field) {
            let rect = self.runner.host().bounds(field);
            platform.set_cursor_area(window, rect.origin(), rect.size());
        }

        // Where the app thinks the controls are, in logical points inside the
        // window's *content* area. Aiming a real click needs a screen
        // coordinate, so the window's own frame has to be added — see the note
        // in `docs/gates/macos.md` about the title-bar offset.
        if let Some(button) = find_button(&self.runner) {
            let rect = self.runner.host().bounds(button);
            let scale = self.runner.host().scale();
            eprintln!(
                "floom-macos: button logical=({:.1},{:.1} {:.1}x{:.1}) scale={scale} content_px=x{:.0}..{:.0} y{:.0}..{:.0}",
                rect.x0,
                rect.y0,
                rect.width(),
                rect.height(),
                rect.x0 * scale,
                rect.x1 * scale,
                rect.y0 * scale,
                rect.y1 * scale,
            );
        }
        if let Some(field) = self.field {
            let rect = self.runner.host().bounds(field);
            eprintln!(
                "floom-macos: field logical=({:.1},{:.1} {:.1}x{:.1})",
                rect.x0,
                rect.y0,
                rect.width(),
                rect.height()
            );
        }
    }

    /// Rewrites the label when the click counter moves, and reports the click.
    fn sync_label(&mut self, platform: &mut WinitPlatform) {
        let clicked = self.clicked.load(Ordering::SeqCst);
        let node = match self.label {
            Some(node) => node,
            None => {
                let Some(found) = find_label(&self.runner) else {
                    return;
                };
                self.label = Some(found);
                eprintln!(
                    "floom-macos: widget tree mounted ({} nodes)",
                    self.runner.host().node_count()
                );
                found
            }
        };
        if clicked != self.logged {
            // A click reached a widget: pointer event -> platform queue -> input
            // routing -> `on_click`. Printed here rather than inside the closure
            // so the trace is identical whether or not a frame follows.
            self.logged = clicked;
            eprintln!("floom-macos: pointer -> button click, counter = {clicked}");

            // Read the clipboard again, well after startup. This is a *second*,
            // independent read: if the pasteboard really took the write, the text
            // is still there now, and it can also be read from outside.
            let after = platform.get_text();
            eprintln!(
                "floom-macos: clipboard read after click = {after:?} (probe survived: {})",
                after.as_deref() == Some(CLIPBOARD_PROBE)
            );
        }
        if let Some(text) = self.runner.host_mut().widget_mut::<Text>(node) {
            text.set_content(format!("clicked {clicked} times"));
        }
    }

    /// Report every change of keyboard focus.
    ///
    /// This is what makes item 7 of the gate falsifiable from the outside: the
    /// host's focus state is readable (`focused()`), so pressing Tab either
    /// moves it or does not, and the log says which. Without this line a
    /// keyboard-navigation gap looks exactly like a spot-check that forgot to
    /// press anything.
    fn sync_focus(&mut self) {
        let focused = self.runner.host().focused();
        if focused == self.last_focus {
            return;
        }
        let described = focused
            .map(|id| describe_focus(&self.runner, id))
            .unwrap_or_else(|| "none".to_string());
        eprintln!(
            "floom-macos: focus {prev:?} -> {described}",
            prev = self.last_focus.map(|id| describe_focus(&self.runner, id))
        );
        self.last_focus = focused;
    }

    /// Report what the text field holds whenever it changes.
    ///
    /// This is the evidence that the input method's characters arrived: the OS
    /// turns keystrokes into `ImeCommit`/`ImePreedit`, the runner routes them to
    /// the focused field, and the field updates its value or its composing text.
    /// Printed only on change, because the frame hook runs every frame.
    fn sync_text_field(&mut self) {
        let Some(node) = self.field else { return };
        let Some(field) = self.runner.host().widget::<TextField>(node) else {
            return;
        };
        let text = field.text().to_string();
        let preedit = field.preedit().to_string();
        let composing = field.is_composing();
        if text == self.last_text
            && preedit == self.last_preedit
            && composing == self.last_composing
        {
            return;
        }
        eprintln!(
            "floom-macos: text field text={text:?} preedit={preedit:?} composing={composing}"
        );
        self.last_text = text;
        self.last_preedit = preedit;
        self.last_composing = composing;
    }
}

/// Optional wall-clock: exit on its own after this many seconds, so a spot-check
/// can be scripted end to end instead of leaving a window behind. Off unless
/// asked (`FLOOM2_MACOS_EXIT_SECS`), because a window that quits by itself is a
/// poor thing to hand someone who asked to see it.
fn arm_exit_watchdog() {
    let Ok(value) = env::var("FLOOM2_MACOS_EXIT_SECS") else {
        return;
    };
    let Ok(secs) = value.parse::<u64>() else {
        return;
    };
    if secs == 0 {
        return;
    }
    thread::spawn(move || {
        thread::sleep(Duration::from_secs(secs));
        eprintln!("floom-macos: exit watchdog fired after {secs}s");
        std::process::exit(0);
    });
}

fn main() {
    arm_exit_watchdog();

    let mut host = WidgetHost::new();

    let clicked = Arc::new(AtomicI32::new(0));
    let flag = clicked.clone();
    let root = Stack::new(Axis::Vertical)
        .gap(16.0)
        // The surface is black by default here, so the label names its colour
        // explicitly instead of relying on `Text`'s black default.
        .push_layout(LayoutRequest::new(), || {
            Box::new(Text::new("clicked 0 times").size(20.0).color(Color::WHITE))
        })
        .push_layout(LayoutRequest::new().width(260.0).height(40.0), || {
            // A non-ASCII string is preloaded so the *rendering* half of the
            // CJK question can be answered without typing anything: the log
            // can prove characters arrived, but only pixels can show whether
            // they were drawn. (Typing over it is unaffected.)
            Box::new(
                TextField::new()
                    .value("你好世界")
                    .placeholder("click here, then type"),
            )
        })
        .push_layout(LayoutRequest::new().width(160.0).height(40.0), move || {
            let flag = flag.clone();
            Box::new(Button::new("click me").on_click(move || {
                flag.fetch_add(1, Ordering::SeqCst);
            }))
        });

    host.mount_root(LayoutStyle::default(), root);

    let presenter = match VelloGpuPresenter::new() {
        Some(presenter) => presenter,
        None => {
            eprintln!("no usable GPU adapter; this example needs a GPU presenter");
            std::process::exit(1);
        }
    };
    eprintln!("GPU adapter: {}", presenter.adapter_info());

    let app = MacWindowApp {
        runner: AppRunner::new(host, presenter),
        clicked,
        logged: 0,
        label: None,
        field: None,
        window: None,
        probed: false,
        last_focus: None,
        last_text: String::new(),
        last_preedit: String::new(),
        last_composing: false,
    };

    let windows = vec![WindowConfig {
        title: "floom — macOS gate".into(),
        size: Size::new(480.0, 320.0),
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}

fn find_label(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
    let host = runner.host();
    for index in 0..host.node_count() {
        if let Some(node) = host.node_at(index) {
            if let Some(text) = host.widget::<Text>(node) {
                if text.content().starts_with("clicked") {
                    return Some(node);
                }
            }
        }
    }
    None
}

fn find_text_field(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
    let host = runner.host();
    for index in 0..host.node_count() {
        if let Some(node) = host.node_at(index) {
            if host.widget::<TextField>(node).is_some() {
                return Some(node);
            }
        }
    }
    None
}

/// Names an `ElementId` by the widget that owns it, so a focus trace reads as
/// "the text field" rather than an opaque id.
fn describe_focus(runner: &AppRunner<VelloGpuPresenter>, id: ElementId) -> String {
    let host = runner.host();
    for index in 0..host.node_count() {
        let Some(node) = host.node_at(index) else {
            continue;
        };
        if host.node_id(node) != id {
            continue;
        }
        if host.widget::<TextField>(node).is_some() {
            return format!("TextField@{index}");
        }
        if host.widget::<Button>(node).is_some() {
            return format!("Button@{index}");
        }
        if host.widget::<Text>(node).is_some() {
            return format!("Text@{index}");
        }
        return format!("node@{index}");
    }
    format!("{id:?}")
}

fn find_button(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
    let host = runner.host();
    for index in 0..host.node_count() {
        if let Some(node) = host.node_at(index) {
            if host.widget::<Button>(node).is_some() {
                return Some(node);
            }
        }
    }
    None
}
