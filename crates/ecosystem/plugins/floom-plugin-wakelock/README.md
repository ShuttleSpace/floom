<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-wakelock

Keep the screen awake while the app is in the foreground.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `FLAG_KEEP_SCREEN_ON` over JNI |
| iOS | `src/ios.rs` | real: `UIApplication.isIdleTimerDisabled` |
| macOS | `src/macos.rs` | real: `IOPMAssertion` |
| Windows | `src/windows.rs` | real: `SetThreadExecutionState` |
| Linux | `src/lib.rs` | refuses: no cross-desktop keep-awake API |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub fn keep_screen_on(on: bool) -> Result<(), String>`
- `pub fn is_keep_screen_on() -> bool`

## Using it

```toml
[dependencies]
floom-plugin-wakelock = { path = "crates/ecosystem/plugins/floom-plugin-wakelock" }
```

```rust,ignore
floom_plugin_wakelock::keep_screen_on(true)?;
// …
floom_plugin_wakelock::keep_screen_on(false)?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_wakelock.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-wakelock
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-wakelock/example/Cargo.toml
```
