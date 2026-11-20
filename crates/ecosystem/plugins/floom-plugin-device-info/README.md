<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-device-info

Device identity and environment facts (model, OS, locale, …).

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`UIDevice` (iOS) and `Build` / `SystemProperty` (Android); the desktop arms
read what `std` already exposes (`/etc/os-release`, `SystemVersion.plist`,
the locale and host-name environment) — see `desktop.rs`. Fields with no
`std` door (`model`, `manufacturer` on a desktop) stay empty rather than
being invented.

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
| macOS | ✅ Real | `src/desktop.rs` |
| Windows | ✅ Real | `src/desktop.rs` |
| Linux | ✅ Real | `src/desktop.rs` |
| OpenHarmony | ⚪ Inert fallback | `src/ohos.rs` returns the documented default (honest degrade) |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `device_name` — A short, human-readable device name (e.g. the user's phone name).


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-device-info = { path = "crates/ecosystem/plugins/floom-plugin-device-info" }
```

A minimal use:

```rust,ignore
let info = floom_plugin_device_info::device_info();
println!("{} {} ({})", info.manufacturer, info.model, info.os_version);
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct DeviceInfo {`
- `pub fn device_name() -> String {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_device-info.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-device-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-device-info/example/Cargo.toml
```
