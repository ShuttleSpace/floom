// SPDX-License-Identifier: MIT
//! OpenHarmony driven by Rust: the loop is ours, the window stays ArkUI's.
//!
//! Until now the OHOS build had exactly one path — `floom-platform-harmony`'s
//! N-API surface, where the ArkTS host owns the frame clock and calls
//! `floom_harmony_present_frame` with whatever the tree last painted. That path
//! has no input: the crate exports three functions (bind, present, release) and
//! nothing that carries a finger. Its `capabilities()` still advertises
//! `TOUCH`, so the contract and the code disagreed.
//!
//! This driver is the other half of the choice recorded in `docs/platforms.md`:
//! the Rust side owns the event loop (`floom-winit-ohos`), ArkUI keeps owning
//! the window, and the C++ bridge forwards the XComponent callbacks into
//! `ohos_winit_runtime_*`. Fingers arrive as pointer events, which is what the
//! floom runtime already understands.
//!
//! Two things are deliberately not here yet, both recorded in
//! `docs/progress.md` under S1.2 rather than pretended:
//!
//! - **Keys.** A soft keyboard needs `Ime::Enabled`/`Preedit`/`Commit`, and the
//!   vendored event loop answers `ImeRequest::Update` with `NotSupported`
//!   (`event_loop.rs:1090`). Hardware key events *are* delivered by the OS, but
//!   mapping them without a text path would let a user type into a field that
//!   never shows the characters.
//! - **High-DPI text.** `ScaleFactorChanged` is honoured for input and layout;
//!   the font stack question (no Han face found on the device) is the same one
//!   iOS answers in S1.3.

use std::ffi::c_void;

use floom_app::{AppRunner, WidgetHost};
use floom_platform::{
    HostPlatform, Platform, PlatformEvent, PointerButton as FloomPointerButton, WindowConfig,
    WindowId,
};
use floom_renderer_software::SoftwarePresenter;
use floom_winit_ohos::{
    ApplicationHandler, ButtonSource, CoreWindow, ElementState, MouseButton, PointerSource,
    RootActiveEventLoop, WindowAttributes, WindowEvent, WindowExtOhos, WinitWindowId,
};

use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

/// Build the handler the runtime pumps.
///
/// Called by `ohos_winit_runtime_new` on the ArkTS thread and moved to the
/// runtime's worker before it runs, which is where the tree is actually built:
/// a floom tree owns `Rc`s and raw handles from the moment it exists, so it can
/// only be born where it will live.
pub fn handler(
    request: AppRequest,
    build: BuildFn,
) -> Box<dyn floom_winit_ohos::ffi::RuntimeApplication> {
    Box::new(OhosWinit {
        mounted: Some((request, build)),
        platform: HostPlatform::new().with_capabilities(floom_platform_harmony::capabilities()),
        window: None,
        native: None,
        runner: None,
        scale: 1.0,
    })
}

struct OhosWinit {
    mounted: Option<(AppRequest, BuildFn)>,
    platform: HostPlatform,
    window: Option<WindowId>,
    /// The ArkUI window this driver presents into; `None` before
    /// `can_create_surfaces` and after `destroy_surfaces`.
    native: Option<Box<dyn CoreWindow>>,
    runner: Option<AppRunner<SoftwarePresenter>>,
    scale: f64,
}

impl OhosWinit {
    fn native_ptr(&self) -> *mut c_void {
        self.native
            .as_ref()
            .map(|window| window.native_window_ptr())
            .unwrap_or(std::ptr::null_mut())
    }

    /// Mount the tree into a platform window of `size` logical pixels, once.
    ///
    /// The size is the surface's, not the request's: ArkUI decides how big the
    /// XComponent is, and a tree laid out for a size the window does not have
    /// would clip. The request only supplies the title and resizability.
    fn mount(&mut self, size: floom_types::Size) {
        let Some((request, build)) = self.mounted.take() else {
            return;
        };
        let window = self.platform.create_window(WindowConfig {
            title: request.title,
            size,
            resizable: request.resizable,
        });
        self.window = Some(window);
        let mut host = WidgetHost::new();
        build(&mut host);
        self.runner = Some(AppRunner::new(host, SoftwarePresenter::new()));
        // One frame is owed even if the tree is static: nothing has been painted
        // yet, and the surface is black until something is.
        self.platform.request_frame(window);
    }

    /// Render whatever the tree has and blit it into the ArkUI window.
    fn paint(&mut self) {
        let Some(window) = self.window else {
            return;
        };
        self.platform.request_frame(window);
        let native = self.native_ptr();
        let Some(runner) = self.runner.as_mut() else {
            return;
        };
        runner.pump(&mut self.platform);
        let presenter = runner.presenter();
        let (width, height) = (presenter.width(), presenter.height());
        if native.is_null() || width == 0 || height == 0 {
            return;
        }
        // The handle is the ArkUI surface this driver was handed and has not
        // been released (it is cleared in `destroy_surfaces`), which is exactly
        // `present_into_window`'s contract.
        floom_platform_harmony::present_into_window(native, presenter.pixels(), width, height);
    }
}

impl ApplicationHandler for OhosWinit {
    fn can_create_surfaces(&mut self, event_loop: &dyn RootActiveEventLoop) {
        if self.native.is_some() {
            return;
        }
        match event_loop.create_window(WindowAttributes::default()) {
            Ok(window) => self.native = Some(window),
            Err(error) => {
                floom_winit_ohos::deveco_log_with_level(
                    floom_winit_ohos::OhosLogLevel::Error,
                    format!("the OHOS window could not be created: {error}"),
                );
                event_loop.exit();
            }
        }
    }

    fn destroy_surfaces(&mut self, _event_loop: &dyn RootActiveEventLoop) {
        // The surface is gone: drop the handle so a late redraw cannot blit
        // into a released window, and keep the tree so a recreated surface
        // repaints it instead of starting over.
        self.native = None;
        self.runner = None;
    }

    fn window_event(
        &mut self,
        _event_loop: &dyn RootActiveEventLoop,
        _id: WinitWindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window else {
            // Before the first `SurfaceResized` there is no platform window to
            // address; the size arrives with that event, and the mount happens
            // there.
            if let WindowEvent::SurfaceResized(size) = event {
                let logical = floom_types::Size::new(
                    size.width as f64 / self.scale,
                    size.height as f64 / self.scale,
                );
                self.mount(logical);
                self.paint();
            }
            return;
        };
        match event {
            WindowEvent::SurfaceResized(size) => {
                let logical = floom_types::Size::new(
                    size.width as f64 / self.scale,
                    size.height as f64 / self.scale,
                );
                self.platform.push_event(PlatformEvent::WindowResized {
                    window,
                    size: logical,
                });
                self.paint();
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                self.scale = scale_factor;
                self.platform.set_scale_factor(window, scale_factor);
            }
            WindowEvent::PointerMoved {
                position,
                source,
                primary,
                ..
            } => {
                // Fingers and mice both land here: winit's OHOS backend maps
                // `OH_NativeXComponent` touch points onto pointer events, and
                // the floom runtime has no separate touch path.
                if primary || matches!(source, PointerSource::Mouse) {
                    self.platform.push_event(PlatformEvent::PointerMoved {
                        window,
                        position: floom_types::Point::new(
                            position.x / self.scale,
                            position.y / self.scale,
                        ),
                        modifiers: Default::default(),
                    });
                }
            }
            WindowEvent::PointerButton {
                state,
                position,
                button,
                ..
            } => {
                let Some(button) = floom_button(button) else {
                    return;
                };
                self.platform.push_event(PlatformEvent::PointerButton {
                    window,
                    position: floom_types::Point::new(
                        position.x / self.scale,
                        position.y / self.scale,
                    ),
                    button,
                    pressed: state == ElementState::Pressed,
                    modifiers: Default::default(),
                });
                self.paint();
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    floom_winit_ohos::MouseScrollDelta::LineDelta(x, y) => {
                        (x as f64 * 40.0, y as f64 * 40.0)
                    }
                    floom_winit_ohos::MouseScrollDelta::PixelDelta(p) => (p.x, p.y),
                };
                self.platform.push_event(PlatformEvent::Scroll {
                    window,
                    position: floom_types::Point::new(0.0, 0.0),
                    delta_x: dx,
                    delta_y: dy,
                    modifiers: Default::default(),
                });
                self.paint();
            }
            WindowEvent::Focused(focused) => {
                self.platform
                    .push_event(PlatformEvent::WindowFocused { window, focused });
            }
            WindowEvent::Occluded(hidden) => {
                if hidden {
                    self.platform.suspend();
                } else {
                    self.platform.resume();
                    self.platform.request_frame(window);
                }
            }
            WindowEvent::RedrawRequested => self.paint(),
            WindowEvent::Destroyed | WindowEvent::CloseRequested => {
                self.native = None;
                self.runner = None;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &dyn RootActiveEventLoop) {
        // ArkUI calls `ohos_winit_runtime_frame` once per vsync; this is where
        // that turn becomes a redraw request. Asking only when the tree says it
        // is still moving is what keeps an idle app at zero frames instead of
        // repainting the same pixels sixty times a second.
        if let (Some(window), true) = (self.native.as_ref(), self.platform.wants_frame()) {
            window.request_redraw();
        }
    }

    fn suspended(&mut self, _event_loop: &dyn RootActiveEventLoop) {
        self.platform.suspend();
    }

    fn memory_warning(&mut self, _event_loop: &dyn RootActiveEventLoop) {
        // Nothing to release yet: the software presenter holds one frame
        // buffer, and dropping the tree would lose state the user can see.
        floom_winit_ohos::deveco_log_with_level(
            floom_winit_ohos::OhosLogLevel::Info,
            "memory warning: the software presenter holds one frame".to_string(),
        );
    }
}

/// winit's button source → the runtime's three-button space.
///
/// A stylus or an unmapped touch button is dropped rather than guessed at: the
/// tree would see a button it never bound.
fn floom_button(source: ButtonSource) -> Option<FloomPointerButton> {
    match source {
        ButtonSource::Mouse(MouseButton::Left) => Some(FloomPointerButton::Primary),
        ButtonSource::Mouse(MouseButton::Right) => Some(FloomPointerButton::Secondary),
        ButtonSource::Mouse(MouseButton::Middle) => Some(FloomPointerButton::Auxiliary),
        ButtonSource::Touch { .. } => Some(FloomPointerButton::Primary),
        _ => None,
    }
}

/// The backend this driver installs into the facade's compile-time cascade.
///
/// `run` returns as soon as the loop is up: on OHOS the process lifetime is the
/// OS's (the HAP's `EntryAbility` destroys the runtime), so blocking the caller
/// would freeze the ArkTS UI thread that has to keep delivering input.
pub struct OhosWinitBackend;

impl PlatformBackend for OhosWinitBackend {
    fn name(&self) -> &'static str {
        "harmony (ArkUI + winit-ohos)"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        floom_platform_harmony::capabilities()
    }

    fn run(&self, _request: AppRequest, _build: BuildFn) -> Result<(), String> {
        // The event loop belongs to the ArkTS entry: ArkUI calls
        // `ohos_winit_runtime_new` and drives the surface. A runtime started
        // from here would be a second loop nobody pumps, so this path refuses
        // instead of painting into a window that never appears.
        Err(
            "on OpenHarmony with the `harmony-winit` feature the app is started by ArkUI: \
             export `floom::ohos_app!(your_build_fn)` and call `floom_ohos_start()` from \
             the XComponent, instead of calling floom::run"
                .to_string(),
        )
    }
}
