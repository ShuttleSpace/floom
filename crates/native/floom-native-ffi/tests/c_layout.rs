// SPDX-License-Identifier: MIT
//! The ABI's layout, checked from C's side.
//!
//! The header is hand-written and the Rust mirror is hand-written, and two
//! hand-written descriptions of the same thing drift: a field is reordered, a
//! type is widened, an enum gets a different discriminant. The symptom is not
//! a compile error — it is a provider writing into a struct the framework
//! reads at the wrong offsets, which is memory corruption.
//!
//! So the header is compiled by the platform's own C compiler, and the
//! program it compiles reports every struct's size and every field's offset.
//! Those numbers are compared against the Rust mirror's, and a difference
//! fails the test.
//!
//! The same program also drives a provider through the vtable **from C**, so
//! the header is proved usable by the language it is written for. A header
//! that only compiles because Rust mirrors it is not a header anybody can
//! implement.
//!
//! # When there is no C compiler
//!
//! The test says so and passes. It is a check that needs a toolchain, and a
//! missing toolchain is not a defect in the ABI. Where a compiler *is*
//! present, a disagreement is a failure.

use std::collections::HashMap;
use std::mem::offset_of;
use std::path::PathBuf;
use std::process::Command;

use floom_native_ffi::abi;

/// The compiler to try, in the order they are tried.
const COMPILERS: &[&str] = &["cc", "clang", "gcc"];

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("layout_probe.c")
}

fn header() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("include")
        .join("floom_native_provider.h")
}

/// A directory for the probe's build products.
fn scratch() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("native-ffi-layout");
    std::fs::create_dir_all(&dir).expect("scratch directory");
    dir
}

/// Find a C compiler, or `None`.
fn find_compiler() -> Option<String> {
    if let Ok(explicit) = std::env::var("CC")
        && !explicit.is_empty()
        && which(&explicit)
    {
        return Some(explicit);
    }
    COMPILERS
        .iter()
        .find(|name| which(name))
        .map(|name| (*name).to_string())
}

fn which(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    path.split(':').any(|dir| {
        let candidate = PathBuf::from(dir).join(name);
        candidate.is_file()
    })
}

/// Compile the probe and run it, returning its `key=value` output.
fn probe(compiler: &str) -> HashMap<String, String> {
    let dir = scratch();
    let binary = dir.join("layout_probe");
    let status = Command::new(compiler)
        .arg(fixture())
        .arg("-I")
        .arg(header().parent().expect("the header has a parent"))
        .arg("-std=c11")
        .arg("-Wall")
        .arg("-Wextra")
        .arg("-Werror")
        .arg("-o")
        .arg(&binary)
        .status()
        .expect("run the C compiler");
    assert!(
        status.success(),
        "the probe did not compile; the header is not valid C"
    );

    let output = Command::new(&binary).output().expect("run the probe");
    assert!(
        output.status.success(),
        "the probe failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect()
}

#[test]
fn the_header_and_the_rust_mirror_agree_on_the_layout() {
    let Some(compiler) = find_compiler() else {
        eprintln!(
            "SKIPPED: no C compiler found (looked for {:?} and $CC); \
             the layout check needs one",
            COMPILERS
        );
        return;
    };

    let facts = probe(&compiler);

    // The sizes. A struct whose size differs is the loudest kind of drift.
    let sizes = [
        (
            "FloomNativeSlot",
            std::mem::size_of::<abi::FloomNativeSlot>(),
        ),
        (
            "FloomNativeDeviceRect",
            std::mem::size_of::<abi::FloomNativeDeviceRect>(),
        ),
        (
            "FloomNativeCapabilities",
            std::mem::size_of::<abi::FloomNativeCapabilities>(),
        ),
        (
            "FloomNativeState",
            std::mem::size_of::<abi::FloomNativeState>(),
        ),
        (
            "FloomNativeProviderV1",
            std::mem::size_of::<abi::FloomNativeProviderV1>(),
        ),
    ];
    for (name, rust) in sizes {
        let c = facts
            .get(&format!("size.{name}"))
            .unwrap_or_else(|| panic!("the probe did not report size.{name}"));
        assert_eq!(
            c.as_str(),
            rust.to_string(),
            "the size of {name} differs between the header and the Rust mirror"
        );
    }

    // The offsets. These are what a wrong field order actually breaks.
    //
    // Every field of every struct, not a sample: a struct whose *size* agrees
    // can still have its fields in a different order, and that is the drift
    // this check exists to catch.
    let offsets: &[(&str, usize)] = &[
        (
            "FloomNativeSlot.index",
            offset_of!(abi::FloomNativeSlot, index),
        ),
        (
            "FloomNativeSlot.generation",
            offset_of!(abi::FloomNativeSlot, generation),
        ),
        (
            "FloomNativeDeviceRect.x",
            offset_of!(abi::FloomNativeDeviceRect, x),
        ),
        (
            "FloomNativeDeviceRect.y",
            offset_of!(abi::FloomNativeDeviceRect, y),
        ),
        (
            "FloomNativeDeviceRect.width",
            offset_of!(abi::FloomNativeDeviceRect, width),
        ),
        (
            "FloomNativeDeviceRect.height",
            offset_of!(abi::FloomNativeDeviceRect, height),
        ),
        (
            "FloomNativeCapabilities.modes",
            offset_of!(abi::FloomNativeCapabilities, modes),
        ),
        (
            "FloomNativeCapabilities.mode_count",
            offset_of!(abi::FloomNativeCapabilities, mode_count),
        ),
        (
            "FloomNativeCapabilities.snapshot",
            offset_of!(abi::FloomNativeCapabilities, snapshot),
        ),
        (
            "FloomNativeCapabilities.input_passthrough",
            offset_of!(abi::FloomNativeCapabilities, input_passthrough),
        ),
        (
            "FloomNativeCapabilities.input_capture",
            offset_of!(abi::FloomNativeCapabilities, input_capture),
        ),
        (
            "FloomNativeCapabilities.focusable",
            offset_of!(abi::FloomNativeCapabilities, focusable),
        ),
        (
            "FloomNativeCapabilities.clip",
            offset_of!(abi::FloomNativeCapabilities, clip),
        ),
        (
            "FloomNativeState.rect",
            offset_of!(abi::FloomNativeState, rect),
        ),
        (
            "FloomNativeState.clip",
            offset_of!(abi::FloomNativeState, clip),
        ),
        (
            "FloomNativeState.has_clip",
            offset_of!(abi::FloomNativeState, has_clip),
        ),
        (
            "FloomNativeState.visible",
            offset_of!(abi::FloomNativeState, visible),
        ),
        ("FloomNativeState.z", offset_of!(abi::FloomNativeState, z)),
        (
            "FloomNativeState.input",
            offset_of!(abi::FloomNativeState, input),
        ),
        (
            "FloomNativeState.mode",
            offset_of!(abi::FloomNativeState, mode),
        ),
        (
            "FloomNativeProviderV1.abi_version",
            offset_of!(abi::FloomNativeProviderV1, abi_version),
        ),
        (
            "FloomNativeProviderV1.self",
            offset_of!(abi::FloomNativeProviderV1, self_),
        ),
        (
            "FloomNativeProviderV1.attach",
            offset_of!(abi::FloomNativeProviderV1, attach),
        ),
        (
            "FloomNativeProviderV1.capabilities",
            offset_of!(abi::FloomNativeProviderV1, capabilities),
        ),
        (
            "FloomNativeProviderV1.create",
            offset_of!(abi::FloomNativeProviderV1, create),
        ),
        (
            "FloomNativeProviderV1.destroy",
            offset_of!(abi::FloomNativeProviderV1, destroy),
        ),
        (
            "FloomNativeProviderV1.apply",
            offset_of!(abi::FloomNativeProviderV1, apply),
        ),
        (
            "FloomNativeProviderV1.focus",
            offset_of!(abi::FloomNativeProviderV1, focus),
        ),
        (
            "FloomNativeProviderV1.blur",
            offset_of!(abi::FloomNativeProviderV1, blur),
        ),
    ];
    for (name, rust) in offsets {
        let key = format!("offset.{name}");
        let c = facts
            .get(&key)
            .unwrap_or_else(|| panic!("the probe did not report {key}"));
        assert_eq!(
            c.as_str(),
            rust.to_string(),
            "the offset of {name} differs between the header and the Rust mirror"
        );
    }
}

#[test]
fn a_provider_can_be_written_and_driven_in_c() {
    let Some(compiler) = find_compiler() else {
        eprintln!(
            "SKIPPED: no C compiler found (looked for {:?} and $CC); \
             the C-side provider check needs one",
            COMPILERS
        );
        return;
    };

    let facts = probe(&compiler);

    // The provider answered with the capabilities it was built with, in the
    // order it published them.
    assert_eq!(facts.get("probe.mode_count").map(String::as_str), Some("2"));
    assert_eq!(
        facts.get("probe.first_mode").map(String::as_str),
        Some("0"),
        "Overlay is 0 in the header, and the provider published it first"
    );
    assert_eq!(facts.get("probe.can_crop").map(String::as_str), Some("1"));

    // The calls arrived, and the arguments crossed intact.
    assert_eq!(facts.get("probe.created").map(String::as_str), Some("1"));
    assert_eq!(facts.get("probe.destroyed").map(String::as_str), Some("1"));
    assert_eq!(facts.get("probe.focused").map(String::as_str), Some("1"));
    assert_eq!(facts.get("probe.blurred").map(String::as_str), Some("1"));
    assert_eq!(facts.get("probe.applied").map(String::as_str), Some("1"));
    assert_eq!(
        facts.get("probe.kind_name").map(String::as_str),
        Some("com.example.three-d"),
        "the custom kind's name crossed as a string"
    );
    assert_eq!(facts.get("probe.slot_index").map(String::as_str), Some("7"));
    assert_eq!(
        facts.get("probe.slot_generation").map(String::as_str),
        Some("3")
    );
    assert_eq!(
        facts.get("probe.applied_rect").map(String::as_str),
        Some("10,20,300,150"),
        "the device rect crossed intact"
    );
    assert_eq!(
        facts.get("probe.applied_mode").map(String::as_str),
        Some("1"),
        "Texture is 1 in the header"
    );
    assert_eq!(
        facts.get("probe.applied_input").map(String::as_str),
        Some("1"),
        "Capture is 1 in the header"
    );

    // The error channel works: the probe refuses on purpose and says why.
    assert_eq!(
        facts.get("probe.create_message").map(String::as_str),
        Some("the probe refuses on purpose"),
        "the error message crossed the boundary"
    );
}
