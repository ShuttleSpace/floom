// SPDX-License-Identifier: MIT
//! Floom 2.0 **device probe**: one window holding every control the on-screen
//! cases in `.superpowers/sdd/device-verification-2026-09-15.md` need.
//!
//! `examples/floom-window` is deliberately a 222-line proof that the pipeline
//! runs — a counter, a label, a gradient. It contains no text field, no
//! `Dialog`, no `Select`, so none of the R1/R2 cases (real IME candidate window,
//! popups escaping their parent's box, light dismiss, the entry animation,
//! focus under a modal) could be driven against it. This example exists to be
//! driven:
//!
//! ```sh
//! cargo run -p floom-probe --features real
//! ```
//!
//! What is in the window, and which case wants it:
//!
//! | control | case |
//! | --- | --- |
//! | two `TextInput`s | R1-1…R1-4 (composition, candidate window, moving focus) |
//! | a `Select` | R2-1 (the reference: what an escaping popup looks like) |
//! | `Dialog` (modal) | R2-4 (scrim must not dismiss), R2-8 (keyboard under a modal) |
//! | `Popup` (non-modal) | R2-3 (click outside dismisses), R2-6 (fade in, then still) |
//! | `Popup` inside a 160×48 box | R2-2 (it must be drawn outside that box) |
//! | `BottomSheet` | R2-6 (slide up from the bottom edge) |
//! | `NavigationDrawer` | R2-7 (leading-edge slide, stacked above a popup) |
//! | `NativeSurface` + "transition" | D11 (a frozen, animating texture, then live again) |
//!
//! D11 is the one case that can also be fired without a click, because its
//! whole point is a 0.15 s tween nobody can aim a mouse at:
//!
//! ```sh
//! FLOOM2_PROBE_TRANSITION_EVERY=90 cargo run -p floom-probe --features real
//! ```
//!
//! re-fires the transition every 90 frames. Off by default, so the window is
//! still idle when G1-4 is being observed.
//!
//! Unlike `floom-window`, this app does **not** ask for a frame on every
//! `about_to_wait`. It asks for one frame to get started and then runs only when
//! something actually happened — an event, or a widget owing a frame (the entry
//! animation, for instance). That is the behaviour G1-4 exists to observe, and
//! it is why a settled probe window should be idle rather than spinning.

#![cfg(feature = "real")]

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use floom_animation::{AnimKind, CompositorProp, Easing, Tween};
use floom_app::{AppRunner, NodeId, WidgetHost};
use floom_core::{Axis, Child, ChildKey, LayoutRequest};
use floom_layout::LayoutStyle;
use floom_platform::{Capability, Platform, WindowConfig};
use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
use floom_renderer_vello::VelloGpuPresenter;
use floom_types::{NativeViewKind, Size, WindowId};
use floom_ui_composites::{BottomSheet, Dialog, NavDestination, NavigationDrawer, Popup, Select};
use floom_widgets::{Button, NativeSurface, Stack, Text, TextInput};

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
    /// How many times "transition" has been pressed. A counter rather than a
    /// flag, because the failures worth finding are the ones that only show on
    /// the second attempt — a participant left frozen, or a view that never
    /// came back.
    transitions: Arc<AtomicUsize>,
    /// How many of those have actually been started.
    fired: usize,
    /// Frames between automatic transitions. `0` = never, which is the default:
    /// a window that fires by itself is not the window G1-4 wants to watch.
    auto_every: u32,
    /// Resolved lazily: the composite children mount during the first frame.
    surfaces: Option<Surfaces>,
    status: Option<NodeId>,
    frames: u32,
    /// The window's content view, once a native-view provider has been
    /// installed. Kept so the *OS* can be asked where the participant landed:
    /// asking the bridge would return the bridge's own story, which is the one
    /// answer a device check is not allowed to trust.
    #[cfg(target_os = "macos")]
    parent: Option<floom_platform_desktop::NativeParent>,
    /// The view tree as last printed, so a *change* is what triggers the next
    /// line rather than a frame count that depends on how the loop is batched.
    #[cfg(target_os = "macos")]
    reported: Option<Vec<floom_platform_desktop::ViewFrame>>,
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
    /// Native composition is installed here, and this is the only hook that can
    /// do it: the provider needs the window's content view, and the window does
    /// not exist until the OS has created it.
    ///
    /// Installing a provider is also what flips `Capability::NATIVE_VIEW` in
    /// `Platform::capabilities`, which is why the two are done together and the
    /// result is printed: a window that mounts nothing looks exactly like a
    /// window that works, and this is the line that tells them apart.
    fn window_created(&mut self, platform: &mut WinitPlatform, window: WindowId) {
        #[cfg(target_os = "macos")]
        {
            match platform.native_view_provider(window) {
                Ok(provider) => {
                    self.runner
                        .host_mut()
                        .register_native_provider(provider, &[NativeViewKind::Video])
                        .ok();
                    self.parent = platform.native_view_parent(window).ok();
                    eprintln!(
                        "probe: native views installed, NATIVE_VIEW {}",
                        platform.capabilities().supports(Capability::NATIVE_VIEW)
                    );
                }
                Err(err) => eprintln!("probe: no native views ({err})"),
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            // No provider on this platform yet, so `NATIVE_VIEW` stays off and
            // the participant is refused — which is the honest outcome and the
            // same code path as a macOS session whose install failed.
            let _ = (platform, window);
        }
    }

    fn events(&mut self, platform: &mut WinitPlatform) {
        self.runner.pump(platform);
        self.sync();
    }

    fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
        self.runner.pump(platform);
        self.sync();
        // One frame to get started, then only what the runner asks for. See the
        // module docs: a settled probe window should be idle, not spinning.
        //
        // The exception is an auto-firing D11 run, which has to keep the loop
        // alive to reach its next trigger — and which is opt-in precisely
        // because it is the one mode that is not idle.
        if self.frames == 0 || self.auto_every > 0 {
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

        self.report_native_tree();

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

        self.fire_transitions();

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

    /// Starts any transition the "transition" button — or the auto-fire env
    /// var — has asked for, and prints what came back.
    ///
    /// The print *is* the evidence. D11 asks whether a native participant can
    /// be pushed as a frozen, animating texture, and on a real window the first
    /// thing to establish is which of the three answers this platform gives:
    /// a snapshot arrived (`Some`), the provider cannot take one (`None` — a
    /// contract, not a failure, and the honest answer for a backend with no
    /// snapshotter), or the host refused the call outright (`Err`).
    ///
    /// Each of those has a different observable consequence, and the log has to
    /// say which one happened: only the first hides the live view.
    fn fire_transitions(&mut self) {
        if self.auto_every > 0 && self.frames.is_multiple_of(self.auto_every) {
            self.transitions.fetch_add(1, Ordering::SeqCst);
        }
        let wanted = self.transitions.load(Ordering::SeqCst);
        if wanted == self.fired {
            return;
        }
        let Some(slot) = self.runner.host().participant_slots().into_iter().next() else {
            // Nothing bound yet is not "no snapshot": come back on a later
            // frame rather than report a verdict about an empty stage.
            return;
        };
        self.fired = wanted;
        let anim = AnimKind::Tween(
            Tween::new(-160.0, 0.0, 0.6)
                .with_easing(Easing::EASE_OUT)
                .with_compositor(CompositorProp::TranslateX(-160.0)),
        );
        match self.runner.host_mut().begin_native_transition(slot, anim) {
            Ok(Some(snap)) => eprintln!(
                "probe: transition {}: FROZEN — resource {:?}, {}x{} px, {} frozen total",
                self.fired,
                snap.resource,
                snap.pixel_width,
                snap.pixel_height,
                self.runner.host().frozen_participants().len(),
            ),
            Ok(None) => eprintln!(
                "probe: transition {}: NO SNAPSHOT — provider cannot freeze, so nothing is \
                 animated and the live view stays live ({} frozen)",
                self.fired,
                self.runner.host().frozen_participants().len(),
            ),
            Err(err) => eprintln!("probe: transition {}: REFUSED — {err}", self.fired),
        }
        // Force the OS view tree to be reprinted next frame: whether the live
        // view got hidden is the observable difference between the two `Ok`
        // answers, and it is the OS that has to be asked.
        #[cfg(target_os = "macos")]
        {
            self.reported = None;
        }
    }

    /// Prints the OS's own view tree under the window, whenever it changes.
    ///
    /// The left-hand side of D1's assertion: the framework computes where a
    /// participant goes, and this is what the OS ended up with. A device check
    /// reads them against each other, so what matters is that these numbers come
    /// from `NSView::frame` and not from our own bookkeeping.
    #[cfg(target_os = "macos")]
    fn report_native_tree(&mut self) {
        let Some(parent) = self.parent.as_ref() else {
            return;
        };
        let subviews = parent.subview_frames();
        if self.reported.as_ref() == Some(&subviews) {
            return;
        }
        let (width, height) = parent.extent();
        eprintln!(
            "probe: content view {width}x{height} points, flipped {}",
            parent.is_flipped()
        );
        for (index, view) in subviews.iter().enumerate() {
            eprintln!(
                "probe:   native view {index}: frame {:?} points, hidden {}, children {}",
                view.frame, view.hidden, view.children
            );
        }
        // The right-hand side of the same assertion. Printed next to the frames
        // above on purpose: the seam case is a claim about *agreement* between
        // these two, so seeing one without the other is not evidence.
        for (slot, state) in self.runner.host().participant_states() {
            eprintln!(
                "probe:   framework state: slot {} gen {} rect ({}, {})..({}, {}) scale {} visible {}",
                slot.id.index(),
                slot.generation,
                state.rect.x0,
                state.rect.y0,
                state.rect.x1,
                state.rect.y1,
                state.scale,
                state.visible,
            );
            // The same box in the units the line above does *not* use, so the
            // two halves of the assertion can be compared without arithmetic.
            // `points` is what an AppKit frame has to say to land on the device
            // rect the provider was handed, and it is what `NSView::frame`
            // reported above: agreeing is the claim, and a shell script can
            // check it where eyes cannot.
            let device = state.device_rect();
            let scale = state.scale;
            let points = (
                device.x as f64 / scale,
                device.y as f64 / scale,
                device.width as f64 / scale,
                device.height as f64 / scale,
            );
            eprintln!(
                "probe:   framework expects: frame {points:?} points = ({}, {}, {}, {}) device pixels",
                device.x, device.y, device.width, device.height,
            );
        }
        self.reported = Some(subviews);
    }

    /// Without an OS handle there is nothing to ask the OS.
    #[cfg(not(target_os = "macos"))]
    fn report_native_tree(&mut self) {}
}

fn main() {
    let mut host = WidgetHost::new();
    let request = Arc::new(AtomicUsize::new(NONE));
    let transitions = Arc::new(AtomicUsize::new(0));
    // Off unless asked for: a window that animates by itself cannot be the
    // window G1-4 watches to see whether an idle window spins.
    let auto_every: u32 = std::env::var("FLOOM2_PROBE_TRANSITION_EVERY")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    if auto_every > 0 {
        eprintln!("probe: auto-firing a transition every {auto_every} frames");
    }

    // Builders are replayable `Fn`s: each call builds a fresh widget, so the
    // flag is cloned *inside* the closure rather than moved out of it.
    let r_dialog = Arc::clone(&request);
    let r_popup = Arc::clone(&request);
    let r_sheet = Arc::clone(&request);
    let r_drawer = Arc::clone(&request);
    let r_close = Arc::clone(&request);
    let r_box = Arc::clone(&request);
    let r_transition = Arc::clone(&transitions);

    let root = Stack::new(Axis::Vertical)
        .gap(10.0)
        .push(|| Box::new(Text::new("floom device probe").size(18.0)))
        // Republished every frame by `sync`: which surface is up.
        .push(|| Box::new(Text::new("status: showing none").size(13.0)))
        // Two fields, so focus can be moved between two composition owners
        // (R1-4: only one composition may ever be live).
        .push_layout(LayoutRequest::new().width(240.0).height(28.0), || {
            Box::new(TextInput::with_text("").placeholder("IME field A — type 你好 here"))
        })
        .push_layout(LayoutRequest::new().width(240.0).height(28.0), || {
            Box::new(TextInput::with_text("").placeholder("IME field B — tab between the two"))
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
        // The native participant (Phase 16). On macOS this is a real `NSView`
        // under the window's content view; elsewhere the request is refused and
        // the surface falls back in-tree, which is the honest outcome and the
        // same path a macOS session takes when no provider was installed.
        //
        // Placed between two framework boxes on purpose: the seam case (D1) is
        // about whether the participant's edges agree with the pixels the scene
        // painted next to it, so there has to be something next to it.
        .push(|| Box::new(Text::new("native surface below").size(13.0)))
        .push_layout(LayoutRequest::new().width(200.0).height(80.0), || {
            Box::new(NativeSurface::new(NativeViewKind::Video).size(200.0, 80.0))
        })
        .push(|| Box::new(Text::new("native surface above").size(13.0)))
        // D11's trigger, next to the participant it acts on. The label names the
        // case because this button is the only reason to open the window.
        .push(move || {
            let flag = r_transition.clone();
            Box::new(Button::new("transition (D11)").on_click(move || {
                flag.fetch_add(1, Ordering::SeqCst);
            }))
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
        transitions,
        fired: 0,
        auto_every,
        surfaces: None,
        status: None,
        frames: 0,
        #[cfg(target_os = "macos")]
        parent: None,
        #[cfg(target_os = "macos")]
        reported: None,
    };

    let windows = vec![WindowConfig {
        title: "floom — device probe".into(),
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
        if let Some(node) = host.node_at(index)
            && let Some(text) = host.widget::<Text>(node)
            && text.content().starts_with("status:")
        {
            return Some(node);
        }
    }
    None
}
