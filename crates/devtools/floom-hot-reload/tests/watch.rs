//! The watcher answers "what changed", and the app decides what to do with it.

use std::path::PathBuf;

use floom_hot_reload::watch::Watch;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("floom-watch-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(dir.join("src")).unwrap();
    dir
}

#[test]
fn the_first_poll_is_quiet() {
    // Otherwise every launch rebuilds the tree it just built.
    let dir = scratch("first");
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
    let mut watch = Watch::new(&dir);
    assert!(watch.poll().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_new_file_is_reported_once() {
    let dir = scratch("new");
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
    let mut watch = Watch::new(&dir);
    watch.poll();

    std::fs::write(dir.join("src/app.rs"), "pub fn build() {}").unwrap();
    let changed = watch.poll();
    assert!(
        changed.iter().any(|p| p.ends_with("app.rs")),
        "got: {changed:?}"
    );

    assert!(
        watch.poll().is_empty(),
        "a reported file is not re-reported"
    );
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_rewrite_with_a_newer_timestamp_is_reported() {
    let dir = scratch("rewrite");
    let file = dir.join("src/main.rs");
    std::fs::write(&file, "fn main() {}").unwrap();
    let mut watch = Watch::new(&dir);
    watch.poll();

    // Editors truncate and rewrite rather than creating a new inode, so the
    // change is only visible through the timestamp.
    std::thread::sleep(std::time::Duration::from_millis(20));
    std::fs::write(&file, "fn main() { println!(\"hi\"); }").unwrap();
    assert!(!watch.poll().is_empty());
    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn a_deleted_file_is_reported_and_build_output_is_ignored() {
    let dir = scratch("deleted");
    std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(dir.join("src/old.rs"), "// gone").unwrap();
    let mut watch = Watch::new(&dir);
    watch.poll();

    std::fs::remove_file(dir.join("src/old.rs")).unwrap();
    std::fs::create_dir_all(dir.join("target/debug")).unwrap();
    std::fs::write(dir.join("target/debug/junk.rs"), "// never").unwrap();

    let changed = watch.poll();
    assert!(
        changed.iter().any(|p| p.ends_with("old.rs")),
        "got: {changed:?}"
    );
    assert!(
        !changed
            .iter()
            .any(|p| p.to_string_lossy().contains("target")),
        "build output is not a source change: {changed:?}"
    );
    std::fs::remove_dir_all(&dir).ok();
}
