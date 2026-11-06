<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# Native halves

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `AlarmManager` through the platform JNI bridge |
| iOS | `src/lib.rs` | refuses: iOS cannot launch an app at a chosen time; `BGTaskScheduler` is opportunistic |
| desktop | `src/lib.rs` | refuses: sleep/wake belongs to the session manager, not the app |

This plugin keeps its platform code in Rust (`Shape A` in
[`docs/plugin-catalog.md` §8.1](../../../../../docs/plugin-catalog.md#81-新插件走哪套形状教义与代码已分叉)),
so `build.rs` compiles no native source and `native/common/` holds only the C
ABI contract — the shape a native half would implement if one is added. The
Rust arms are listed in the table above; the contract header is checked against
the crate name by `scripts/check-plugin-templates.sh`.

The generic layout is documented in
[`docs/platform-and-plugins.md`](../../../../../docs/platform-and-plugins.md#the-layout).
