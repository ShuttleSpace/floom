<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-connectivity

The device's network connectivity: type, connected, metered.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `ConnectivityManager` over JNI |
| iOS | `src/ios.rs` | real: `SCNetworkReachability` through objc2 |
| desktop | `src/lib.rs` | inert: `ConnectionType.None`, `false`, `false` |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum ConnectionType { None, Wifi, Mobile, Ethernet, Bluetooth, Other }`
- `pub struct ConnectivityStatus { pub connection_type: ConnectionType, pub is_connected: bool, pub is_metered: bool }`
- `pub fn connection_type() -> ConnectionType`
- `pub fn connectivity_status() -> ConnectivityStatus`
- `pub fn is_connected() -> bool`
- `pub fn is_metered() -> bool`

## Using it

```toml
[dependencies]
floom-plugin-connectivity = { path = "crates/ecosystem/plugins/floom-plugin-connectivity" }
```

```rust,ignore
let status = floom_plugin_connectivity::connectivity_status();
if status.is_connected && status.is_metered {
    println!("on a metered {:?} link", status.connection_type);
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_connectivity.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-connectivity
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-connectivity/example/Cargo.toml
```
