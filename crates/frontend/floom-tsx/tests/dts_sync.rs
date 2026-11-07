// SPDX-License-Identifier: MIT
//! Keeps the published `@types/floom` artifact in lockstep with the generator.

use std::path::PathBuf;

fn package_dir() -> PathBuf {
    // crates/frontend/floom-tsx -> repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../npm/@types-floom")
}

const REGEN: &str =
    "regenerate with: SYNC_DTS=1 rustup run 1.99.1 cargo test -p floom-tsx --test dts_sync";

#[test]
fn npm_artifact_matches_generator() {
    let generated = floom_tsx::generate_dts();
    let path = package_dir().join("index.d.ts");
    if std::env::var_os("SYNC_DTS").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent dir")).expect("mkdir");
        std::fs::write(&path, &generated).expect("write index.d.ts");
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}; {REGEN}", path.display()));
    assert!(
        committed == generated,
        "@types/floom is stale or was edited by hand; {REGEN}"
    );
}

#[test]
fn npm_package_version_matches_crate() {
    let path = package_dir().join("package.json");
    let pkg = std::fs::read_to_string(&path).expect("read package.json");
    let expected = format!("\"version\": \"{}\"", env!("CARGO_PKG_VERSION"));
    assert!(
        pkg.contains(&expected),
        "{} must declare {expected} so the types track floom-tsx",
        path.display()
    );
}
