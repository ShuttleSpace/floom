<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-notifications

Local notifications — permission, enablement and scheduling.

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`UNUserNotificationCenter` (iOS and macOS) and `NotificationManager`
(Android). Windows and Linux refuse and name the door: WinRT's toasts need
an app identity, and Linux has no cross-desktop API (a freedesktop
notification daemon speaks D-Bus).

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
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
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

- `request_authorization` — Request permission to present notifications.
- `schedule` — Display or schedule a notification.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-notifications = { path = "crates/ecosystem/plugins/floom-plugin-notifications" }
```

A minimal use:

```rust,ignore
floom_plugin_notifications::request_authorization()?;
floom_plugin_notifications::schedule(&floom_plugin_notifications::Notification {
    title: "Build finished".into(),
    body: "the release is ready".into(),
    ..Default::default()
})?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct Notification {`
- `pub fn request_authorization() -> Result<(), String> {`
- `pub fn schedule(notification: &Notification) -> Result<(), String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_notifications.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-notifications
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-notifications/example/Cargo.toml
```
