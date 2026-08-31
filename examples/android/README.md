# Floem Android Example

A minimal Floem counter app that runs on Android. It demonstrates the
Phase 1 Android entry point: `floem::Application::new_android(app)`.

> Status: **skeleton**. The app compiles for `aarch64-linux-android` and the
> event loop / input path reuses the lapce winit fork's Android backend. The
> GPU renderer surface from the Android `ANativeWindow*` is the remaining
> integration step (see [`docs/android.md`](../../docs/android.md)).

## Prerequisites

- The Android SDK + NDK (`$ANDROID_HOME` / `ANDROID_NDK_HOME` set).
- Rust Android targets:
  ```bash
  rustup target add aarch64-linux-android armv7-linux-androideabi \
    i686-linux-android x86_64-linux-android
  ```
- [`cargo-ndk`](https://crates.io/crates/cargo-ndk):
  ```bash
  cargo install cargo-ndk
  ```

## Build

From this directory:

```bash
cargo ndk -t arm64-v8a -o ../app/src/main/jniLibs build --release
```

This produces `libfloem_android_example.so` under `../app/src/main/jniLibs/arm64-v8a/`.

## Package into an APK

Use the `AndroidManifest.xml` in this folder together with a tiny Android
Gradle project (or `apksigner` + `zipalign` by hand). The manifest is already
configured for a `native-activity` based app (`android:hasCode="false"`,
`android.app.NativeActivity`, `lib_name = floem_android_example`).

## Run

```bash
cargo ndk -t arm64-v8a run
```

Or install the APK manually and launch the activity; view logs with
`adb logcat`.
