<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# Native halves

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `WifiManager` over JNI |
| iOS / macOS | `src/apple.rs` | real: `CNCopyCurrentNetworkInfo` / `CWWiFiClient` |
| Linux | `src/lib.rs` | refuses `wifi_info`: NetworkManager needs a D-Bus conversation |
| Windows | `src/lib.rs` | refuses `wifi_info`: `WlanQueryInterface` is a named follow-up |
| all targets | `src/lib.rs` | `local_ip` is real everywhere: a route lookup through a UDP socket |

This plugin keeps its platform code in Rust (`Shape A` in
[`docs/plugin-catalog.md` §8.1](../../../../../docs/plugin-catalog.md#81-新插件走哪套形状教义与代码已分叉)),
so `build.rs` compiles no native source and `native/common/` holds only the C
ABI contract — the shape a native half would implement if one is added. The
Rust arms are listed in the table above; the contract header is checked against
the crate name by `scripts/check-plugin-templates.sh`.

The generic layout is documented in
[`docs/platform-and-plugins.md`](../../../../../docs/platform-and-plugins.md#the-layout).
