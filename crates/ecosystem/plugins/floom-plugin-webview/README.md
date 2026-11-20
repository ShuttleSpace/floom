<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-webview

The `webview` plugin — a capability that is a **native view**.

The capability is a view the framework does not paint: a `WKWebView` on
macOS, a `WebView` on Android or Linux, a WebView2 on Windows, and so on.
What the view *shows* is the web's own business; this crate owns the
*capability* (create / place / destroy a native view, drive a JS bridge)
and nothing about any one platform.

# Layout

`src/` is the **unified API**. It is the only surface an example or host
touches: [`enable`], [`provider`], [`load_url`], [`post_message`],
[`set_message_callback`], [`diagnostics`], [`register`]. None of it names a
platform.

Each platform is a **peer project to `src/`**, implemented in whatever
native code it needs and reached through FFI. All compiled halves live
under `native/<platform>/`:

| dir              | platform | native half                                   |
|------------------|----------|-----------------------------------------------|
| `native/macos/`  | macOS    | Objective-C (`WKWebView`), built by `build.rs` |
| `native/ios/`    | iOS      | Objective-C (`WKWebView`), built by `build.rs` |
| `native/linux/`  | Linux    | C11 + GTK/WebKitGTK, built by `build.rs`       |
| `native/windows/`| Windows  | C++ (WebView2), built by `build.rs`            |
| `android/`       | Android  | Gradle + Java (`WebView`)                      |

`src/web.rs` and `src/tui.rs` are honest refusals: a browser host *is* the
web view and a terminal has no view layer, so there is no separate
native-view capability to contribute — the plugin says so rather than faking
one. `src/ohos.rs` is scaffolded as a refusal until a native XComponent half
lands.

# Two contracts

| | asks | shape |
|---|---|---|
| the plugin's own ABI (`native/common/floom_webview.h`) | is the capability available? | four functions; the plugin is a singleton |
| the provider ABI (`floom-native-ffi`) | how is the native view created, placed and torn down? | a versioned struct of function pointers; the provider is an instance per window |

A plugin whose capability is a native view implements both.

# Which side owns what

The plugin owns the *capability*; the host owns the *transport*. The native
half reaches the host's window through the provider's `attach`, and the
framework drives the provider through the ordinary Rust [`NativeViewProvider`]
trait — so the controller's lifecycle invariants apply to this plugin exactly
as they apply to a Rust one.

# How the two halves find each other

On macOS, the native half (Objective-C) hands its vtable to this crate from a
constructor, and this crate hands the provider to the host on request. On
every other platform the binding is a Rust struct that drives a native
control through that platform's own bridge. Nothing is looked up by name at
runtime: the symbols resolve when the crate is linked.

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

- `register` — Registers this plugin with the process-wide table, using the active


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-webview = { path = "crates/ecosystem/plugins/floom-plugin-webview" }
```

A minimal use:

```rust,ignore
floom_plugin_webview::register();
floom_platform::plugin("webview").enable()?;
let provider = unsafe { floom_plugin_webview::provider(window) }?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn register() {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_webview.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-webview
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-webview/example/Cargo.toml
```
