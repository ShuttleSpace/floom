<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-tray

A tray icon with a menu, in the system tray or menu bar.

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| macOS | `src/macos.rs` | real: `NSStatusItem` |
| Linux | `src/linux.rs` | real: `libappindicator` |
| Windows | `src/windows.rs` | real: `Shell_NotifyIconW` |
| mobile / other | `src/unsupported.rs` | refuses: a phone has no tray |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub enum MenuItem { Item { id: u32, label: String, enabled: bool }, Separator }`
- `pub fn new(id: u32, label: impl Into<String>) -> MenuItem`
- `pub fn disabled(self) -> MenuItem`
- `pub fn separator() -> MenuItem`
- `pub enum TrayEvent { Menu(u32), Activate }`
- `pub struct Tray`
- `pub fn show(label: &str, items: &[MenuItem]) -> Result<Tray, String>`
- `pub fn events(&mut self) -> Vec<TrayEvent>`

## Using it

```toml
[dependencies]
floom-plugin-tray = { path = "crates/ecosystem/plugins/floom-plugin-tray" }
```

```rust,ignore
use floom_plugin_tray::{MenuItem, Tray};

let mut tray = Tray::show(
    "Floom",
    &[MenuItem::new(1, "Open"), MenuItem::separator(), MenuItem::new(2, "Quit")],
)?;
for event in tray.events() {
    println!("{event:?}");
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_tray.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-tray
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-tray/example/Cargo.toml
```
