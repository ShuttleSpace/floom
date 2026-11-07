<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-local-auth

Local authentication — biometric unlock (Touch ID / Face ID / fingerprint).

Adapted from `gpui-mobile` (Apache-2.0). The native arms read
`LocalAuthentication` (iOS) and `BiometricPrompt` (Android). The host arm
reports no biometric and refuses to authenticate.

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
| macOS | ✅ Real | `src/macos.rs` |
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

- `available_biometric` — The biometric kind available on this device, if any.
- `authenticate` — Prompt the user to authenticate with biometrics.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-local-auth = { path = "crates/ecosystem/plugins/floom-plugin-local-auth" }
```

A minimal use:

```rust,ignore
match floom_plugin_local_auth::available_biometric() {
    floom_plugin_local_auth::BiometricKind::None => println!("no biometric sensor"),
    kind => println!("sensor: {kind:?}"),
}
floom_plugin_local_auth::authenticate("unlock the vault")?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum BiometricKind {`
- `pub fn available_biometric() -> BiometricKind {`
- `pub fn authenticate(reason: &str) -> Result<(), String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_local-auth.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-local-auth
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-local-auth/example/Cargo.toml
```
