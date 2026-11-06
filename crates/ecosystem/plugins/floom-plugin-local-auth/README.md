<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-local-auth

Biometric and device-owner authentication.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `BiometricPrompt` over JNI |
| iOS | `src/ios.rs` | real: `LAContext` through objc2 |
| desktop | `src/lib.rs` | inert: `BiometricKind.None`, `authenticate` refuses |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum BiometricKind { None, Fingerprint, Face, Iris }`
- `pub fn available_biometric() -> BiometricKind`
- `pub fn authenticate(reason: &str) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-local-auth = { path = "crates/ecosystem/plugins/floom-plugin-local-auth" }
```

```rust,ignore
match floom_plugin_local_auth::available_biometric() {
    floom_plugin_local_auth::BiometricKind::None => println!("no biometric sensor"),
    kind => println!("sensor: {kind:?}"),
}
floom_plugin_local_auth::authenticate("unlock the vault")?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_local_auth.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-local-auth
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-local-auth/example/Cargo.toml
```
