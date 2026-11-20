// SPDX-License-Identifier: MIT
//! The emitted Rust is compiled by a real toolchain.
//!
//! A syntax check (`syn`-style) proves the output *parses*; this fixture
//! proves it *means something*: the builder calls type-check against the
//! actual `floom-widgets` API. The scratch crate lives under the workspace
//! `tests/` directory with its own target dir, so nothing collides with the
//! outer build's file lock and the dependency stack compiles once and is
//! cached across runs.
//!
//! The negative half matters as much as the positive: mutating one emitted
//! token must turn the check red, otherwise a fixture that compiles an empty
//! file would pass forever.

use std::path::{Path, PathBuf};
use std::process::Command;

const SAMPLE: &str = r#"
bindings {
    title: text
    save: handler
}
root = stack axis=vertical gap=8 {
    heading = text label=@title size=20
    actions = stack axis=horizontal gap=4 {
        ok = button label="OK" variant=filled on:click=save
        cancel = button label="Cancel" variant=text on:click=save
    }
    agreed = checkbox label="I agree"
}
"#;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../")
        .canonicalize()
        .expect("workspace root")
}

/// Write a scratch crate under `name` and return its path.
///
/// One directory *and one package name* per test: the tests run in parallel,
/// so sharing a crate directory would let one test's mutation leak into
/// another's check, and sharing a package name would let cargo's
/// mtime-based fingerprint treat the second crate as already-fresh when both
/// files land in the same timestamp tick.
fn scratch_crate(name: &str, lib_rs: &str) -> PathBuf {
    let root = workspace_root();
    let crate_dir = root.join("tests/dsl-fixture").join(name);
    std::fs::create_dir_all(crate_dir.join("src")).expect("create scratch crate");

    // The composite controls live in the satellite `floom-ui` workspace, so a
    // program that uses them needs that crate as a dependency. It is a path
    // dependency across a workspace boundary, which is fine for a scratch
    // crate with its own `[workspace]`.
    let manifest = format!(
        "[package]\nname = \"dsl-fixture-{name}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n\
         [dependencies]\n\
         floom-core = {{ path = \"{}/crates/core/floom-core\" }}\n\
         floom-types = {{ path = \"{}/crates/core/floom-types\" }}\n\
         floom-widgets = {{ path = \"{}/crates/widgets/floom-widgets\" }}\n\
         floom-ui-composites = {{ path = \"{}/crates/ecosystem/floom-ui/crates/floom-ui-composites\" }}\n\n\
         [workspace]\n",
        // Forward slashes and no `\\?\` prefix: a TOML basic string treats a
        // backslash as an escape, and Windows' `Path::display()` hands out both
        // (`"\\?\D:\a\floom/crates/..."`) -- which cargo then rejects as an
        // invalid escape, not as a bad path.
        toml_path(&root),
        toml_path(&root),
        toml_path(&root),
        toml_path(&root)
    );
    std::fs::write(crate_dir.join("Cargo.toml"), manifest).expect("write manifest");
    std::fs::write(crate_dir.join("src/lib.rs"), lib_rs).expect("write lib.rs");
    crate_dir
}

/// A path as TOML wants it: forward slashes, no verbatim prefix.
fn toml_path(path: &Path) -> String {
    path.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/")
}

/// Run `cargo check` over the scratch crate with a dedicated target dir.
fn cargo_check(crate_dir: &Path) -> Result<String, String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let target_dir = workspace_root().join("tests/dsl-fixture/target");
    let output = Command::new(&cargo)
        .arg("check")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", &target_dir)
        .output()
        .map_err(|e| format!("spawning {cargo}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stderr).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

#[test]
fn the_emitted_program_type_checks_against_the_real_widget_api() {
    let compiled = floom_dsl::compile(SAMPLE).expect("the sample is valid");
    // The sample uses a checkbox, and the checkbox lives in the composite
    // crate — so the program is emitted against that widget set.
    let options = floom_dsl::EmitOptions {
        widgets: floom_dsl::WidgetSource::WithComposites,
        ..floom_dsl::EmitOptions::default()
    };
    let emitted = floom_dsl::emit(&compiled.document, &options).expect("emits");
    // `variant` has no seat in the widget builder and a named handler becomes
    // an empty closure — both are documented approximations, reported in
    // `unmapped`. They still emit compilable code, which is what this fixture
    // is about.

    let crate_dir = scratch_crate("positive", &emitted.code);
    cargo_check(&crate_dir)
        .unwrap_or_else(|stderr| panic!("the generated program does not compile:\n{stderr}"));
}

/// The progress kinds compile against the real widget API.
///
/// The vocabulary grew a ring and a bar, because the satellite ships the
/// circular indicator as its own component and one generic kind cannot stand
/// for both. The proof that the two are real is that the emitted program
/// type-checks against `floom-widgets`' actual builders — not that the
/// emitter produces the right text.
#[test]
fn the_progress_kinds_compile_against_the_real_widget_api() {
    let source = r#"
root = stack axis=vertical gap=8 {
    ring = progress_circular value=0.5 size=32
    spin = progress_circular indeterminate=true
    bar = progress_linear value=50
}
"#;
    let compiled = floom_dsl::compile(source).expect("the sample is valid");
    let emitted =
        floom_dsl::emit(&compiled.document, &floom_dsl::EmitOptions::default()).expect("emits");
    assert!(
        emitted.code.contains("CircularProgress::determinate"),
        "the ring is determinate:\n{}",
        emitted.code
    );
    assert!(
        emitted.code.contains("CircularProgress::indeterminate"),
        "the spinning ring is indeterminate:\n{}",
        emitted.code
    );
    assert!(
        emitted.code.contains("LinearProgress::new"),
        "the bar is a linear progress:\n{}",
        emitted.code
    );

    let crate_dir = scratch_crate("progress", &emitted.code);
    cargo_check(&crate_dir).unwrap_or_else(|stderr| {
        panic!("the generated program does not compile:\n{stderr}");
    });
}

#[test]
fn a_program_emitted_against_the_primitives_alone_still_compiles() {
    // The other half of the widget-set choice: an app that depends only on
    // `floom-widgets` cannot name a checkbox, so the emitter substitutes a
    // placeholder — and says so, rather than emitting a call to a type that
    // does not exist.
    let compiled = floom_dsl::compile(SAMPLE).expect("the sample is valid");
    let emitted =
        floom_dsl::emit(&compiled.document, &floom_dsl::EmitOptions::default()).expect("emits");

    assert!(
        !emitted.code.contains("Checkbox::new("),
        "a widget the chosen set cannot name must not be called:\n{}",
        emitted.code
    );
    assert!(
        emitted
            .unmapped
            .iter()
            .any(|u| u.reason.contains("Checkbox")),
        "the substitution must be reported, not silent: {:?}",
        emitted.unmapped
    );

    let crate_dir = scratch_crate("primitives-only", &emitted.code);
    cargo_check(&crate_dir)
        .unwrap_or_else(|stderr| panic!("the generated program does not compile:\n{stderr}"));
}

#[test]
fn a_mutated_emission_turns_the_check_red() {
    // The control: the check above is only evidence if it can fail. Break one
    // emitted token and the toolchain must refuse.
    let compiled = floom_dsl::compile(SAMPLE).expect("the sample is valid");
    let emitted =
        floom_dsl::emit(&compiled.document, &floom_dsl::EmitOptions::default()).expect("emits");
    let broken = emitted.code.replace("Axis::Vertical", "Axis::Sideways");
    assert_ne!(broken, emitted.code, "the mutation must change something");

    let crate_dir = scratch_crate("negative", &broken);
    let error = cargo_check(&crate_dir).expect_err("a broken emission must not compile");
    assert!(
        error.contains("Sideways") || error.contains("error"),
        "the failure should name the problem:\n{error}"
    );
}
