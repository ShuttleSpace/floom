<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-wakelock

Keep the screen awake while the app is in the foreground.

This is `wakelock_plus`'s question, and the catalog's A6
(`docs/plugin-catalog.md`): a video is playing, a recipe is on screen
with hands busy, a turn-by-turn map is showing — the app asks the OS not
to sleep the display, and *only* the display; this crate does not touch
CPU or network idle policy, which are different questions with different
permissions.

A **capability plugin**: one capability, one crate, opt-in by dependency.

# The state is the OS's, where the OS lets us read it

`keep_screen_on(true)` is a **true launch**: `Ok(())` claims the request
was accepted, not that the pixels stayed lit — no arm can observe that.
[`is_keep_screen_on`] reads the truth back where the platform exposes a
readable truth (the window flag on Android, `idleTimerDisabled` on iOS,
`IOPMAssertionCopyProperties` on macOS) and tracks it where it does not
(Windows — `SetThreadExecutionState` has no query; asking for the
current state would change it, which is the one thing a read here must
not do). Linux has no API at all and says so.

# Why enable/disable rather than acquire/release handles

A handle would make the caller hold something that must be balanced on
every path — and the OS already balances it: the flag clears with the
window, the assertion with the process, the execution state with the
thread. A boolean API matches what the platform actually guarantees,
which is "while your app is alive and asking".

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
| macOS | ✅ Real | `src/macos.rs` |
| Windows | ✅ Real | `src/windows.rs` |
| Linux | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `keep_screen_on` — Ask the OS to keep the display awake (`true`) or stop asking (`false`).
- `is_keep_screen_on` — Whether the display is currently being kept awake by this app's request.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-wakelock = { path = "crates/ecosystem/plugins/floom-plugin-wakelock" }
```

A minimal use:

```rust,ignore
floom_plugin_wakelock::keep_screen_on(true)?;
// …
floom_plugin_wakelock::keep_screen_on(false)?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn keep_screen_on(on: bool) -> Result<(), String> {`
- `pub fn is_keep_screen_on() -> bool {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_wakelock.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-wakelock
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-wakelock/example/Cargo.toml
```
