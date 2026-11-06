<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-package-info

The running application's own identity.

`AppInfo` answers "which app am I, and which version of it is installed" —
the display name, the bundle identifier / application id, the version and
the build number. An app uses it for an about screen, for a user-agent
string, or to gate a migration on "the data was written by 1.3".

A **capability plugin**: one capability, one crate, opt-in by dependency.

# Why every field is an `Option`

The platform does not always have an answer, and the caller has to be able
to tell that apart from an empty one. macOS reads the bundle it is running
inside; a bare executable is not in a bundle and has no `Info.plist`, so
every field is `None` there. Linux has no application-package concept at
all, and the Windows version resource is a deliberate follow-up rather than
something this crate pretends to read.

A `String::new()` default would be the mistake the battery plugin documents:
an empty string and a real-but-empty value are indistinguishable, so a
caller formatting `"app {}"` would print `"app "` and believe it.

# Why the host arm is not a stub on every desktop

On macOS the *host* is the real source: `NSBundle.mainBundle` is exactly
what `package_info_plus` reads on that platform. So this plugin's Apple arm
is compiled by an ordinary `cargo test` on a Mac, unlike the nine
mobile-only plugins whose native arms no host build ever reaches. That is
why the arm lives in `apple.rs` rather than being split into `ios.rs` and
`macos.rs`: the code is the same, and keeping it in one file keeps it in the
one place a host test covers.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
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

_See the API section below._


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-package-info = { path = "crates/ecosystem/plugins/floom-plugin-package-info" }
```

A minimal use:

```rust,ignore
let info = floom_plugin_package_info::app_info();
println!("{:?} {:?}", info.name, info.version);
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct AppInfo {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_package-info.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-package-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-package-info/example/Cargo.toml
```
