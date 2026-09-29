//! The Android app: `native-activity` loads `libfloem2_capabilities.so` and calls
//! [`android_main`], which mounts the same gallery and runs it on the Android
//! (winit) loop.
//!
//! This is the build where the gallery earns its keep: every row in the Readings
//! tab calls a real JNI bridge, so an emulator shows actual battery levels,
//! sensor samples and permission states rather than a column of `—`.
//!
//! This is also why the crate is a `cdylib` (`[lib] crate-type` in
//! `Cargo.toml`): a bin-only crate would make `cargo ndk` copy nothing into
//! `jniLibs/`. The manifest's `android.app.lib_name` must stay equal to the
//! `[lib] name`.

use floem_app::{AppRunner, WidgetHost};
use floem_layout::LayoutStyle;
use floem_platform::WindowConfig;
use floem_platform_android::{AndroidPlatform, AndroidRunner, run, set_android_app};
use floem_renderer_vello::VelloGpuPresenter;
use floem_types::Size;
use winit::platform::android::activity::AndroidApp;

use crate::ui;

/// Phone-portrait viewport. Narrow, on purpose: the name column is fixed-width,
/// so this is the width at which the layout is most stressed.
const WINDOW: Size = Size::new(360.0, 720.0);

struct CapabilitiesAndroidApp {
    runner: AppRunner<VelloGpuPresenter>,
}

impl AndroidRunner for CapabilitiesAndroidApp {
    fn events(&mut self, platform: &mut AndroidPlatform) {
        self.runner.pump(platform);
    }

    fn about_to_wait(&mut self, platform: &mut AndroidPlatform) {
        // No `request_frame` here on purpose: it would make every wake-up
        // produce another redraw, so the loop never parks. The runner asks for
        // a frame when the tree is dirty, which is what drives the first one.
        self.runner.pump(platform);
    }
}

/// Entry point invoked by the `native-activity` glue.
#[unsafe(no_mangle)]
pub fn android_main(app: AndroidApp) {
    // Hand the `AndroidApp` to the platform so `run` can build the winit event
    // loop that requires it.
    set_android_app(app);

    let mut host = WidgetHost::new();
    host.set_size(WINDOW);
    host.mount_root(LayoutStyle::default(), ui::gallery_root());

    let presenter = match VelloGpuPresenter::new() {
        Some(presenter) => presenter,
        None => {
            eprintln!("no usable GPU adapter; floem2-capabilities needs a GPU presenter");
            return;
        }
    };
    eprintln!("GPU adapter: {}", presenter.adapter_info());

    let app = CapabilitiesAndroidApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floem2 — platform capabilities".into(),
        size: WINDOW,
        resizable: false,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("android loop failed: {err}");
    }
}
