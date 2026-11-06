<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-open-url

Open a URL with the application the OS routes its scheme to.

This is `url_launcher`'s question — "show this `https://` in a browser,
this `mailto:` in a mail app, this `tel:` in a dialer" — and it is the
catalog's A2 (`docs/plugin-catalog.md`): the implementation half of the
handoff doc's C4 decision, which deleted the `OPEN_URL` capability bit
because a bit with no operation behind it is a promise nothing keeps.

A **capability plugin**: one capability, one crate, opt-in by dependency.

# What `Ok` means — and what it does not

`Ok(())` claims one thing: **the OS accepted the request.** It does not
claim a window opened, a browser launched, or the user saw anything. Every
arm here hands a URL to the platform's routing and returns what the routing
said — `startActivity` returning, `openURL:`'s `BOOL`, `ShellExecuteW`'s
`HINSTANCE`, `xdg-open`'s exit status. Whether the handler then shows
anything is outside this crate's knowledge, and pretending otherwise (an
`Ok` that waited for a window) is not possible over these APIs at all.

This is the "true launch" category from the catalog's verification matrix:
the action is the answer, and the caller who needs more must watch for
focus or use a deeper integration.

# Every arm is a real read

Like `floom-plugin-paths`, there is no host-inert arm on the desktops:
macOS routes through `NSWorkspace`, Linux through `xdg-open`, Windows
through `ShellExecuteW`, and those are the platform's own mechanisms — a
host is the source of truth here, not a stand-in for a phone. Only targets
with no such mechanism (a terminal UI, say) get the refusal `Err`.

# What this crate deliberately does not do

`url_launcher` has a launch *mode* — external application, in-app web
view, platform default. The in-app mode is a webview, which is a different
catalog entry (B1, `floom-plugin-webview`, provider ABI and all); this
crate is the external mode only, and says so rather than shipping an
in-app fallback that renders nothing. It also does not answer
`canLaunchUrl` — "does some app handle this scheme" is a query with its own
platform story (intent resolution on Android, Launch Services on macOS),
and half of it behind the same function would blur what `Ok` means.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/ios.rs` |
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

- `open_url` — Open `url` with the application the OS routes its scheme to.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-open-url = { path = "crates/ecosystem/plugins/floom-plugin-open-url" }
```

A minimal use:

```rust,ignore
floom_plugin_open_url::open_url("https://example.com")?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn open_url(url: &str) -> Result<(), String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_open-url.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-open-url
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-open-url/example/Cargo.toml
```
