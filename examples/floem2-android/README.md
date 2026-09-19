# Floem 2.0 Android Example

A minimal Floem 2.0 counter app that runs on Android. It is the floem2
analogue of `examples/android` (the v1 entry point): same `AppRunner` +
`VelloGpuPresenter` stack as `floem2-window`, but driven by the Android
(winit) event loop through `platform/floem-platform-android`'s
`AndroidRunner` / `set_android_app` / `run` seam.

> Status: **real entry point wired**. The app compiles for
> `aarch64-linux-android` (with `--features real`) and the GPU surface is
> built from the Android `ANativeWindow*` by the vello renderer — the same
> path the desktop window uses. On-device render is the only unverified step
> (no NDK / device in this environment).

## How it works

```
android_main(app: AndroidApp)              // native-activity C glue
        │ set_android_app(app)
        ▼
run(AndroidWindowApp, windows)             // floem-platform-android
        │  winit-android event loop
        ▼
AndroidRunner::events / about_to_wait      // pump the AppRunner
        │  AppRunner::pump(platform)
        ▼
WindowCreated → presenter.attach(surface)  // surface from ANativeWindow*
        │  VelloGpuPresenter (wgpu / Vulkan)
        ▼
vello raster → wgpu swap-chain present
```

The only Android-specific seams are the `android_main` entry point and the
`AndroidRunner` callbacks that pump the app from winit's push loop. Everything
else (widgets, layout, the renderer, the surface attach) is identical to the
desktop window.

## Prerequisites

- The Android SDK **with an NDK** (`$ANDROID_HOME` / `$ANDROID_NDK_HOME` set):
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
scripts/build-apk.sh                      # arm64-v8a, release, --features real
scripts/build-apk.sh -t arm64-v8a --debug # debug profile
scripts/build-apk.sh --install            # then `adb install -r`
scripts/build-apk.sh --skip-build         # repackage an already-built .so
```

The APK lands in `target/android/floem2_android_example-<abi>-<profile>.apk`.

### Why `cargo ndk` alone is not enough

`cargo ndk` only cross-compiles and copies `*.so` into a `jniLibs/<abi>/`
layout — it never produces an APK. `scripts/build-apk.sh` covers the
packaging half with SDK tools only (`aapt2 link` → bundle `lib/` uncompressed
→ `zipalign -P 16` → `apksigner`, auto-creating a debug keystore). No
Gradle/AGP required.

The entry point must be a `cdylib` (not a bin): `native-activity` loads the
library named by the manifest's `android.app.lib_name` meta-data
(`floem2_android_example`). The `android_main` symbol is exported from
`src/lib.rs`; `src/main.rs` is just a host placeholder.

The `real` feature is likewise **mandatory, not optional**: the whole `android`
module — and with it the exported `android_main` — is gated on
`#[cfg(all(target_os = "android", feature = "real"))]`. Building without it
succeeds and yields a `.so` with no entry point, so the app would install and
then fail to start. The feature exists so the default host build (and
`cargo test --workspace`) never pulls wgpu/vello.
