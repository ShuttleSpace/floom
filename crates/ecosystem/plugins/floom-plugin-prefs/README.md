<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-prefs

The OS's own key-value store, for user preferences.

This is `shared_preferences`'s question, and the catalog's A5
(`docs/plugins.md`): one string-to-string map per store name,
whose *where* is the platform's choice — `SharedPreferences` on Android,
`NSUserDefaults` on Apple, a file in the user's config directory on Linux
and Windows (the same shape Flutter's own `shared_preferences_linux` and
`_windows` implementations chose, for the same reason: those platforms
have no OS preferences store to ask).

A **capability plugin**: one capability, one crate, opt-in by dependency.

# The split from `floom-persistence` — read this before using either

These are different questions, and the answer decides which crate you
want:

| | `floom-persistence` | this crate |
|---|---|---|
| Where | A backend *you* hand it (`FileStore` at a path you chose, `MemoryStore`) | The OS's own store — nowhere to configure |
| What for | Session state, restorable UI, snapshots | User settings that outlive every session |
| Restores | `RestorationSnapshot`, versioned capture/replay | Nothing — a preference has no lifecycle |

`floom-persistence` does not know where your app lives; pair it with
`floom-plugin-paths` when you need that. This crate *is* the "where":
asking it to also take a path would be asking the OS's store to not be
the OS's store.

# What `set` promises

`Ok(())` means the value is stored — visible to the next `get` and to the
next launch. On Android that is `commit()` (the synchronous write), not
`apply()` (fire-and-forget): `apply()` returning immediately would make
`Ok` a claim about a write that may not have happened. On Apple the
suite's in-memory cache is updated synchronously and the plist flush is
the system's; on the file arm the write is temp-file-then-rename, so a
reader never sees a half-written store.

# Store names, keys, values

A store name is an identifier, not a path: non-empty, no separators, no
NUL. Keys and values are arbitrary UTF-8 except NUL (which no arm can
carry — it breaks every one of the three transports). The file arm
percent-encodes beyond that, transparently.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
| Windows | ✅ Real | `src/file.rs` |
| Linux | ✅ Real | `src/file.rs` |
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
floom-plugin-prefs = { path = "crates/ecosystem/plugins/floom-plugin-prefs" }
```

A minimal use:

```rust,ignore
let prefs = floom_plugin_prefs::Preferences::open("settings")?;
prefs.set("theme", "dark")?;
assert_eq!(prefs.get("theme").as_deref(), Some("dark"));
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct Preferences {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_prefs.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-prefs
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-prefs/example/Cargo.toml
```
