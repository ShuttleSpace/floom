// SPDX-License-Identifier: MIT
//! The one platform door.
//!
//! An app calls [`run`](crate::run) and, when it needs to know what the
//! compiled-in platform can do, [`require`](crate::platform::require). It never names a backend, never
//! chooses between `run_desktop` / `run_android` / `run_web`, and never links a
//! backend crate directly.
//!
//! # Why a trait
//!
//! Every backend (`winit` desktop, Android, web, terminal, embedded) already
//! implements [`floom_platform::Platform`], but that trait is the *backend*
//! contract: it creates surfaces and polls events, and a caller that held one
//! would be writing the app loop itself. [`PlatformBackend`](crate::platform::PlatformBackend) is the *app*
//! contract — a name, a capability set, and one `run` that owns the loop — so
//! the facade can swap backends behind a single call.
//!
//! # What happens when a platform cannot do something
//!
//! A capability that is missing is a policy decision, not an error the caller
//! invents each time. [`require`](crate::platform::require) takes the policy:
//!
//! | Policy | Missing capability means |
//! |---|---|
//! | [`OnUnsupported::Warn`](crate::platform::OnUnsupported::Warn) | log and carry on without the feature |
//! | [`OnUnsupported::Fail`](crate::platform::OnUnsupported::Fail) | return [`PlatformError`](crate::platform::PlatformError) to the caller |
//! | [`OnUnsupported::Exit`](crate::platform::OnUnsupported::Exit) | log and `exit(1)` — the app cannot work at all |
//!
//! The default for [`run`](crate::run) is [`OnUnsupported::Fail`](crate::platform::OnUnsupported::Fail): a window that
//! can never paint is not a degraded app, it is a broken one.

use std::fmt;

use floom_app::WidgetHost;
pub use floom_platform::{Capability, PlatformCapabilities};
// The whole `floom-platform` ABI is also reachable as `floom::platform::…`:
// `platform` is a hand-written module (`require`, `current`, the backends), so
// the crate is merged in rather than aliased over it. Local items shadow the
// glob's same-named ones (e.g. our `embedded` backend module), keeping every
// existing path intact.
pub use floom_platform::*;

/// What to do when the compiled-in backend lacks a capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnUnsupported {
    /// Log on stderr and continue without the feature.
    Warn,
    /// Return an error so the caller can adapt.
    Fail,
    /// Log on stderr and terminate: without this capability the app cannot run.
    Exit,
}

/// A capability the backend does not have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformError {
    /// Name of the backend that lacks the capability.
    pub backend: &'static str,
    /// The capability it cannot provide.
    pub capability: Capability,
}

impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "platform backend `{}` does not support `{:?}`",
            self.backend, self.capability
        )
    }
}

impl std::error::Error for PlatformError {}

/// The app loop's request: one window, described without a backend in sight.
#[derive(Debug, Clone)]
pub struct AppRequest {
    /// Window title shown by the backend.
    pub title: String,
    /// Initial window size.
    pub size: floom_types::Size,
    /// Whether the user may resize the window.
    pub resizable: bool,
}

/// The mount closure handed to a backend, type-erased so the trait stays
/// object-safe. `Box<dyn FnOnce(..)>` is callable, so backends just call it.
pub type BuildFn = Box<dyn FnOnce(&mut WidgetHost) + 'static>;

/// The app-facing platform contract: every backend implements this.
pub trait PlatformBackend: Send + Sync {
    /// A human-readable backend name, used in diagnostics.
    fn name(&self) -> &'static str;

    /// What this backend can do.
    fn capabilities(&self) -> PlatformCapabilities;

    /// Run the app loop until the app quits.
    ///
    /// The backend owns window creation, event dispatch and the frame loop;
    /// `build` is called once, when the window exists, to mount the tree.
    fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String>;

    /// Whether `cap` is available.
    fn supports(&self, cap: Capability) -> bool {
        self.capabilities().supports(cap)
    }

    /// Check `cap`, applying `policy` when the backend lacks it.
    fn require(&self, cap: Capability, policy: OnUnsupported) -> Result<(), PlatformError> {
        if self.supports(cap) {
            return Ok(());
        }
        let error = PlatformError {
            backend: self.name(),
            capability: cap,
        };
        match policy {
            OnUnsupported::Warn => {
                eprintln!("warning: {error}; continuing without it");
                Ok(())
            }
            OnUnsupported::Fail => Err(error),
            OnUnsupported::Exit => {
                eprintln!("error: {error}");
                std::process::exit(1);
            }
        }
    }
}

/// Check a capability on the compiled-in backend.
pub fn require(cap: Capability, policy: OnUnsupported) -> Result<(), PlatformError> {
    current().require(cap, policy)
}

/// The backend compiled into this binary.
///
/// Selection is compile-time (`#[cfg]`), which is the whole point: one call site
/// works on every target the binary was built for, and a target the binary was
/// *not* built for fails loudly at [`run`](crate::run) instead of silently
/// doing nothing.
///
/// The cascade mirrors the targets the backends target: a browser tab (wasm +
/// `web`), an Android surface (`mobile` + target `android`), a desktop GPU
/// window (`desktop` + `renderer-gpu`), a terminal (`tui`), an embedded board
/// (`embedded`), and the two full mobile/desktop OSes — iOS (`ios` + target
/// `ios`) and HarmonyOS (`harmony` + target_env `ohos`) — which are first-class
/// OS backends, not embedded boards. Each of the last two runs the software
/// renderer and presents into the native window its OS host hands it.
pub fn current() -> &'static dyn PlatformBackend {
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    {
        return &web::WebBackend;
    }
    #[cfg(all(feature = "mobile", feature = "renderer-gpu", target_os = "android"))]
    {
        return &android::AndroidBackend;
    }
    #[cfg(all(
        feature = "desktop",
        feature = "renderer-gpu",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    {
        return &desktop::DesktopBackend;
    }
    #[cfg(all(
        feature = "tui",
        not(target_arch = "wasm32"),
        not(target_os = "android")
    ))]
    #[allow(unreachable_code)]
    {
        return &tui::TuiBackend;
    }
    // An embedded board: a framebuffer and the software rasteriser. It runs
    // anywhere with no window manager and no GPU, but iOS and HarmonyOS are
    // *not* embedded boards — they have real window systems and their own
    // backends above — so this fallback explicitly excludes them.
    #[cfg(all(
        feature = "embedded",
        not(target_arch = "wasm32"),
        not(target_os = "android"),
        not(target_os = "ios"),
        not(target_env = "ohos")
    ))]
    #[allow(unreachable_code)]
    {
        return &embedded::EmbeddedBackend;
    }
    // iOS and HarmonyOS are full OSes with their own feature bundles and
    // siblings, ahead of the generic embedded fallback (which must not swallow
    // them: it is gated on `not ios/ohos` below).
    #[cfg(all(feature = "ios", target_os = "ios"))]
    #[allow(unreachable_code)]
    {
        return &ios::IosBackend;
    }
    // `harmony-winit` puts the event loop in Rust; plain `harmony` leaves the
    // frame clock with the ArkTS host. Both are real builds, so the newer one
    // wins only when it was asked for.
    #[cfg(all(feature = "harmony-winit", target_env = "ohos"))]
    #[allow(unreachable_code)]
    {
        return &ohos_winit::OhosWinitBackend;
    }
    #[cfg(all(feature = "harmony", target_env = "ohos"))]
    #[allow(unreachable_code)]
    {
        return &harmony::HarmonyBackend;
    }
    // No backend matches this build: the right feature bundle is off.
    #[allow(unreachable_code)]
    {
        &NO_BACKEND
    }
}

/// The stand-in when no backend feature is enabled.
///
/// Compiled even when a real backend is selected so the capability tests can
/// exercise the policy against a backend that has nothing.
#[cfg_attr(all(feature = "desktop", feature = "renderer-gpu"), allow(dead_code))]
struct NoBackend;

impl PlatformBackend for NoBackend {
    fn name(&self) -> &'static str {
        "none"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::default()
    }

    fn run(&self, _request: AppRequest, _build: BuildFn) -> Result<(), String> {
        Err(
            "no platform backend compiled in: rebuild with a backend bundle, \
             e.g. `--features desktop,renderer-gpu`"
                .to_string(),
        )
    }
}

#[cfg_attr(all(feature = "desktop", feature = "renderer-gpu"), allow(dead_code))]
static NO_BACKEND: NoBackend = NoBackend;

#[cfg(all(feature = "desktop", feature = "renderer-gpu"))]
mod desktop {
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform::WindowConfig;
    use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run as winit_run};
    use floom_renderer_vello::VelloGpuPresenter;

    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    pub struct DesktopBackend;

    impl PlatformBackend for DesktopBackend {
        fn name(&self) -> &'static str {
            "desktop (winit + vello)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities::desktop_default()
        }

        fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String> {
            struct ClosureApp {
                build: Option<BuildFn>,
                runner: Option<AppRunner<VelloGpuPresenter>>,
            }

            impl WinitApp for ClosureApp {
                fn window_created(
                    &mut self,
                    _platform: &mut WinitPlatform,
                    _window: floom_types::WindowId,
                ) {
                    let Some(build) = self.build.take() else {
                        return;
                    };
                    let mut host = WidgetHost::new();
                    build(&mut host);
                    // `WinitApp` has no error channel, so a machine with no
                    // usable adapter is reported on stderr rather than
                    // panicking inside the event loop.
                    let Some(presenter) = VelloGpuPresenter::new() else {
                        eprintln!(
                            "floom::run: no usable GPU adapter; a desktop window needs one to paint"
                        );
                        return;
                    };
                    self.runner = Some(AppRunner::new(host, presenter));
                }

                fn events(&mut self, platform: &mut WinitPlatform) {
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }

                fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
                    // Drain once more so redraws the runner requested during the
                    // last `events` call are picked up before the loop blocks.
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }
            }

            winit_run(
                ClosureApp {
                    build: Some(build),
                    runner: None,
                },
                vec![WindowConfig {
                    title: request.title,
                    size: request.size,
                    resizable: request.resizable,
                }],
            )
        }
    }
}

/// Terminal backend: the `floom-platform-tui` run loop behind the same door.
#[cfg(all(
    feature = "tui",
    not(target_arch = "wasm32"),
    not(target_os = "android")
))]
mod tui {
    use std::io::Stdout;

    use floom_app::{AppRunner, WidgetHost};
    use floom_platform_tui::{TuiApp, TuiPlatform};
    use floom_renderer_tui::{TerminalSize, TuiPresenter};

    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    pub struct TuiBackend;

    impl PlatformBackend for TuiBackend {
        fn name(&self) -> &'static str {
            "tui (terminal)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities::tui_default()
        }

        fn run(&self, _request: AppRequest, build: BuildFn) -> Result<(), String> {
            struct ClosureApp {
                build: Option<BuildFn>,
                runner: Option<AppRunner<TuiPresenter<Stdout>>>,
            }

            impl TuiApp for ClosureApp {
                fn events(&mut self, platform: &mut TuiPlatform) {
                    if let Some(build) = self.build.take() {
                        let mut host = WidgetHost::new();
                        build(&mut host);
                        let size = platform.size();
                        let presenter = TuiPresenter::new(
                            TerminalSize::new(size.cols, size.rows),
                            std::io::stdout(),
                        );
                        self.runner = Some(AppRunner::new(host, presenter));
                    }
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }
            }

            floom_platform_tui::run(ClosureApp {
                build: Some(build),
                runner: None,
            })
        }
    }
}

/// Embedded backend: a framebuffer and the software rasteriser.
///
/// The board has no window manager and no GPU: it has a rectangle of memory
/// and a way to receive input. The platform half owns the memory, the
/// software presenter draws into it, and the frame is copied across — which
/// is the embedded equivalent of a swap-chain present.
///
/// Unlike the other backends this one is not driven by an operating system
/// event source, so the loop ends when the board asks it to: the platform
/// counts frames, and the board stops it by closing the window or by asking
/// for an exit.
#[cfg(feature = "embedded")]
pub mod embedded {
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform::{Platform, WindowConfig};
    use floom_platform_embedded::EmbeddedPlatform;
    use floom_renderer_software::SoftwarePresenter;

    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    /// The embedded backend: a framebuffer and the software rasteriser.
    pub struct EmbeddedBackend;

    impl PlatformBackend for EmbeddedBackend {
        fn name(&self) -> &'static str {
            "embedded (framebuffer)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities::embedded_default()
        }

        fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String> {
            let mut platform = EmbeddedPlatform::new();
            platform.create_window(WindowConfig {
                title: request.title,
                size: request.size,
                resizable: request.resizable,
            });
            run_frames(&mut platform, build);
            Ok(())
        }
    }

    /// Run the embedded loop against `platform`, and report the frames served.
    ///
    /// The loop ends when the platform says so: the board closes the window,
    /// asks for an exit, or the frame limit it was given is reached. A board
    /// that has nothing left to show says so with
    /// [`EmbeddedPlatform::request_exit`].
    ///
    /// The frame is copied into the platform's framebuffer after every frame,
    /// which is the embedded equivalent of a swap-chain present: the software
    /// presenter owns a CPU buffer, and this is the step that puts it on the
    /// "screen".
    pub fn run_frames(platform: &mut EmbeddedPlatform, build: BuildFn) -> u64 {
        let mut host = WidgetHost::new();
        build(&mut host);
        // The presenter sizes its buffer when it attaches to the window,
        // which the runner does on the first pump.
        let mut runner = AppRunner::new(host, SoftwarePresenter::new());

        let mut frames = 0u64;
        while platform.count_frame() {
            runner.pump(platform);
            let frame = runner.presenter().pixels();
            if let Some(surface) = platform.framebuffer_mut() {
                surface.present(frame);
            }
            frames += 1;
        }
        frames
    }
}

/// iOS and HarmonyOS backends: full OS platforms, each behind its own feature.
///
/// Both follow the `ohos-rs`/UIKit model: the OS host owns the window and the
/// loop, and hands Rust a native surface (a `UIView` drawable or an ArkUI
/// `XComponent` `OHNativeWindow`). Rust's job is the pixels. The software
/// renderer runs into an in-memory staging platform — the same CPU framebuffer
/// the embedded board uses, which is a rendering detail, not a claim that
/// these OSes are embedded — and every frame is presented into the native
/// surface by the sibling crate's C ABI seam
/// ([`floom_platform_ios::present`] / [`floom_platform_harmony::present`]).
///
/// The loop is bounded by the stage's `count_frame()` the same way the embedded
/// backend is: the OS host stops it by releasing the surface. A build without
/// a bound native surface simply presents nowhere, which is exactly what a
/// headless cross-compile does.
#[cfg(all(feature = "ios", target_os = "ios"))]
mod ios {
    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    /// The iOS backend: UIKit window + software renderer + native present.
    pub struct IosBackend;

    impl PlatformBackend for IosBackend {
        fn name(&self) -> &'static str {
            "ios (UIKit + software)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            floom_platform_ios::capabilities()
        }

        fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String> {
            super::run_os_frames(
                request,
                build,
                floom_platform_ios::IOS_PLATFORM_ID,
                floom_platform_ios::capabilities(),
                floom_platform_ios::present,
            )
        }
    }
}

/// The iOS host-driven path: UIKit owns the run loop and asks for each frame,
/// the way the ArkTS shell does on HarmonyOS. The blocking [`crate::run`] is
/// the wrong door here — a `UIApplicationMain` that never returns cannot be
/// pumped from inside itself.
#[cfg(all(feature = "ios", target_os = "ios"))]
pub mod ios_host {
    use std::cell::RefCell;

    use floom_app::{AppRunner, WidgetHost};
    use floom_platform::{
        HostPlatform, Modifiers, Platform, PlatformEvent, PointerButton, WindowConfig, WindowId,
    };
    use floom_renderer_software::SoftwarePresenter;
    use floom_types::Point;

    use super::{AppRequest, BuildFn};

    /// The live app. Thread-local because UIKit calls every export from its own
    /// thread and none of this is `Send`.
    struct Hosted {
        platform: HostPlatform,
        runner: AppRunner<SoftwarePresenter>,
        window: WindowId,
        /// Device pixels per logical point, as UIKit reported it.
        scale: f64,
    }

    thread_local! {
        static APP: RefCell<Option<Hosted>> = const { RefCell::new(None) };
    }

    /// Build the tree and paint the first frame. `scale` is
    /// `UIScreen.main.scale`: UIKit hands out points, the host wants pixels.
    pub fn start(request: AppRequest, build: BuildFn, scale: f64) -> bool {
        let built = build;
        let mut platform = HostPlatform::new()
            .with_id(floom_platform_ios::IOS_PLATFORM_ID)
            .with_capabilities(floom_platform_ios::capabilities())
            .with_scale_factor(if scale > 0.0 { scale } else { 1.0 })
            // The Swift host installs the sink in `viewDidMoveToWindow`; until
            // then the adapter keeps the tree and publishes nothing.
            .with_accessibility(Box::new(
                floom_platform_ios::accessibility::IosAccessibility::new(),
            ));
        let window = platform.create_window(WindowConfig {
            title: request.title,
            size: request.size,
            resizable: request.resizable,
        });
        let mut host = WidgetHost::new();
        built(&mut host);
        let runner = AppRunner::new(host, SoftwarePresenter::new());
        platform.request_frame(window);
        APP.with(|slot| {
            let mut hosted = Hosted {
                platform,
                runner,
                window,
                scale: if scale > 0.0 { scale } else { 1.0 },
            };
            apply_safe_area(&mut hosted);
            *slot.borrow_mut() = Some(hosted)
        });
        // UIKit reaches the widget tree only through here: the adapter knows the
        // node's id, and the id is resolved by the host that owns it.
        floom_platform_ios::accessibility::set_activate_sink(Box::new(|id| {
            APP.with(|slot| match slot.try_borrow_mut() {
                Ok(mut live) => live
                    .as_mut()
                    .is_some_and(|hosted| hosted.runner.host_mut().activate(id)),
                // A frame is on the stack right now; the activation is dropped
                // rather than queued, because UIKit's caller expects an answer.
                Err(_) => false,
            })
        }));
        pump()
    }

    /// Run one frame: dispatch whatever the host queued, paint, blit.
    /// The insets UIKit reports are a floor on the root's padding, so content
    /// clears the status bar and the home indicator while backgrounds still
    /// paint edge to edge.
    fn apply_safe_area(hosted: &mut Hosted) {
        hosted
            .runner
            .host_mut()
            .set_content_insets(floom_platform_ios::packages::safe_area::safe_area_insets());
    }

    pub fn pump() -> bool {
        APP.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(app) = slot.as_mut() else {
                return false;
            };
            apply_safe_area(app);
            app.runner.pump(&mut app.platform);
            let presenter = app.runner.presenter();
            let (width, height) = (presenter.width(), presenter.height());
            let painted = floom_platform_ios::present(presenter.pixels(), width, height);
            app.platform.request_frame(app.window);
            painted
        })
    }

    /// Forward one UIKit touch. `phase` is 0 down, 1 move, 2 up, 3 cancel;
    /// coordinates are pixels, so the caller multiplies points by the scale.
    pub fn touch(x: f64, y: f64, phase: u32) -> bool {
        APP.with(|slot| {
            let mut slot = slot.borrow_mut();
            let Some(app) = slot.as_mut() else {
                return false;
            };
            // UIKit hands us device pixels; layout and hit testing are in
            // logical points, so the platform's own scale is divided out here.
            let scale = if app.scale > 0.0 { app.scale } else { 1.0 };
            let position = Point::new(x / scale, y / scale);
            let window = app.window;
            app.platform.push_event(PlatformEvent::PointerMoved {
                window,
                position,
                modifiers: Modifiers::empty(),
            });
            if phase == 0 || phase == 2 {
                app.platform.push_event(PlatformEvent::PointerButton {
                    window,
                    position,
                    button: PointerButton::Primary,
                    pressed: phase == 0,
                    modifiers: Modifiers::empty(),
                });
            }
            app.platform.request_frame(window);
            true
        })
    }
}

/// The OHOS driver that owns the event loop in Rust (see the module docs).
#[cfg(all(feature = "harmony-winit", target_env = "ohos"))]
pub mod ohos_winit;

#[cfg(all(feature = "harmony", target_env = "ohos"))]
mod harmony {
    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    /// The HarmonyOS backend: ArkUI XComponent + software renderer + native present.
    pub struct HarmonyBackend;

    impl PlatformBackend for HarmonyBackend {
        fn name(&self) -> &'static str {
            "harmony (ArkUI + software)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            floom_platform_harmony::capabilities()
        }

        fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String> {
            super::run_os_frames(
                request,
                build,
                floom_platform_harmony::HARMONY_PLATFORM_ID,
                floom_platform_harmony::capabilities(),
                floom_platform_harmony::present,
            )
        }
    }
}

/// The frame pump shared by the two OS backends.
///
/// The OS host owns the window and the loop; Rust owns the pixels. The staging
/// platform is the OS-neutral [`HostPlatform`] from `floom-platform` — an event
/// queue and a sized surface, with **no** embedded/frameboard dependency —
/// because the real surface lives in ArkUI/UIKit, not in Rust. Each rendered
/// frame is presented into that native surface through `present`. The loop is
/// driven by `request_frame`/`poll_events` and stops when the host releases the
/// surface (a failed present) rather than spinning.
#[cfg(any(
    all(feature = "ios", target_os = "ios"),
    all(feature = "harmony", target_env = "ohos")
))]
fn run_os_frames(
    request: AppRequest,
    build: BuildFn,
    id: floom_types::PlatformId,
    caps: PlatformCapabilities,
    present: fn(&[u8], u32, u32) -> bool,
) -> Result<(), String> {
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform::{HostPlatform, Platform, WindowConfig};
    use floom_renderer_software::SoftwarePresenter;

    let mut platform = HostPlatform::new().with_id(id).with_capabilities(caps);
    let window = platform.create_window(WindowConfig {
        title: request.title,
        size: request.size,
        resizable: request.resizable,
    });
    let mut host = WidgetHost::new();
    build(&mut host);
    let mut runner = AppRunner::new(host, SoftwarePresenter::new());

    // The OS host drives the cadence on a real device via the sibling crate's
    // C ABI (vsync → `floom_*_present_frame`); `floom::run` here is the
    // standalone/testing entry. Queue the first redraw, then render-and-present
    // until the native present fails (surface released, or none bound in a
    // headless cross-build) so it terminates instead of spinning.
    platform.request_frame(window);
    loop {
        runner.pump(&mut platform);
        let p = runner.presenter();
        let (w, h) = (p.width(), p.height());
        if !present(p.pixels(), w, h) {
            break;
        }
        platform.request_frame(window);
    }
    Ok(())
}

/// Web backend: the `floom-platform-web` run loop behind the same door.
///
/// Only meaningful on `wasm32` — the backend's own `run` refuses any other target,
/// and the crate's `web` feature also pulls in `floom-renderer-web`.
#[cfg(all(feature = "web", target_arch = "wasm32"))]
mod web {
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform_web::{WebApp, WebPlatform};
    use floom_renderer_web::WebPresenter;

    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    pub struct WebBackend;

    impl PlatformBackend for WebBackend {
        fn name(&self) -> &'static str {
            "web (wasm + canvas)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            // The browser's `navigator.gpu` probe decides the WebGPU bit; vello
            // negotiates it at presenter creation, so claim it optimistically.
            PlatformCapabilities::web_default(true)
        }

        fn run(&self, _request: AppRequest, build: BuildFn) -> Result<(), String> {
            struct ClosureApp {
                build: Option<BuildFn>,
                runner: Option<AppRunner<WebPresenter>>,
            }

            impl WebApp for ClosureApp {
                fn events(&mut self, platform: &mut WebPlatform) {
                    if let Some(build) = self.build.take() {
                        let mut host = WidgetHost::new();
                        build(&mut host);
                        let presenter = WebPresenter::new();
                        self.runner = Some(AppRunner::new(host, presenter));
                    }
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }
            }

            floom_platform_web::run(
                ClosureApp {
                    build: Some(build),
                    runner: None,
                },
                "floom-canvas",
            )
        }
    }
}

/// Android backend: the `floom-platform-android` run loop behind the same door.
///
/// The `android_main` seam — [`floom_platform_android::set_android_app`] called
/// exactly once before this — is unchanged: the caller still owns it, because the
/// `AndroidApp` arrives before any Rust entry point and winit needs it while
/// building the loop.
#[cfg(all(feature = "mobile", feature = "renderer-gpu", target_os = "android"))]
mod android {
    use floom_app::{AppRunner, WidgetHost};
    use floom_platform::WindowConfig;
    use floom_platform_android::{AndroidPlatform, AndroidRunner};
    use floom_renderer_vello::VelloGpuPresenter;

    use super::{AppRequest, BuildFn, PlatformBackend, PlatformCapabilities};

    pub struct AndroidBackend;

    impl PlatformBackend for AndroidBackend {
        fn name(&self) -> &'static str {
            "android (winit + vello)"
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities::android_default()
        }

        fn run(&self, request: AppRequest, build: BuildFn) -> Result<(), String> {
            struct ClosureApp {
                build: Option<BuildFn>,
                runner: Option<AppRunner<VelloGpuPresenter>>,
            }

            impl AndroidRunner for ClosureApp {
                fn events(&mut self, platform: &mut AndroidPlatform) {
                    if let Some(build) = self.build.take() {
                        let mut host = WidgetHost::new();
                        build(&mut host);
                        let Some(presenter) = VelloGpuPresenter::new() else {
                            eprintln!(
                                "floom::run: no usable GPU adapter; an Android surface needs one to paint"
                            );
                            return;
                        };
                        self.runner = Some(AppRunner::new(host, presenter));
                    }
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }

                fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
                    if let Some(runner) = self.runner.as_mut() {
                        runner.pump(platform);
                    }
                }
            }

            let windows = vec![WindowConfig {
                title: request.title,
                size: request.size,
                resizable: request.resizable,
            }];
            floom_platform_android::run(
                ClosureApp {
                    build: Some(build),
                    runner: None,
                },
                windows,
            )
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warn_policy_reports_and_continues() {
        let backend = NoBackend;
        assert!(
            backend
                .require(Capability::CLIPBOARD, OnUnsupported::Warn)
                .is_ok()
        );
    }

    #[test]
    fn fail_policy_returns_the_missing_capability() {
        let backend = NoBackend;
        let error = backend
            .require(Capability::CLIPBOARD, OnUnsupported::Fail)
            .expect_err("no backend has a clipboard");
        assert_eq!(error.capability, Capability::CLIPBOARD);
        assert_eq!(error.backend, "none");
        assert!(error.to_string().contains("does not support"));
    }

    #[test]
    fn a_run_without_a_backend_explains_what_to_enable() {
        let error = NoBackend
            .run(
                AppRequest {
                    title: "t".into(),
                    size: floom_types::Size::new(1.0, 1.0),
                    resizable: false,
                },
                Box::new(|_| {}),
            )
            .expect_err("no backend");
        assert!(error.contains("--features"), "{error}");
    }

    #[test]
    fn every_available_backend_has_a_name() {
        assert!(!current().name().is_empty());
    }
}
