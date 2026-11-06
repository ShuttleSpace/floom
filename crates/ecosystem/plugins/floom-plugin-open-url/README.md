<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-open-url

Open a URL with the application the OS routes its scheme to.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `ACTION_VIEW` over JNI |
| iOS / macOS | `src/ios.rs` / `src/macos.rs` | real: `UIApplication` / `NSWorkspace` |
| Linux | `src/linux.rs` | real: `xdg-open` |
| Windows | `src/windows.rs` | real: `ShellExecuteW` |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub fn open_url(url: &str) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-open-url = { path = "crates/ecosystem/plugins/floom-plugin-open-url" }
```

```rust,ignore
floom_plugin_open_url::open_url("https://example.com")?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_open_url.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-open-url
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-open-url/example/Cargo.toml
```
