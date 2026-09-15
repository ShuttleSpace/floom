//! Floem 2.0 **device probe**: one window holding every control the on-screen
//! cases in `.superpowers/sdd/device-verification-2026-09-15.md` need.
//!
//! `examples/floem2-window` is deliberately a 222-line proof that the pipeline
//! runs — a counter, a label, a gradient. It contains no `TextField`, no
//! `Dialog`, no `Select`, so none of the R1/R2 cases (real IME candidate window,
//! popups escaping their parent's box, light dismiss, the entry animation,
//! focus under a modal) could be driven against it. This example exists to be
//! driven:
//!
//! ```sh
//! cargo run -p floem2-probe --features real
//! ```
//!
//! What is in the window, and which case wants it:
//!
//! | control | case |
//! | --- | --- |
//! | two `TextField`s | R1-1…R1-4 (composition, candidate window, moving focus) |
//! | a `Select` | R2-1 (the reference: what an escaping popup looks like) |
//! | `Dialog` (modal) | R2-4 (scrim must not dismiss), R2-8 (keyboard under a modal) |
//! | `Popup` (non-modal) | R2-3 (click outside dismisses), R2-6 (fade in, then still) |
//! | `Popup` inside a 160×48 box | R2-2 (it must be drawn outside that box) |
//! | `BottomSheet` | R2-6 (slide up from the bottom edge) |
//! | `NavigationDrawer` | R2-7 (leading-edge slide, stacked above a popup) |
//!
//! Unlike `floem2-window`, this app does **not** ask for a frame on every
//! `about_to_wait`. It asks for one frame to get started and then runs only when
//! something actually happened — an event, or a widget owing a frame (the entry
//! animation, for instance). That is the behaviour G1-4 exists to observe, and
//! it is why a settled probe window should be idle rather than spinning.

#![cfg(feature = "real")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floem_app::{AppRunner, NodeId, WidgetHost};
use floem_core::{Axis, Child, ChildKey, LayoutRequest};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
use floem_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;
use floem_widgets::{
    BottomSheet, Button, Dialog, NavDestination, NavigationDrawer, Popup, Select, Stack, Text,
    TextField,
};

/// Which surface the last click asked for. Buttons write, the frame hook reads.
const NONE: usize = 0;
const DIALOG: usize = 1;
const POPUP: usize = 2;
const SHEET: usize = 3;
const DRAWER: usize = 4;

fn name_of(request: usize) -> &'static str {
    match request {
        DIALOG => "dialog",
        POPUP => "popup",
        SHEET => "sheet",
        DRAWER => "drawer",
        _ => "none",
    }
}

struct ProbeApp {
    runner: AppRunner<VelloGpuPresenter>,
    /// What the buttons asked for; `NONE` closes everything.
    request: Arc<AtomicUsize>,
    /// The request already pushed into the widgets, so each click is applied once.
    applied: usize,
    /// Resolved lazily: the composite children mount during the first frame.
    surfaces: Option<Surfaces>,
    status: Option<NodeId>,
    frames: u32,
}

/// The four surfaces, by node. Resolved once, then held: the nodes stay put for
/// the life of the tree (a composite only mounts and unmounts its *children*).
#[derive(Clone, Copy)]
struct Surfaces {
    dialog: NodeId,
    popup: NodeId,
    sheet: NodeId,
    drawer: NodeId,
}

impl WinitApp for ProbeApp {
    fn events(&mut self, platform: &mut WinitPlatform) {
        self.runner.pump(platform);
        self.sync();
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        self.runner.pump(platform);
        self.sync();
        // One frame to get started, then only what the runner asks for. See the
        // module docs: a settled probe window should be idle, not spinning.
        if self.frames == 0 {
            self.runner.request_frame(platform);
        }
    }
}

impl ProbeApp {
    /// Applies a pending request and republishes the status line.
    fn sync(&mut self) {
        self.frames = self.frames.wrapping_add(1);

        let surfaces = match self.surfaces {
            Some(surfaces) => surfaces,
            None => {
                let Some(found) = find_surfaces(&self.runner) else {
                    return;
                };
                // The one diagnostic worth having: a window that opens but
                // mounts nothing looks exactly like a window that works.
                eprintln!("probe: all four surfaces mounted (frame {})", self.frames);
                self.surfaces = Some(found);
                found
            }
        };

        let want = self.request.load(Ordering::SeqCst);
        if want != self.applied {
            let host = self.runner.host_mut();
            if let Some(dialog) = host.widget_mut::<Dialog>(surfaces.dialog) {
                dialog.set_present(want == DIALOG);
            }
            if let Some(popup) = host.widget_mut::<Popup>(surfaces.popup) {
                popup.set_present(want == POPUP);
            }
            if let Some(sheet) = host.widget_mut::<BottomSheet>(surfaces.sheet) {
                sheet.set_present(want == SHEET);
            }
            if let Some(drawer) = host.widget_mut::<NavigationDrawer>(surfaces.drawer) {
                drawer.set_present(want == DRAWER);
            }
            self.applied = want;
        }

        let status = match self.status {
            Some(node) => node,
            None => {
                let Some(found) = find_status(&self.runner) else {
                    return;
                };
                self.status = Some(found);
                found
            }
        };
        if let Some(text) = self.runner.host_mut().widget_mut::<Text>(status) {
            text.set_content(format!("status: showing {}", name_of(self.applied)));
        }
    }
}

fn main() {
    let mut host = WidgetHost::new();
    let request = Arc::new(AtomicUsize::new(NONE));

    // Builders are replayable `Fn`s: each call builds a fresh widget, so the
    // flag is cloned *inside* the closure rather than moved out of it.
    let r_dialog = Arc::clone(&request);
    let r_popup = Arc::clone(&request);
    let r_sheet = Arc::clone(&request);
    let r_drawer = Arc::clone(&request);
    let r_close = Arc::clone(&request);
    let r_box = Arc::clone(&request);

    let root = Stack::new(Axis::Vertical)
        .gap(10.0)
        .push(|| Box::new(Text::new("floem2 device probe").size(18.0)))
        // Republished every frame by `sync`: which surface is up.
        .push(|| Box::new(Text::new("status: showing none").size(13.0)))
        // Two fields, so focus can be moved between two composition owners
        // (R1-4: only one composition may ever be live).
        .push_layout(LayoutRequest::new().width(240.0).height(28.0), || {
            Box::new(TextField::new().placeholder("IME field A — type 你好 here"))
        })
        .push_layout(LayoutRequest::new().width(240.0).height(28.0), || {
            Box::new(TextField::new().placeholder("IME field B — tab between the two"))
        })
        .push_layout(LayoutRequest::new().width(240.0).height(28.0), || {
            Box::new(Select::new(["alpha", "beta", "gamma"]))
        })
        .push(move || {
            Box::new(
                Stack::new(Axis::Horizontal)
                    .gap(8.0)
                    .push({
                        let flag = r_dialog.clone();
                        move || {
                            Box::new(Button::new("dialog").on_click({
                                let flag = flag.clone();
                                move || flag.store(DIALOG, Ordering::SeqCst)
                            }))
                        }
                    })
                    .push({
                        let flag = r_popup.clone();
                        move || {
                            Box::new(Button::new("popup").on_click({
                                let flag = flag.clone();
                                move || flag.store(POPUP, Ordering::SeqCst)
                            }))
                        }
                    })
                    .push({
                        let flag = r_sheet.clone();
                        move || {
                            Box::new(Button::new("sheet").on_click({
                                let flag = flag.clone();
                                move || flag.store(SHEET, Ordering::SeqCst)
                            }))
                        }
                    })
                    .push({
                        let flag = r_drawer.clone();
                        move || {
                            Box::new(Button::new("drawer").on_click({
                                let flag = flag.clone();
                                move || flag.store(DRAWER, Ordering::SeqCst)
                            }))
                        }
                    })
                    .push({
                        let flag = r_close.clone();
                        move || {
                            Box::new(Button::new("close").on_click({
                                let flag = flag.clone();
                                move || flag.store(NONE, Ordering::SeqCst)
                            }))
                        }
                    }),
            )
        })
        // The clip case: a 160×48 box that cannot possibly contain a popup.
        // Anything the popup paints outside this box proves it left the
        // parent's layout box (R2-2); the same button drives R2-3.
        .push_layout(LayoutRequest::new().width(160.0).height(48.0), move || {
            let flag = r_box.clone();
            Box::new(
                Stack::new(Axis::Horizontal)
                    .gap(6.0)
                    .push(move || {
                        Box::new(Button::new("popup →").on_click({
                            let flag = flag.clone();
                            move || flag.store(POPUP, Ordering::SeqCst)
                        }))
                    })
                    .push(|| {
                        Box::new(Popup::new().label("probe popup").content(Child::new(
                            ChildKey::index(0),
                            LayoutRequest::new().width(180.0).height(96.0),
                            || Box::new(Text::new("this popup left the small box")),
                        )))
                    }),
            )
        })
        .push(|| {
            Box::new(Dialog::new().title("modal dialog").body(Child::new(
                ChildKey::index(0),
                LayoutRequest::new(),
                || {
                    Box::new(
                        Stack::new(Axis::Vertical)
                            .gap(8.0)
                            .push(|| Box::new(Text::new("the scrim must not dismiss me"))),
                    )
                },
            )))
        })
        .push(|| {
            Box::new(
                BottomSheet::new()
                    .title("bottom sheet")
                    .height(180.0)
                    .body(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
                        Box::new(Text::new("I slide up from the bottom edge"))
                    })),
            )
        })
        .push(|| {
            Box::new(NavigationDrawer::new().header("probe").destinations([
                NavDestination::new("first"),
                NavDestination::new("second"),
                NavDestination::new("third"),
            ]))
        });

    host.mount_root(LayoutStyle::default(), root);

    let presenter = VelloGpuPresenter::new().unwrap_or_else(|| {
        eprintln!("no usable GPU adapter; this example needs a GPU presenter");
        std::process::exit(1);
    });
    println!("GPU adapter: {}", presenter.adapter_info());

    let app = ProbeApp {
        runner: AppRunner::new(host, presenter),
        request,
        applied: NONE,
        surfaces: None,
        status: None,
        frames: 0,
    };

    let windows = vec![WindowConfig {
        title: "floem2 — device probe".into(),
        size: Size::new(520.0, 560.0),
        resizable: true,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}

/// Finds the four surfaces: the first mounted widget of each type.
fn find_surfaces(runner: &AppRunner<VelloGpuPresenter>) -> Option<Surfaces> {
    let host = runner.host();
    let mut found = (None, None, None, None);
    for index in 0..host.node_count() {
        let Some(node) = host.node_at(index) else {
            continue;
        };
        if !host.is_mounted(node) {
            continue;
        }
        if found.0.is_none() && host.widget::<Dialog>(node).is_some() {
            found.0 = Some(node);
        } else if found.1.is_none() && host.widget::<Popup>(node).is_some() {
            found.1 = Some(node);
        } else if found.2.is_none() && host.widget::<BottomSheet>(node).is_some() {
            found.2 = Some(node);
        } else if found.3.is_none() && host.widget::<NavigationDrawer>(node).is_some() {
            found.3 = Some(node);
        }
    }
    match found {
        (Some(dialog), Some(popup), Some(sheet), Some(drawer)) => Some(Surfaces {
            dialog,
            popup,
            sheet,
            drawer,
        }),
        _ => None,
    }
}

/// Finds the status label: the `Text` whose content starts with "status:".
fn find_status(runner: &AppRunner<VelloGpuPresenter>) -> Option<NodeId> {
    let host = runner.host();
    for index in 0..host.node_count() {
        if let Some(node) = host.node_at(index) {
            if let Some(text) = host.widget::<Text>(node) {
                if text.content().starts_with("status:") {
                    return Some(node);
                }
            }
        }
    }
    None
}
