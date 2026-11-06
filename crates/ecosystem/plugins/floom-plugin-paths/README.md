<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-paths

The app's own directories: application support, documents, cache, downloads.

This is `path_provider`'s question — "where may this app write" — and it is
the first thing almost every app that persists anything needs, which is why
it opens the catalog's Tier A (`docs/plugin-catalog.md`, A1).

A **capability plugin**: one capability, one crate, opt-in by dependency.

# Why every arm is a real read

Unlike the nine mobile-only plugins, this plugin has no host-inert arm at
all. On the desktop the *host conventions are the source of truth*: Linux
resolves `XDG_DATA_HOME`/`XDG_CACHE_HOME`, macOS answers
`~/Library/Application Support` and `~/Library/Caches`, Windows answers
`%APPDATA%` and `%LOCALAPPDATA%` — the same questions the phone arms ask
the OS, answered by the same kind of authority. An `Err` here would be a
lie about the desktop, and a "no directories" default would be worse: it
would look like an answer while being none.

The one place with no answer is a platform the crate does not cover (say,
wasm or a terminal target): there [`app_dir`] reports `None`, which is
"this platform has nothing to say", not "an empty path".

# What this crate deliberately does not do

It does not create the directories it reports — the OS convention says
where they belong, not that they exist (`dirs` itself documents that
`data_dir` may point at a non-existent path on some platforms). Creation is
policy: an app may want permissions, may want a versioned subdirectory, or
may not want the directory at all until first write. Callers combine this
with `std::fs::create_dir_all` and own the failure.

It also does not answer "where are the user's *Documents* on a phone".
Mobile `Documents` is app-private storage — the app's own files, not the
user-visible folder — because that is what `path_provider`'s
`documentsDirectory` means there. A shared user-documents picker is a
different capability (file-dialog, A3 in the catalog) and deliberately not
this crate's promise.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
| Windows | ✅ Real | `src/desktop.rs` |
| Linux | ✅ Real | `src/desktop.rs` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `external_files_dir` — The app's own external storage on Android: the app-scoped directory on
- `app_dir` — Where the app's own `[AppDirectory::Downloads]` files live, if this platform


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-paths = { path = "crates/ecosystem/plugins/floom-plugin-paths" }
```

A minimal use:

```rust,ignore
let cache = floom_plugin_paths::app_dir(floom_plugin_paths::AppDirectory::Cache);
println!("cache: {cache:?}");
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum AppDirectory {`
- `pub fn external_files_dir() -> Option<PathBuf> {`
- `pub fn app_dir(dir: AppDirectory) -> Option<PathBuf> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_paths.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-paths
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-paths/example/Cargo.toml
```
