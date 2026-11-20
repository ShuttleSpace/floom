<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-secure-storage

Secrets that outlive the process and must not sit in a readable file:
tokens, passwords, private keys.

This is `flutter_secure_storage`'s question. It is **not** `prefs` with a
different name — `prefs` stores plaintext in the OS's preferences store,
which is the thing this crate exists to avoid. The split Flutter draws
between `shared_preferences` and `flutter_secure_storage` is the same one.

| | `floom-plugin-prefs` | this crate |
|---|---|---|
| Readable by | anything that can read the user's files | only code running as this user (and, on Apple, only this app) |
| On disk | the OS's plaintext store | keychain item / Keystore-sealed / DPAPI-sealed / libsecret |
| For | settings | secrets |

# What `set` promises

`Ok(())` means the platform store accepted the value: the next `get`, in
this process or the next launch, returns it. On Android the ciphertext is
written with `commit()` for the same reason `prefs` does.

# What `get` promises

`Ok(None)` means *absent*; `Err` means the store could not be asked at all
(a locked keychain, no `secret-tool`, a Keystore key invalidated by a
restore). Folding those two into one `None` would report a broken store as
"no secret here", which is the one wrong answer worth distinguishing.

# Arms

- Apple: a keychain generic-password item, `service` = the store name,
`account` = the key.
- Android: an AES-256-GCM key held by the Android Keystore (alias derived
from the store name), ciphertext in `SharedPreferences`.
- Linux: libsecret through `secret-tool`.
- Windows: a file in the user's config directory, sealed with DPAPI.

# Store names, keys, values

A store name is an identifier: non-empty, no separators, no NUL — the same
rule `prefs` applies, because Windows turns it into a filename. Keys and
values are arbitrary UTF-8 except NUL.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
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

_See the API section below._


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-secure-storage = { path = "crates/ecosystem/plugins/floom-plugin-secure-storage" }
```

A minimal use:

```rust,ignore
let store = floom_plugin_secure_storage::SecureStorage::open("auth")?;
store.set("token", "…")?;
assert!(store.get("token")?.is_some());
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct SecureStorage {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_secure-storage.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-secure-storage
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-secure-storage/example/Cargo.toml
```
