<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-notifications

The device's notification authorisation and scheduling.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `NotificationManager` over JNI |
| iOS / macOS | `src/apple.rs` | real: `UNUserNotificationCenter` through objc2 |
| Windows | `src/lib.rs` | refuses: toasts need an app identity and the WinRT crate |
| Linux | `src/lib.rs` | refuses: no cross-desktop API; the freedesktop daemon is D-Bus |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct Notification { pub title: String, pub body: String, pub badge: Option<u32>, pub sound: bool, pub delay_seconds: Option<u64> }`
- `pub fn are_notifications_enabled() -> bool`
- `pub fn request_authorization() -> Result<(), String>`
- `pub fn schedule(notification: &Notification) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-notifications = { path = "crates/ecosystem/plugins/floom-plugin-notifications" }
```

```rust,ignore
floom_plugin_notifications::request_authorization()?;
floom_plugin_notifications::schedule(&floom_plugin_notifications::Notification {
    title: "Build finished".into(),
    body: "the release is ready".into(),
    ..Default::default()
})?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_notifications.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-notifications
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-notifications/example/Cargo.toml
```
