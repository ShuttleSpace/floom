<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-secure-storage

Secrets in the OS's own protected store.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `EncryptedSharedPreferences` over JNI |
| iOS / macOS | `src/apple.rs` | real: Keychain |
| Linux | `src/linux.rs` | real: libsecret through the secret-service D-Bus API |
| Windows | `src/windows.rs` | real: DPAPI |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct SecureStorage`
- `pub fn open(service: &str) -> Result<SecureStorage, String>`
- `pub fn get(&self, key: &str) -> Result<Option<String>, String>`
- `pub fn set(&self, key: &str, value: &str) -> Result<(), String>`
- `pub fn remove(&self, key: &str) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-secure-storage = { path = "crates/ecosystem/plugins/floom-plugin-secure-storage" }
```

```rust,ignore
let store = floom_plugin_secure_storage::SecureStorage::open("auth")?;
store.set("token", "…")?;
assert!(store.get("token")?.is_some());
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_secure_storage.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-secure-storage
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-secure-storage/example/Cargo.toml
```
