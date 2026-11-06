// SPDX-License-Identifier: MIT
//! A GUI that is authored in TSX and materialized at build time.
//!
//! `build.rs` compiles `src/counter.tsx` into `OUT_DIR/ui_gen.rs`; this file just
//! mounts the generated tree. The default build only checks that codegen compiles;
//! `--features real` runs it in a real winit + vello window.
//!
//! NOTE: this binary needs a GPU presenter to actually open a window. Run it with
//! `--features real` on a machine with a working Vulkan/Metal/DX12 adapter.

#[allow(unused_imports, clippy::all)]
mod ui_gen {
    include!(concat!(env!("OUT_DIR"), "/ui_gen.rs"));
}

#[cfg(feature = "real")]
fn run_gui() {
    use floom_app::{AppRunner, WidgetHost};
    use floom_layout::LayoutStyle;
    use floom_platform::WindowConfig;
    use floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
    use floom_renderer_vello::VelloGpuPresenter;
    use floom_types::Size;

    /// The app wraps the runner; the only event work is pumping the runner so its
    /// frame loop keeps running.
    struct DemoApp {
        runner: AppRunner<VelloGpuPresenter>,
    }

    impl WinitApp for DemoApp {
        fn events(&mut self, platform: &mut WinitPlatform) {
            self.runner.pump(platform);
        }
        fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
            self.runner.pump(platform);
        }
    }

    let mut host = WidgetHost::new();
    host.mount_root_boxed(LayoutStyle::default(), ui_gen::build());

    let presenter = VelloGpuPresenter::new().unwrap_or_else(|| {
        eprintln!("no usable GPU adapter; this example needs a GPU presenter");
        std::process::exit(1);
    });
    let app = DemoApp {
        runner: AppRunner::new(host, presenter),
    };
    let windows = vec![WindowConfig {
        title: "floom-tsx".into(),
        size: Size::new(480.0, 320.0),
        resizable: true,
    }];
    if let Err(err) = run(app, windows) {
        eprintln!("window loop failed: {err}");
        std::process::exit(1);
    }
}

fn main() {
    #[cfg(feature = "real")]
    run_gui();
    #[cfg(not(feature = "real"))]
    println!(
        "floom-tsx-showcase: codegen compiled. Rebuild with --features real to open the window."
    );
}
