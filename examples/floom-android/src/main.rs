//! Desktop placeholder for the Floom 2.0 Android example.
//!
//! The Android entry point (`android_main`) lives in `crates/floom/src/lib.rs`, built as a
//! `cdylib` (`libfloom_android_example.so`) — that is the artifact
//! `native-activity` loads, and the only one `cargo ndk` copies into
//! `jniLibs/`. This binary exists only so the crate still has a runnable target
//! on the host; it does nothing useful.

fn main() {
    eprintln!(
        "This example targets Android only. Package an APK with:\n\
         \n  scripts/build-apk.sh -t arm64-v8a\n"
    );
}
