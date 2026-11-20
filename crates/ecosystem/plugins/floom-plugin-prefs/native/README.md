<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# Native halves

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `SharedPreferences` over JNI |
| iOS / macOS | `src/apple.rs` | real: `NSUserDefaults` |
| Linux / Windows | `src/file.rs` | real: a config file in the app's config directory |

This plugin keeps its platform code in Rust (`Shape A` in
[`docs/plugins.md` §8.1](../../../../../docs/plugins.md#4-新增插件的-checklist)),
so `build.rs` compiles no native source and `native/common/` holds only the C
ABI contract — the shape a native half would implement if one is added. The
Rust arms are listed in the table above; the contract header is checked against
the crate name by `scripts/check-plugin-templates.sh`.

The generic layout is documented in
[`docs/plugins.md`](../../../../../docs/plugins.md).
