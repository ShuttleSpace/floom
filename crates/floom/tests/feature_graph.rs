//! Phase 29.3 — the feature graph is trimmed: unused backends are not linked.
//!
//! "Not linked" is a *graph* claim, and it is checked where it is decidable:
//! the resolved dependency graph of the facade. With default features no
//! windowing backend, no renderer backend, no GPU stack and no tooling may
//! appear; enabling a bundle must add exactly that bundle's crates and
//! nothing else. A regression here is a beginner pulling wgpu into a headless
//! test binary.

use std::process::Command;

fn cargo_tree(extra: &[&str]) -> String {
    // The workspace root manifest, so `-p floom` resolves in workspace
    // context (the test crate's own manifest dir is the facade crate).
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .arg("tree")
        .arg(format!("--manifest-path={}", manifest.display()))
        .arg("-p")
        .arg("floom")
        .arg("-e")
        .arg("normal")
        .args(extra)
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("spawn cargo tree");
    assert!(
        output.status.success(),
        "cargo tree failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("cargo tree output is utf-8")
}

fn assert_absent(graph: &str, crate_name: &str) {
    assert!(
        !graph.contains(crate_name),
        "`{crate_name}` must not be in the resolved graph, but cargo tree reports it:\n{graph}"
    );
}

fn assert_present(graph: &str, crate_name: &str) {
    assert!(
        graph.contains(crate_name),
        "`{crate_name}` must be in the resolved graph, but cargo tree does not report it:\n{graph}"
    );
}

#[test]
fn default_features_are_headless_and_backend_free() {
    let graph = cargo_tree(&[]);

    // The core surface is always there.
    for core in ["floom-app", "floom-widgets", "floom-core", "floom-types"] {
        assert_present(&graph, core);
    }

    // No windowing backend, no renderer backend, no GPU stack, no tooling,
    // and never the legacy monolith.
    for backend in [
        "floom-platform-desktop",
        "floom-platform-android",
        "floom-platform-web",
        "floom-renderer-vello",
        "floom-renderer-software",
        "floom-renderer-skia",
        "floom-devtools",
        "winit",
        "wgpu",
        "vello",
    ] {
        assert_absent(&graph, backend);
    }
    // The v1 monolith must not be a normal dependency of the facade.
    //
    // Matched by *path*, not by package name. The package-name form of this
    // assertion was `name != "floom"`, and it became wrong the moment the
    // facade itself was renamed from `floom2` to `floom`: cargo tree printed the
    // facade's own name and the test read it as the monolith it was trying to
    // forbid. The intent -- "nothing in this graph is the v1 tree" -- is better
    // expressed by where a package lives than by what it is called, and it stays
    // true under any future rename.
    for line in graph.lines() {
        assert!(
            !line.contains("crates/floem/"),
            "the v1 monolith must not be a normal dependency of the facade:\n{graph}"
        );
    }
}

#[test]
fn desktop_bundle_adds_only_the_desktop_windowing_stack() {
    let graph = cargo_tree(&["--features", "desktop"]);

    assert_present(&graph, "floom-platform-desktop");
    assert_present(&graph, "winit");

    // Renderer and tooling stay out: a windowing backend is not a renderer.
    for backend in [
        "floom-renderer-vello",
        "floom-renderer-software",
        "floom-devtools",
        "wgpu",
    ] {
        assert_absent(&graph, backend);
    }
}

#[test]
fn renderer_gpu_bundle_adds_vello_and_the_gpu_stack() {
    let graph = cargo_tree(&["--features", "renderer-gpu"]);

    assert_present(&graph, "floom-renderer-vello");
    assert_present(&graph, "wgpu");

    // Windowing and other renderers stay out.
    for backend in [
        "floom-platform-desktop",
        "floom-renderer-software",
        "floom-renderer-skia",
        "floom-devtools",
    ] {
        assert_absent(&graph, backend);
    }
}

#[test]
fn renderer_software_bundle_adds_only_the_software_renderer() {
    let graph = cargo_tree(&["--features", "renderer-software"]);

    assert_present(&graph, "floom-renderer-software");

    for backend in [
        "floom-renderer-vello",
        "floom-renderer-skia",
        "wgpu",
        "winit",
        "floom-devtools",
    ] {
        assert_absent(&graph, backend);
    }
}

#[test]
fn devtools_bundle_adds_the_tooling_tier() {
    let graph = cargo_tree(&["--features", "devtools"]);

    assert_present(&graph, "floom-devtools");

    // Tooling is not a backend: no windowing, no GPU.
    for backend in [
        "floom-platform-desktop",
        "floom-renderer-vello",
        "wgpu",
        "winit",
    ] {
        assert_absent(&graph, backend);
    }
}
