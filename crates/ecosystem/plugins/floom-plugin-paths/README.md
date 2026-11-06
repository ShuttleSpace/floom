<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-paths

Where this app may write: app support, documents, cache, downloads, external storage.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `Context` directories over JNI |
| iOS / macOS | `src/apple.rs` | real: `NSFileManager` search paths |
| Linux / Windows | `src/desktop.rs` | real: the platform's known folders |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum AppDirectory { ApplicationSupport, Documents, Cache, Downloads }`
- `pub fn external_files_dir() -> Option<PathBuf>`
- `pub fn app_dir(dir: AppDirectory) -> Option<PathBuf>`

## Using it

```toml
[dependencies]
floom-plugin-paths = { path = "crates/ecosystem/plugins/floom-plugin-paths" }
```

```rust,ignore
let cache = floom_plugin_paths::app_dir(floom_plugin_paths::AppDirectory::Cache);
println!("cache: {cache:?}");
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_paths.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-paths
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-paths/example/Cargo.toml
```
