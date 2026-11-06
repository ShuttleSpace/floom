<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-network-info

The current Wi-Fi network's name, and this device's address on it.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `WifiManager` over JNI |
| iOS / macOS | `src/apple.rs` | real: `CNCopyCurrentNetworkInfo` / `CWWiFiClient` |
| Linux | `src/lib.rs` | refuses `wifi_info`: NetworkManager needs a D-Bus conversation |
| Windows | `src/lib.rs` | refuses `wifi_info`: `WlanQueryInterface` is a named follow-up |
| all targets | `src/lib.rs` | `local_ip` is real everywhere: a route lookup through a UDP socket |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct WifiInfo { pub ssid: Option<String>, pub bssid: Option<String> }`
- `pub fn local_ip() -> Option<IpAddr>`
- `pub fn wifi_info() -> Result<WifiInfo, String>`

## Using it

```toml
[dependencies]
floom-plugin-network-info = { path = "crates/ecosystem/plugins/floom-plugin-network-info" }
```

```rust,ignore
println!("local address: {:?}", floom_plugin_network_info::local_ip());
match floom_plugin_network_info::wifi_info() {
    Ok(info) => println!("ssid: {:?}", info.ssid),
    Err(reason) => println!("wifi_info: {reason}"),
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_network_info.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-network-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-network-info/example/Cargo.toml
```
