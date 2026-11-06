<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-share

Share sheet — hand text or files to other apps.

Adapted from `gpui-mobile` (Apache-2.0). The arms present
`NSSharingServicePicker` (macOS), `UIActivityViewController` (iOS) and an
`Intent.ACTION_SEND` / `ACTION_SEND_MULTIPLE` chooser (Android). Windows
and Linux refuse and name the door: WinRT's `DataTransferManager` needs the
app's `HWND`, and Linux has no cross-desktop share API at all.

A **capability plugin**: one capability, one crate, opt-in by
dependency. The SDK does not carry it — an app that wants this
capability adds this crate, and an app that does not, does not link
it.

Where an arm is not wired yet, the call reports that plainly rather
than returning a value that looks measured.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
| macOS | ✅ Real | `src/macos.rs` |
| Windows | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Linux | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `share_text` — Present the system share sheet for a block of text.
- `share_files` — Present the system share sheet for one or more local file paths.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-share = { path = "crates/ecosystem/plugins/floom-plugin-share" }
```

A minimal use:

```rust,ignore
floom_plugin_share::share_text("look at this")?;
floom_plugin_share::share_files(&["/tmp/report.pdf"])?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn share_text(text: &str) -> Result<(), String> {`
- `pub fn share_files(paths: &[&str]) -> Result<(), String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_share.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-share
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-share/example/Cargo.toml
```
