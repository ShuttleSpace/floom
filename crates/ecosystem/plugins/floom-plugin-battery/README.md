<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-battery

The device battery: level, charging state, and low-power mode.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `BatteryManager` over JNI |
| iOS | `src/ios.rs` | real: `UIDevice` through objc2 |
| desktop | `src/lib.rs` | inert: no portable battery API, so `level` is `None` |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct BatteryState { pub level: Option<f32>, pub is_charging: bool, pub is_full: bool }`
- `pub fn battery_state() -> BatteryState`
- `pub fn is_low_power_mode() -> bool`

## Using it

```toml
[dependencies]
floom-plugin-battery = { path = "crates/ecosystem/plugins/floom-plugin-battery" }
```

```rust,ignore
let state = floom_plugin_battery::battery_state();
match state.level {
    Some(level) => println!("{:.0}%", level * 100.0),
    None => println!("this host does not report a battery"),
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_battery.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-battery
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-battery/example/Cargo.toml
```
