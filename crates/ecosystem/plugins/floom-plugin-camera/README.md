<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->
# floom-plugin-camera

A native camera view, contributed by a plugin.

The capability is a **view** the framework does not paint: a camera preview the
platform's own camera API renders live frames into, driven from the framework
side through the provider ABI (`floom-native-ffi`). The plugin owns the
capability; the host owns the window.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants a camera adds the crate.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `android/` + `src/android.rs` | Java `FloomCameraView`, driven over JNI; the Rust half owns the `NativeViewProvider` |
| iOS | `native/ios/` + `src/ios.rs` | Objective-C `AVFoundation`, compiled by `build.rs` |
| macOS | `native/macos/` + `src/macos.rs` | Objective-C `AVFoundation`, compiled by `build.rs` |
| Linux | `native/linux/` + `src/linux.rs` | C11 + GStreamer/X11, compiled by `build.rs` |
| Windows | `native/windows/` + `src/windows.rs` | C++ `MediaFoundation`, compiled by `build.rs` |
| OpenHarmony | `native/ohos/` + `src/ohos.rs` | C++ half, compiled by `build.rs` |
| Web | `src/web.rs` | pure Rust `getUserMedia` into a `<video>` element |
| Terminal | `src/tui.rs` | refuses: a terminal has no camera |

## API

- `pub fn version() -> String`
- `pub fn capability() -> i32`
- `pub fn enable() -> Result<(), String>`
- `pub fn disable()`
- `pub unsafe fn provider(host_window: *const c_void) -> Result<Box<dyn NativeViewProvider>, CProviderError>`
- `pub fn register()`

## Using it

```toml
[dependencies]
floom-plugin-camera = { path = "crates/ecosystem/plugins/floom-plugin-camera" }
```

```rust,ignore
floom_plugin_camera::register();
floom_platform::plugin("camera").enable()?;
// Then ask for the provider and hand it the host's window.
let provider = unsafe { floom_plugin_camera::provider(window) }?;
```

## Layout

This plugin follows the native-view variant of the template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#a-plugin-that-contributes-a-native-view):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's Rust bridge.
- `native/common/floom_camera.h` — the plugin's own C ABI (enable/disable/
  capability plus provider registration).
- `native/common/` also carries `floom_native_provider.h`, the framework's
  provider ABI.
- `native/<target>/` — the compiled native halves.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-camera
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-camera/example/Cargo.toml
```

Device verification is separate: a camera preview needs a real camera and the
platform's permission prompt.
