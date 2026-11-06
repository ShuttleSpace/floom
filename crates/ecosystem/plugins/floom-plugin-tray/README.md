<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-tray

A tray icon with a menu: the app's presence in the system tray or menu bar.

This is `tray_manager`'s question — an app that keeps running with its
windows closed still needs a way to be found — and the batch recorded in
`docs/plugin-catalog.md` §10.

[`Tray::show`] puts an icon in the tray and a menu behind it;
[`Tray::events`] drains what the user did; dropping the [`Tray`] removes the
icon. The icon carries a text label rather than an image: a status item's
title is the one thing every arm can render without shipping assets, and
guessing at a per-platform image format is how an API becomes three APIs.

# Why this is a plugin

A tray needs no host handle to exist. macOS creates its own `NSStatusItem`,
Windows its own message-only window, Linux its own D-Bus object — none of
them asks the app for a window, a surface or a frame. That is the whole
difference from window control, which had to go into the platform layer
because the window belongs to the host. So this crate depends on nothing
above L4 and an app that does not name it links nothing.

# What each arm needs from the app

The tray does not run a loop; the app already has one, and this crate hands
its events to it:

- **macOS** — the status item and its menu are created on the main thread,
and `NSMenu` runs its own tracking loop while the menu is open. Call
[`Tray::show`] from the main thread; call [`Tray::events`] from whatever
runs after input.
- **Windows** — `Shell_NotifyIconW` posts its callback to a message-only
window this crate creates. That window's messages are dispatched by
whatever pumps the thread's message queue (a winit event loop does), so
[`Tray::events`] only drains what the pump already delivered.
- **Linux** — `StatusNotifierItem` over D-Bus. zbus runs its own thread, so
events arrive without the app pumping anything; [`Tray::events`] drains
the queue it fills.

# What is deliberately not here

No images, no submenus, no dynamic menu or label updates, no per-item
checkboxes, and no left/right-click distinction on macOS — a status item
with a menu opens the menu, because that is what a menu bar icon does. Each
of these is a real feature with three shapes; adding one is a decision, not
a flag.

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

- `MenuItem` — One row of the tray menu.
- `Tray` — A tray icon and its menu.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-tray = { path = "crates/ecosystem/plugins/floom-plugin-tray" }
```

A minimal use:

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

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub enum MenuItem {`
- `pub enum TrayEvent {`
- `pub struct Tray {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_tray.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-tray
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-tray/example/Cargo.toml
```
