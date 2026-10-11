//! The Android app: `native-activity` loads `libfloom_demo.so` and calls
//! [`android_main`], which mounts the same gallery and runs it on the Android
//! (winit) loop.
//!
//! This is why the crate is a `cdylib` (`[lib] crate-type` in `Cargo.toml`): a
//! bin-only crate would make `cargo ndk` copy nothing into `jniLibs/`. The
//! manifest's `android.app.lib_name` must stay equal to the `[lib] name`.

use floom_app::{AppRunner, WidgetHost};
use floom_layout::LayoutStyle;
use floom_platform::WindowConfig;
use floom_platform_android::{AndroidPlatform, AndroidRunner, run, set_android_app};
use floom_renderer_vello::VelloGpuPresenter;
use floom_types::Size;
use winit::platform::android::activity::AndroidApp;

use crate::demos;

/// Phone-portrait viewport.
const WINDOW: Size = Size::new(360.0, 720.0);

struct DemoAndroidApp {
    runner: AppRunner<VelloGpuPresenter>,
}

impl AndroidRunner for DemoAndroidApp {
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
    host.mount_root(LayoutStyle::default(), demos::gallery_root());

    let presenter = match VelloGpuPresenter::new() {
        Some(presenter) => presenter,
        None => {
            eprintln!("no usable GPU adapter; floom-demo needs a GPU presenter");
            return;
        }
    };
    eprintln!("GPU adapter: {}", presenter.adapter_info());

    let app = DemoAndroidApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floom — demo gallery".into(),
        size: WINDOW,
        resizable: false,
    }];

    if let Err(err) = run(app, windows) {
        eprintln!("android loop failed: {err}");
    }
}
