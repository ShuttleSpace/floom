<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-background-work

Ask the OS to bring this app back at a chosen time.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `AlarmManager` through the platform JNI bridge |
| iOS | `src/lib.rs` | refuses: iOS cannot launch an app at a chosen time; `BGTaskScheduler` is opportunistic |
| desktop | `src/lib.rs` | refuses: sleep/wake belongs to the session manager, not the app |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub fn schedule_app_launch(at: SystemTime) -> Result<(), String>`
- `pub fn cancel_scheduled_app_launch() -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-background-work = { path = "crates/ecosystem/plugins/floom-plugin-background-work" }
```

```rust,ignore
floom_plugin_background_work::schedule_app_launch(
    std::time::SystemTime::now() + std::time::Duration::from_secs(300),
)?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_background_work.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-background-work
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-background-work/example/Cargo.toml
```
