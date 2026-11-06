<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-connectivity

Network reachability and connection type.

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`SystemConfiguration` (iOS) and `ConnectivityManager` (Android). The host
arm reports "no connection".

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

- `connectivity_status` — The full connectivity status.
- `is_connected` — Whether any network path is currently usable.
- `is_metered` — Whether the active connection is metered (e.g. cellular, capped).


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-connectivity = { path = "crates/ecosystem/plugins/floom-plugin-connectivity" }
```

A minimal use:

```rust,ignore
let status = floom_plugin_connectivity::connectivity_status();
if status.is_connected && status.is_metered {
    println!("on a metered {:?} link", status.connection_type);
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum ConnectionType {`
- `pub struct ConnectivityStatus {`
- `pub fn connectivity_status() -> ConnectivityStatus {`
- `pub fn is_connected() -> bool {`
- `pub fn is_metered() -> bool {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_connectivity.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-connectivity
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-connectivity/example/Cargo.toml
```
