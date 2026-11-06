<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->
# Native halves

| Target | Arm | What it does |
|---|---|---|
| Android | `android/` + `src/android.rs` | Java `FloomCameraView`, driven over JNI |
| iOS | `native/ios/` + `src/ios.rs` | Objective-C `AVFoundation`, compiled by `build.rs` |
| macOS | `native/macos/` + `src/macos.rs` | Objective-C `AVFoundation`, compiled by `build.rs` |
| Linux | `native/linux/` + `src/linux.rs` | C11 + GStreamer/X11, compiled by `build.rs` |
| Windows | `native/windows/` + `src/windows.rs` | C++ `MediaFoundation`, compiled by `build.rs` |
| OpenHarmony | `native/ohos/` + `src/ohos.rs` | C++ half, compiled by `build.rs` |
| Web | `src/web.rs` | pure Rust `getUserMedia`; no native source |
| Terminal | `src/tui.rs` | refuses: a terminal has no camera |

`native/common/floom_camera.h` is the plugin's own C ABI; the native halves
implement it, and `src/<target>.rs` calls it through `extern "C"`.
`native/common/floom_native_provider.h` is the framework's provider ABI, copied
in by `build.rs` from `crates/native/floom-native-ffi/include/` so the two
cannot drift.

The generic layout is documented in
[`docs/platform-and-plugins.md`](../../../../../docs/platform-and-plugins.md#the-layout).
