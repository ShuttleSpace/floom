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
//! `--render <path>` writes one frame to a PNG through the software renderer
//! instead of opening a window. That is how the demo's pixels are checked on a
//! machine with no display, and how a screenshot of a specific state is taken
//! without a human clicking: `--render` draws the same scene the window would.

use std::path::PathBuf;

use floom::prelude::*;
use floom_renderer::{FrameInfo, Renderer};
use floom_renderer_software::SoftwareRenderer;
use ide_demo::{Ide, mount_ide};

fn main() {
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = None;
    let mut render: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--render" => render = args.next().map(PathBuf::from),
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
