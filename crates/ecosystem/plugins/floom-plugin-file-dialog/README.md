<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-file-dialog



## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| iOS | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| macOS | ✅ Real | `src/macos.rs` |
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

- `pick_file` — Ask the user to choose an existing file.
- `pick_save_file` — Ask the user where to save a file.
- `pick_directory` — Ask the user to choose an existing directory.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-file-dialog = { path = "crates/ecosystem/plugins/floom-plugin-file-dialog" }
```

A minimal use:

```rust,ignore
let options = floom_plugin_file_dialog::Options::new();
match floom_plugin_file_dialog::pick_file(&options)? {
    Some(path) => println!("picked {}", path.display()),
    None => println!("cancelled"),
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct Options {`
- `pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {`
- `pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {`
- `pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/plugins.md`](../../../../docs/plugins.md):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_file-dialog.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-file-dialog
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-file-dialog/example/Cargo.toml
```
