<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-location

Geolocation — authorization, availability and a one-shot position read.

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`CLLocationManager` (iOS) and `LocationManager` (Android). The host arm
refuses every call that needs a real position.

Note: continuous location streaming is a separate concern (a subscription
with callbacks); this package only exposes the synchronous reads plus the
permission gates an app must check first. What "synchronous" costs is
documented per call: `current_location` on Android is the OS's last-known
cached fix, not a fresh one, and on iOS there is no synchronous route at all.

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

- `is_location_enabled` — Whether the device has any location provider enabled at the OS level.
- `request_authorization` — Request location authorization. Returns `Err` if already denied or the
- `current_location` — Best-effort one-shot position read.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-location = { path = "crates/ecosystem/plugins/floom-plugin-location" }
```

A minimal use:

```rust,ignore
floom_plugin_location::request_authorization()?;
let fix = floom_plugin_location::current_location()?;
println!("{}, {}", fix.latitude, fix.longitude);
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum LocationAuthorization {`
- `pub struct Location {`
- `pub fn is_location_enabled() -> bool {`
- `pub fn request_authorization() -> Result<(), String> {`
- `pub fn current_location() -> Result<Location, String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_location.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-location
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-location/example/Cargo.toml
```
