//! The IDE in a real window.
//!
//! Run with `cargo run -p ide`, or point it at a directory:
//!
//! ```text
//! cargo run -p ide -- ../../
//! ```
//!
//! The window is winit + vello, through the facade's one-call entry.
//!
//! Two more ways to run it, both for looking at pixels rather than at the app:
//!
//! * `--render <path>` writes one frame as a PPM through the software
//!   renderer, with no window and no display.
//! * `--software` opens the *same* window driven by the software renderer
//!   instead of the GPU one. The two exist so a picture can be attributed:
//!   a defect that appears in both is the demo's, and one that appears only in
//!   the GPU window is the presenter's.

use std::path::PathBuf;

use floom::prelude::*;
use floom_renderer::{FrameInfo, Renderer};
use floom_renderer_software::{SoftwarePresenter, SoftwareRenderer};
use ide_demo::{Ide, mount_ide};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut render: Option<PathBuf> = None;
    let mut software = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--render" => render = args.next().map(PathBuf::from),
            "--software" => software = true,
            other if root.is_none() => root = Some(PathBuf::from(other)),
            _ => {}
        }
    }
    let root = root.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("workspace"));
    let ide = Ide::new(root);

    if let Some(path) = render {
        render_png(&ide, &path);
        return;
    }

    if software {
        run_software_window(ide);
        return;
    }

    let result = floom::run_desktop("floom - IDE", Size::new(1180.0, 720.0), move |host| {
        mount_ide(host, &ide);
    });

    if let Err(error) = result {
        eprintln!("desktop loop failed: {error}");
        std::process::exit(1);
    }
}

/// Mounts the demo headlessly, frames it, and writes the pixels to `path`.
///
/// The scene is rasterised by the software renderer rather than by a GPU
/// presenter, so this needs no window and no display — and it is the same
/// scene, from the same host, that the window presents.
///
/// The file written is a binary PPM, which is the whole image in one `write`
/// with no encoder dependency: `sips -s format png in.ppm --out out.png`
/// turns it into anything else. The demo is checked by looking at pixels, and
/// a format that needs no crate is the format that keeps working.
fn render_png(ide: &Ide, path: &std::path::Path) {
    const WIDTH: u32 = 1180;
    const HEIGHT: u32 = 720;

    let mut host = WidgetHost::new();
    host.set_size(Size::new(WIDTH as f64, HEIGHT as f64));
    mount_ide(&mut host, ide);
    host.frame();

    let mut renderer = SoftwareRenderer::new(WIDTH, HEIGHT);
    renderer.begin_frame(&FrameInfo::software(WIDTH, HEIGHT, 1));
    renderer.render_scene_with_resources(host.scene(), host.scene().damage(), host.resources());
    renderer.end_frame();

    let pixels = renderer.pixels();
    let mut out = Vec::with_capacity(pixels.len() + 32);
    out.extend_from_slice(format!("P6\n{WIDTH} {HEIGHT}\n255\n").as_bytes());
    // The renderer's buffer is RGBA; a PPM is RGB, and the alpha is dropped
    // rather than composited over anything because the scene fills every
    // pixel it draws.
    for chunk in pixels.chunks_exact(4) {
        out.extend_from_slice(&chunk[..3]);
    }
    std::fs::write(path, out).unwrap_or_else(|error| {
        eprintln!("could not write {}: {error}", path.display());
        std::process::exit(1);
    });
    println!("wrote {} ({}x{})", path.display(), WIDTH, HEIGHT);
}

/// The same window, driven by the software renderer.
///
/// `floom::run_desktop` hard-codes the GPU presenter, so this is the same
/// shape as its `ClosureApp` with the presenter swapped — which is the whole
/// point: the tree, the host and the frame loop are identical, so a picture
/// that differs between this and the GPU window is the presenter's doing and
/// not the demo's.
fn run_software_window(ide: Ide) {
    use floom::floom_app::AppRunner;
    use floom::floom_platform_desktop::winit_backend::{WinitApp, WinitPlatform, run};
    use floom_platform::WindowConfig;

    struct SoftwareApp {
        ide: Option<Ide>,
        runner: Option<AppRunner<SoftwarePresenter>>,
    }

    impl WinitApp for SoftwareApp {
        fn window_created(
            &mut self,
            _platform: &mut WinitPlatform,
            _window: floom_types::WindowId,
        ) {
            let Some(ide) = self.ide.take() else {
                return;
            };
            let mut host = WidgetHost::new();
            mount_ide(&mut host, &ide);
            self.runner = Some(AppRunner::new(host, SoftwarePresenter::new()));
        }

        fn events(&mut self, platform: &mut WinitPlatform) {
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }

        fn about_to_wait(&mut self, platform: &mut WinitPlatform) {
            if let Some(runner) = self.runner.as_mut() {
                runner.pump(platform);
            }
        }
    }

    let windows = vec![WindowConfig {
        title: "floom - IDE (software)".to_string(),
        size: Size::new(1180.0, 720.0),
        resizable: true,
    }];
    if let Err(error) = run(
        SoftwareApp {
            ide: Some(ide),
            runner: None,
        },
        windows,
    ) {
        eprintln!("software window failed: {error}");
        std::process::exit(1);
    }
}
