<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-device-info

Device identity and environment facts.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `android.os.Build` over JNI |
| iOS | `src/ios.rs` | real: `UIDevice` through objc2 |
| desktop | `src/lib.rs` | inert: `DeviceInfo::default()` and `"unknown"` |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct DeviceInfo { pub model: String, pub manufacturer: String, pub os_name: String, pub os_version: String, pub sdk_version: Option<i32>, pub is_physical_device: bool, pub locale: String }`
- `pub fn device_info() -> DeviceInfo`
- `pub fn device_name() -> String`

## Using it

```toml
[dependencies]
floom-plugin-device-info = { path = "crates/ecosystem/plugins/floom-plugin-device-info" }
```

```rust,ignore
let info = floom_plugin_device_info::device_info();
println!("{} {} ({})", info.manufacturer, info.model, info.os_version);
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_device_info.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-device-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-device-info/example/Cargo.toml
```
