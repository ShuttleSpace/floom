<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Mobile platform layering

The mobile platform layer is split into two sibling adapters:

- `floom-platform-android` — the **JNI bridge** (JavaVM, activity, `with_env`)
  with its packages (`clipboard`, `keyboard`, `safe_area`, `system_chrome`) backed
  by real JNI calls (`ClipboardManager`, `WindowInsets`,
  `WindowInsetsController`). The `jni` crate is a target-gated dependency, so
  nothing pulls it on any other target.
- `floom-platform-ios` — a UIKit staticlib with the same packages. The reads
  Rust can do on its own (`UIPasteboard`, `UIStatusBarStyle`) go through objc2;
  the facts only UIKit's host owns (safe-area insets, keyboard notifications,
  IME text) arrive through the `floom_ios_*` C ABI seam.

The OS-neutral types and pure logic they share — `TargetPlatform`,
`floom_platform::mobile::{SoftKeyboardState, ImeInput, KeyboardHeightFilter,
split_composition, android_table, SystemChromeStyle, …}` — live in
`floom-platform`, so the core ABI stays free of concrete OS dependencies.

## What is not here

Every *capability* — the battery, the connectivity, the device's identity, the
biometrics, the location, the notifications, the sensors, the share sheet, the
haptics — is a **plugin**: one crate under `crates/ecosystem/plugins/`, opt-in by
dependency, reaching the OS through a native host library. A capability plugin
that needs the Android JNI bridge reaches *down* to
`floom_platform_android::jni`; it does not own a second copy of it.

The rule is enforced rather than merely documented: the architecture gate
(`scripts/check-architecture-deps.sh`) refuses a dependency from a
`floom-platform-*` crate to a `floom-plugin-*` crate, because the platform
reaching *up* to a plugin is the direction that would put the capability back
in.

## Attribution

The dispatch pattern, the vibration FFI and the runtime platform enum are adapted
from `gpui-mobile`, used under its Apache-2.0 option. The same attribution is
kept at the top of `floom-platform/src/target_platform.rs`.
