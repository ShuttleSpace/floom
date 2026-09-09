# Floem Android Example

A minimal Floem counter app that runs on Android. It demonstrates the
Phase 1 Android entry point: `floem::Application::new_android(app)`.

> Status: **skeleton**. The app compiles for `aarch64-linux-android` and the
> event loop / input path reuses the lapce winit fork's Android backend. The
> GPU renderer surface from the Android `ANativeWindow*` is the remaining
> integration step (see [`docs/android.md`](../../docs/android.md)).

## Prerequisites

- The Android SDK **with an NDK** (`$ANDROID_HOME` / `$ANDROID_NDK_HOME` set).
  This machine's SDK has `build-tools` + `platforms` but **no NDK**, so the
  Rust part cannot be compiled until one is installed:
  ```bash
  $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager "ndk;27.0.12077973"
  ```
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi \
    i686-linux-android x86_64-linux-android
  ```
- [`cargo-ndk`](https://crates.io/crates/cargo-ndk):
  ```bash
  cargo install cargo-ndk
  ```
- A JDK (only for `apksigner`; without one the script still produces an APK
  named `*-unsigned.apk` that you must sign yourself).

## Build + package an APK

```bash
scripts/build-apk.sh                      # arm64-v8a, release
scripts/build-apk.sh -t arm64-v8a --debug # debug profile
scripts/build-apk.sh --install            # then `adb install -r`
scripts/build-apk.sh --skip-build         # repackage an already-built .so
```

The APK lands in `target/android/floem_android_example-<abi>-<profile>.apk`.

### Why `cargo ndk` alone is not enough

Two separate things are going on, and both bite:

1. **`cargo ndk` never produces an APK by design.** It only cross-compiles and
   copies `*.so` files into a `jniLibs/<abi>/` layout. APK creation was
   `cargo-apk`'s job (unmaintained; `x build` is its successor).
2. **`cargo ndk` also produced no `.so` here**, because the crate was bin-only.
   `native-activity` loads the library named by the manifest's
   `android.app.lib_name` meta-data, so the Android entry point must be a
   `cdylib`. `Cargo.toml` now declares:
   ```toml
   [lib]
   name = "floem_android_example"
   path = "src/lib.rs"
   crate-type = ["cdylib"]
   ```
   and `android_main` + the JNI bridges moved from `src/main.rs` to
   `src/lib.rs`. `src/main.rs` is now just a host placeholder.

`scripts/build-apk.sh` covers the packaging half with SDK tools only:
`aapt2 link` → bundle `lib/` uncompressed → `zipalign -P 16` → `apksigner`
(auto-creates a debug keystore). No Gradle/AGP download required.

If you prefer Gradle/AGP, run
`cargo ndk -t arm64-v8a -o app/src/main/jniLibs build --release` and point a
standard Android app module's `AndroidManifest.xml` at the manifest in this
folder.

## Optional: file dialogs and receiving shared files

The default configuration uses `android.app.NativeActivity` with
`android:hasCode="false"`, which covers everything except receiving
file-picker results and files shared into a *running* app. See the header of
`FloemNativeActivity.java` for the three-step opt-in.
