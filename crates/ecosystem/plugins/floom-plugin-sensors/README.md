<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-sensors

The device's motion and environment sensors.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `SensorManager` over JNI |
| iOS | `src/ios.rs` | real: `CMMotionManager` through objc2 |
| desktop | `src/lib.rs` | inert: no sensors, `false` for every kind |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum SensorKind { Accelerometer, Gyroscope, Magnetometer, Barometer, Proximity, AmbientLight }`
- `pub fn all() -> &'static [SensorKind]`
- `pub fn available_sensors() -> Vec<SensorKind>`
- `pub fn is_sensor_available(kind: SensorKind) -> bool`

## Using it

```toml
[dependencies]
floom-plugin-sensors = { path = "crates/ecosystem/plugins/floom-plugin-sensors" }
```

```rust,ignore
for kind in floom_plugin_sensors::SensorKind::all() {
    println!("{kind:?}: {}", floom_plugin_sensors::is_sensor_available(*kind));
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_sensors.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-sensors
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-sensors/example/Cargo.toml
```
