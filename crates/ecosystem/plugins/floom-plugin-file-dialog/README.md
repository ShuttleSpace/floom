<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-file-dialog

Ask the user for a file path: open, save-as, or pick a directory.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| macOS | `src/macos.rs` | real: `NSOpenPanel` / `NSSavePanel` |
| Linux | `src/linux.rs` | real: `zenity`/`kdialog`, whichever is installed |
| Windows | `src/windows.rs` | real: `IFileOpenDialog` / `IFileSaveDialog` |
| Android / iOS | `src/lib.rs` | refuses: the answer arrives through an activity/delegate channel the platform layer does not expose |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub const SUPPORTED: bool`
- `pub struct Options { pub title: Option<String>, pub suggested_name: Option<String> }`
- `pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String>`
- `pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String>`
- `pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String>`

## Using it

```toml
[dependencies]
floom-plugin-file-dialog = { path = "crates/ecosystem/plugins/floom-plugin-file-dialog" }
```

```rust,ignore
let options = floom_plugin_file_dialog::Options::new();
match floom_plugin_file_dialog::pick_file(&options)? {
    Some(path) => println!("picked {}", path.display()),
    None => println!("cancelled"),
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_file_dialog.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-file-dialog
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-file-dialog/example/Cargo.toml
```
