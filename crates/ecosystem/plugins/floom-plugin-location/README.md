<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-location

The device's location and its authorisation state.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `LocationManager` over JNI |
| iOS | `src/ios.rs` | real: `CLLocationManager` through objc2 |
| desktop | `src/lib.rs` | inert: `NotDetermined`, `false`, and named refusals |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum LocationAuthorization { NotDetermined, Restricted, Denied, AuthorizedWhenInUse, AuthorizedAlways }`
- `pub struct Location { pub latitude: f64, pub longitude: f64, pub altitude: f64, pub horizontal_accuracy: f64, pub vertical_accuracy: f64, pub timestamp: i64 }`
- `pub fn authorization_status() -> LocationAuthorization`
- `pub fn is_location_enabled() -> bool`
- `pub fn request_authorization() -> Result<(), String>`
- `pub fn current_location() -> Result<Location, String>`

## Using it

```toml
[dependencies]
floom-plugin-location = { path = "crates/ecosystem/plugins/floom-plugin-location" }
```

```rust,ignore
floom_plugin_location::request_authorization()?;
let fix = floom_plugin_location::current_location()?;
println!("{}, {}", fix.latitude, fix.longitude);
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_location.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-location
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-location/example/Cargo.toml
```
