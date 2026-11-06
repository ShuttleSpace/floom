<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-sensors

Motion and environment sensors — availability queries.

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`CMMotionManager` (iOS) and `SensorManager` (Android).

This package answers **which sensors exist**, and nothing else. Reading a
sensor is a subscription concern: Android delivers samples through a
`SensorEventListener` and iOS through `startAccelerometerUpdatesToQueue:`,
both of which are callbacks this synchronous API has nowhere to put. There is
deliberately no `reading()` function rather than one that returns `None` —
a `None` from a wired arm and a `None` from an unwired one look identical to
the caller, which is the confusion this plugin family exists to remove.

`available_sensors` returns the full set of kinds the package knows how to
query, filtered to those the device actually has; `is_sensor_available`
answers the same question for one kind.

A **capability plugin**: one capability, one crate, opt-in by
dependency. The SDK does not carry it — an app that wants this
capability adds this crate, and an app that does not, does not link
it.

Where an arm is not wired yet, the call reports that plainly rather
than returning a value that looks measured.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
| macOS | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Windows | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Linux | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `available_sensors` — The set of sensors present on the device.
- `is_sensor_available` — Whether a specific sensor is present.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-sensors = { path = "crates/ecosystem/plugins/floom-plugin-sensors" }
```

A minimal use:

```rust,ignore
for kind in floom_plugin_sensors::SensorKind::all() {
    println!("{kind:?}: {}", floom_plugin_sensors::is_sensor_available(*kind));
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum SensorKind {`
- `pub fn available_sensors() -> Vec<SensorKind> {`
- `pub fn is_sensor_available(kind: SensorKind) -> bool {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_sensors.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-sensors
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-sensors/example/Cargo.toml
```
