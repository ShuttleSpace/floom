<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-prefs

The OS's own key-value store for user preferences.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `SharedPreferences` over JNI |
| iOS / macOS | `src/apple.rs` | real: `NSUserDefaults` |
| Linux / Windows | `src/file.rs` | real: a config file in the app's config directory |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct Preferences`
- `pub fn open(store: &str) -> Result<Preferences, String>`
- `pub fn get(&self, key: &str) -> Option<String>`
- `pub fn set(&self, key: &str, value: &str) -> Result<(), String>`
- `pub fn remove(&self, key: &str) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-prefs = { path = "crates/ecosystem/plugins/floom-plugin-prefs" }
```

```rust,ignore
let prefs = floom_plugin_prefs::Preferences::open("settings")?;
prefs.set("theme", "dark")?;
assert_eq!(prefs.get("theme").as_deref(), Some("dark"));
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_prefs.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-prefs
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-prefs/example/Cargo.toml
```
