<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-vibration

Haptic feedback and vibration.

Adapted from `gpui-mobile` (Apache-2.0).

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

- `vibrate` — Vibrate the device for the given duration in milliseconds.
- `haptic_feedback` — Trigger a specific haptic feedback pattern.
- `can_vibrate` — Check whether the device supports vibration.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-vibration = { path = "crates/ecosystem/plugins/floom-plugin-vibration" }
```

A minimal use:

```rust,ignore
if floom_plugin_vibration::can_vibrate() {
    floom_plugin_vibration::haptic_feedback(
        floom_plugin_vibration::HapticFeedback::Light,
    )?;
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum HapticFeedback {`
- `pub fn vibrate(duration_ms: u32) -> Result<(), String> {`
- `pub fn haptic_feedback(feedback: HapticFeedback) -> Result<(), String> {`
- `pub fn can_vibrate() -> bool {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_vibration.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-vibration
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-vibration/example/Cargo.toml
```
