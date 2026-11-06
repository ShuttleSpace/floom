<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->
# floom-plugin-webview

A native web view, contributed by a plugin.

The capability is a **view** the framework does not paint: a `WKWebView` on
macOS and iOS, a `WebView` on Android and Linux, a WebView2 on Windows. What
the view *shows* is the web's own business; this crate owns the capability
(create / place / destroy a native view, drive a JS bridge) and nothing about
any one platform.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants a web view adds the crate.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `android/` + `src/android.rs` | Gradle + Java `WebView`, reached over JNI |
| iOS | `native/ios/` + `src/ios.rs` | Objective-C `WKWebView`, compiled by `build.rs` |
| macOS | `native/macos/` + `src/macos.rs` | Objective-C `WKWebView`, compiled by `build.rs` |
| Linux | `native/linux/` + `src/linux.rs` | C11 + GTK/WebKitGTK, compiled by `build.rs` |
| Windows | `native/windows/` + `src/windows.rs` | C++ WebView2, compiled by `build.rs` |
| OpenHarmony | `src/ohos.rs` | refuses until a native XComponent half lands |
| Web | `src/web.rs` | refuses: the browser host *is* the web view |
| Terminal | `src/tui.rs` | refuses: a terminal has no view layer |

## API

- `pub fn version() -> String`
- `pub fn enable() -> Result<(), String>`
- `pub fn disable()`
- `pub fn capability() -> i32`
- `pub fn load_url(slot: NativeSlot, url: &str) -> Result<(), String>`
- `pub fn post_message(slot: NativeSlot, json: &str) -> Result<(), String>`
- `pub fn set_message_callback(cb: Option<unsafe extern "C" fn(FloomNativeSlot, *const c_char)>)`
- `pub unsafe fn provider(host_window: *const c_void) -> Result<Box<dyn NativeViewProvider>, CProviderError>`
- `pub fn register()`

## Using it

```toml
[dependencies]
floom-plugin-webview = { path = "crates/ecosystem/plugins/floom-plugin-webview" }
```

```rust,ignore
floom_plugin_webview::register();
floom_platform::plugin("webview").enable()?;
let provider = unsafe { floom_plugin_webview::provider(window) }?;
```

## Layout

This plugin follows the native-view variant of the template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#a-plugin-that-contributes-a-native-view):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's Rust bridge.
- `native/common/floom_webview.h` — the plugin's own C ABI (enable/disable/
  capability/version plus provider registration).
- `native/common/floom_native_provider.h` — the framework's provider ABI.
- `native/<target>/` — the compiled native halves.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.
- `examples/` — the Rust examples that drive a real view.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-webview
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-webview/example/Cargo.toml
```

Device verification is separate: a real web view needs a live window and the
platform's web engine.
