//! Compile fixtures for the codegen backends (Phase 18.2, Step 5).
//!
//! Each test renders a reference schema, writes the generated files to a
//! scratch directory, and then *actually invokes the local compiler* on the
//! result. A generator that emits plausible-looking but uncompilable code fails
//! here, which is the point: the fixture is a check that can go red.
//!
//! Toolchain coverage is environment-dependent and reported honestly:
//!   * C    — `cc`   (Apple clang)
//!   * C++  — `c++`  (Apple clang)
//!   * Swift— `swiftc`
//!   * Kotlin — `kotlinc` (absent on this machine; the test skips with a
//!     printed reason rather than silently passing).

use floem_codegen::fixtures::{demo_video_player, wide_type_coverage};
use floem_codegen::{GeneratedFile, Language, generate};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workdir(tag: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("floem-codegen-fx-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).expect("create scratch dir");
    p
}

fn write_files(dir: &Path, files: &[GeneratedFile]) {
    for f in files {
        std::fs::write(dir.join(&f.path), &f.contents)
            .unwrap_or_else(|e| panic!("write {}: {e}", f.path));
    }
}

/// Run a program in `dir`. Returns `None` (with a printed reason) when the
/// program is not installed, so a missing toolchain is visible but not a lie.
fn run(dir: &Path, program: &str, args: &[&str]) -> Option<Output> {
    match Command::new(program).args(args).current_dir(dir).output() {
        Ok(o) => Some(o),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("SKIP: `{program}` not found on PATH");
            None
        }
        Err(e) => panic!("failed to spawn {program}: {e}"),
    }
}

fn assert_success(out: &Output, program: &str) {
    assert!(
        out.status.success(),
        "{program} failed (status {:?}):\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn c_demo_header_compiles() {
    let schema = demo_video_player();
    let files = generate(&schema, Language::C).unwrap();
    let dir = workdir("c-demo");
    write_files(&dir, &files);
    let header = &files[0].path;
    std::fs::write(
        dir.join("probe.c"),
        format!(
            "#include \"{header}\"\n\
             void probe(floem_video_player_handle *h) {{\n\
             \x20   floem_video_player_set_src(h, \"x\");\n\
             \x20   const char *s = floem_video_player_get_src(h);\n\
             \x20   (void)s;\n\
             \x20   floem_video_player_cmd_play(h);\n\
             \x20   floem_video_player_cmd_seek(h, 10);\n\
             \x20   uint64_t p = floem_video_player_cmd_position(h);\n\
             \x20   (void)p;\n\
             \x20   floem_video_player_on_ended(h, NULL, NULL);\n\
             \x20   floem_video_player_on_error(h, NULL, NULL);\n\
             }}\n"
        ),
    )
    .unwrap();

    let Some(out) = run(
        &dir,
        "cc",
        &["-std=c11", "-Wall", "-c", "probe.c", "-o", "probe.o"],
    ) else {
        return;
    };
    assert_success(&out, "cc");
}

#[test]
fn c_wide_header_compiles() {
    let schema = wide_type_coverage();
    let files = generate(&schema, Language::C).unwrap();
    let dir = workdir("c-wide");
    write_files(&dir, &files);
    let header = &files[0].path;
    std::fs::write(
        dir.join("probe.c"),
        format!(
            "#include \"{header}\"\n\
             void probe(floem_wide_probe_handle *h) {{\n\
             \x20   floem_bytes b = {{ NULL, 0 }};\n\
             \x20   floem_wide_probe_set_payload(h, b);\n\
             \x20   floem_wide_probe_set_texture(h, 0);\n\
             \x20   floem_wide_probe_set_native(h, NULL);\n\
             \x20   int32_t c = floem_wide_probe_get_maybe_count(h);\n\
             \x20   (void)c;\n\
             \x20   floem_slice_f32 s = {{ NULL, 0 }};\n\
             \x20   floem_wide_probe_set_device(h, NULL);\n\
             \x20   floem_bytes r = floem_wide_probe_cmd_submit(h, s);\n\
             \x20   (void)r;\n\
             }}\n"
        ),
    )
    .unwrap();

    let Some(out) = run(
        &dir,
        "cc",
        &["-std=c11", "-Wall", "-c", "probe.c", "-o", "probe.o"],
    ) else {
        return;
    };
    assert_success(&out, "cc");
}

#[test]
fn cpp_demo_facade_compiles() {
    let schema = demo_video_player();
    let dir = workdir("cpp-demo");
    // The facade includes the C header, so both must be present.
    write_files(&dir, &generate(&schema, Language::C).unwrap());
    write_files(&dir, &generate(&schema, Language::Cpp).unwrap());
    std::fs::write(
        dir.join("probe.cpp"),
        "#include \"floem_video_player_glue.hpp\"\n\
         void probe(floem_video_player_handle *h) {\n\
         \x20   floem_gen::VideoPlayer v(h);\n\
         \x20   v.set_src(\"x\");\n\
         \x20   std::string s = v.get_src();\n\
         \x20   v.cmd_seek(10);\n\
         \x20   uint64_t p = v.cmd_position();\n\
         \x20   v.on_ended([]{});\n\
         \x20   v.on_error([](const char *){});\n\
         \x20   (void)s; (void)p;\n\
         }\n",
    )
    .unwrap();

    let Some(out) = run(
        &dir,
        "c++",
        &["-std=c++17", "-Wall", "-c", "probe.cpp", "-o", "probe.o"],
    ) else {
        return;
    };
    assert_success(&out, "c++");
}

#[test]
fn cpp_wide_facade_compiles() {
    let schema = wide_type_coverage();
    let dir = workdir("cpp-wide");
    write_files(&dir, &generate(&schema, Language::C).unwrap());
    write_files(&dir, &generate(&schema, Language::Cpp).unwrap());
    std::fs::write(
        dir.join("probe.cpp"),
        "#include \"floem_wide_probe_glue.hpp\"\n\
         void probe(floem_wide_probe_handle *h) {\n\
         \x20   floem_gen::WideProbe w(h);\n\
         \x20   w.set_payload(floem_bytes{nullptr, 0});\n\
         \x20   w.set_texture(0);\n\
         \x20   w.set_native(nullptr);\n\
         \x20   int32_t c = w.get_maybe_count();\n\
         \x20   (void)c;\n\
         \x20   w.set_device(nullptr);\n\
         \x20   floem_bytes r = w.cmd_submit(floem_slice_f32{nullptr, 0});\n\
         \x20   (void)r;\n\
         \x20   w.on_frame([](const uint8_t *, size_t){});\n\
         }\n",
    )
    .unwrap();

    let Some(out) = run(
        &dir,
        "c++",
        &["-std=c++17", "-Wall", "-c", "probe.cpp", "-o", "probe.o"],
    ) else {
        return;
    };
    assert_success(&out, "c++");
}

#[test]
fn swift_glue_typechecks() {
    let dir = workdir("swift");
    for schema in [demo_video_player(), wide_type_coverage()] {
        write_files(&dir, &generate(&schema, Language::Swift).unwrap());
    }
    let Some(out) = run(
        &dir,
        "swiftc",
        &[
            "-typecheck",
            "VideoPlayerShims.swift",
            "VideoPlayer.swift",
            "WideProbeShims.swift",
            "WideProbe.swift",
        ],
    ) else {
        return;
    };
    assert_success(&out, "swiftc");
}

#[test]
fn kotlin_fixture_compiles_when_kotlinc_present() {
    let dir = workdir("kotlin");
    write_files(
        &dir,
        &generate(&demo_video_player(), Language::Kotlin).unwrap(),
    );
    let Some(out) = run(&dir, "kotlinc", &["VideoPlayer.kt", "-d", "out"]) else {
        return;
    };
    assert_success(&out, "kotlinc");
}
