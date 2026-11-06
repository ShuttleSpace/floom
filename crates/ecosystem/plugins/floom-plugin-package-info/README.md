<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-package-info

The running app's own identity: name, package id, version, build.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `PackageManager` over JNI |
| iOS / macOS | `src/apple.rs` | real: the main bundle's `Info.plist` |
| desktop | `src/lib.rs` | inert: every field is `None` |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct AppInfo { pub name: Option<String>, pub package_name: Option<String>, pub version: Option<String>, pub build_number: Option<String> }`
- `pub fn app_info() -> AppInfo`

## Using it

```toml
[dependencies]
floom-plugin-package-info = { path = "crates/ecosystem/plugins/floom-plugin-package-info" }
```

```rust,ignore
let info = floom_plugin_package_info::app_info();
println!("{:?} {:?}", info.name, info.version);
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_package_info.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-package-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-package-info/example/Cargo.toml
```
