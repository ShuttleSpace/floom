<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-share

The platform share sheet.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| Android | `src/android.rs` | real: `ACTION_SEND` over JNI |
| iOS | `src/ios.rs` | real: `UIActivityViewController` through objc2 |
| macOS | `src/macos.rs` | real: `NSSharingServicePicker` |
| Windows | `src/lib.rs` | refuses: `DataTransferManager` needs the app's HWND |
| Linux | `src/lib.rs` | refuses: no cross-desktop share API |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub fn share_text(text: &str) -> Result<(), String>`
- `pub fn share_files(paths: &[&str]) -> Result<(), String>`

## Using it

```toml
[dependencies]
floom-plugin-share = { path = "crates/ecosystem/plugins/floom-plugin-share" }
```

```rust,ignore
floom_plugin_share::share_text("look at this")?;
floom_plugin_share::share_files(&["/tmp/report.pdf"])?;
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_share.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-share
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-share/example/Cargo.toml
```
