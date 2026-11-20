<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->
# Native halves

| Target | Dir | What it does |
|---|---|---|
| Android | `android/` + `src/android.rs` | Gradle + Java `WebView`, reached over JNI |
| iOS | `native/ios/` + `src/ios.rs` | Objective-C `WKWebView` |
| macOS | `native/macos/` + `src/macos.rs` | Objective-C `WKWebView` |
| Linux | `native/linux/` + `src/linux.rs` | C11 + GTK/WebKitGTK |
| Windows | `native/windows/` + `src/windows.rs` | C++ WebView2 |
| OpenHarmony | `src/ohos.rs` | refuses until a native XComponent half lands |
| Web / Terminal | `src/web.rs` / `src/tui.rs` | refuse: no separate native view to contribute |

`native/common/floom_webview.h` is the plugin's own C ABI; the compiled halves
implement it, and `src/<target>.rs` calls it through `extern "C"`.
`native/common/floom_native_provider.h` is the framework's provider ABI, copied
in by `build.rs` from `crates/native/floom-native-ffi/include/` so the two
cannot drift.

The generic layout is documented in
[`docs/plugins.md`](../../../../../docs/plugins.md).
