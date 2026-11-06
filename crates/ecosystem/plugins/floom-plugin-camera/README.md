<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-camera

A **native camera**, contributed by a plugin.

The capability is a view the framework does not paint: a camera preview the
platform's own camera API renders live frames into, driven from the framework
side through the provider ABI ([`floom_native_ffi::floom_native`]).

# Layout

The plugin is split by platform. Each platform's native half lives under
`native/<platform>/` (`android/` is the exception: it is a Gradle module at
the crate root), and `src/<platform>.rs` holds that half's Rust FFI bridge.
The shared C-ABI contract lives in `native/common/floom_camera.h`. This file
dispatches the public surface to whichever platform is being compiled.

- `android/` — Java half (`FloomCameraView`), driven through JNI; the Rust
half owns the `NativeViewProvider`.
- `native/macos/` `native/ios/` `native/windows/` `native/linux/`
`native/ohos/` — a C/Objective-C/C++ native half that registers a
`FloomNativeProviderV1` vtable; the Rust half calls it through `extern "C"`.
- `src/web.rs` — pure Rust (`web-sys`): `getUserMedia` into a `<video>` element.
- `src/tui.rs` — unsupported: a terminal has no camera.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
| macOS | ✅ Real | `src/macos.rs` |
| Windows | ✅ Real | `src/windows.rs` |
| Linux | ✅ Real | `src/linux.rs` |
| OpenHarmony | ✅ Real | `src/ohos.rs` |
| Web | ✅ Real | `src/web.rs` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `version` — The plugin's version.
- `capability` — The plugin's capability id.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-camera = { path = "crates/ecosystem/plugins/floom-plugin-camera" }
```

A minimal use:

```rust,ignore
floom_plugin_camera::register();
floom_platform::plugin("camera").enable()?;
// Then ask for the provider and hand it the host's window.
let provider = unsafe { floom_plugin_camera::provider(window) }?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn version() -> String {`
- `pub fn capability() -> i32 {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_camera.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-camera
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-camera/example/Cargo.toml
```
