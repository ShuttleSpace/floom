<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-battery

Battery level and charging state.

Adapted from `gpui-mobile` (Apache-2.0). The native arms read `UIDevice`
(iOS), `BatteryManager` (Android), IOKit's `IOPS` API (macOS), sysfs
`/sys/class/power_supply` (Linux), and Win32 `GetSystemPowerStatus`
(Windows). Each is a real measurement on the platform that has it; a
platform with no readable power supply (a desktop with no battery, a box
without the sysfs node) reports "no reading": `level: None`, not charging,
not full.

A **capability plugin**: one capability, one crate, opt-in by
dependency. The SDK does not carry it — an app that wants this
capability adds this crate, and an app that does not, does not link
it.

Where an arm cannot measure, the call reports that plainly rather
than returning a value that looks measured. That is why
[`BatteryState::level`] is an `Option`: a read that failed and a battery at
some fraction are different answers, and a caller gating UI on "low battery"
needs to be able to tell them apart.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
| macOS | ✅ Real | `src/macos.rs` |
| Windows | ✅ Real | `src/windows.rs` |
| Linux | ✅ Real | `src/linux.rs` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `battery_state` — Snapshot the current battery state.
- `is_low_power_mode` — Whether the OS is in low-power / battery-saver mode.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-battery = { path = "crates/ecosystem/plugins/floom-plugin-battery" }
```

A minimal use:

```rust,ignore
let state = floom_plugin_battery::battery_state();
match state.level {
    Some(level) => println!("{:.0}%", level * 100.0),
    None => println!("this host does not report a battery"),
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct BatteryState {`
- `pub fn battery_state() -> BatteryState {`
- `pub fn is_low_power_mode() -> bool {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_battery.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-battery
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-battery/example/Cargo.toml
```
