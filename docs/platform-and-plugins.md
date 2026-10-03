# Platform layer and plugins

Decides what belongs in `floom-platform*` (the platform layer) and what must be
a plugin. The short version:

> The platform layer provides **only what a window cannot be built without**.
> Everything else — camera, video, sensors, biometrics, … — is a plugin that
> reaches the OS through a native host library over FFI.

## Why

Two costs drove this split.

1. **Native code volume.** A camera or video API cannot be implemented in Rust
   against the NDK without reimplementing Camera2/CameraX or AVFoundation. Doing
   it per-call in JNI is worse: the bridge is chatty, every call crosses the
   JNI boundary, and object lifetimes must be hand-managed. The maintainable
   shape is one small Kotlin/Java (or Swift) host library per capability that
   owns the platform objects and exposes a narrow C ABI, with Rust calling into
   it.
2. **Backend count.** Every capability added to the core multiplies across nine
   platforms. `floom-platform-android`, `-ios`, `-harmony`, `-web`, `-desktop`,
   `-mobile`, `-tui`, `-embedded` would each grow a camera arm. As plugins, a
   capability ships for the platforms it supports and is inert elsewhere.

## The core surface

`crates/floom-platform` defines the contract, and it is deliberately three
traits (`crates/floom-platform/src/lib.rs`):

| Trait | Why it is core |
|---|---|
| `Platform` | surface/window creation, event pump, lifecycle — without it there is no app |
| `Clipboard` | core text editing; no OS clipboard means paste is impossible |
| `Ime` | soft keyboard; required for any text input on mobile |

Everything else — window chrome, dialogs, file pickers, sensors, haptics — is
either a plugin or already lives behind an existing capability package.

The `floom-platform-*` crates are **adapters**: they translate one OS into those
traits. They must not accumulate capabilities of their own.

## Plugin rules

A capability is a plugin when it satisfies any of: it needs a native host
library; it is only meaningful on some platforms; or it is large enough that
its API would otherwise destabilise the core.

1. **One capability, one crate**, under `crates/floom-plugin-<capability>/`.
2. **Reach the OS through a host library, not ad-hoc JNI.** Each plugin owns a
   Kotlin/Java (`.kt`/`.java`) or Swift host module and exposes a narrow
   `extern "C"` ABI; Rust never touches platform objects directly. Crossings
   happen at coarse granularity (open a camera session, deliver one frame),
   never per widget property.
3. **Be host-inert.** On an unsupported target the API must still compile and
   return `Err`/`false`/`ZERO`. A plugin never breaks a desktop or web build.
4. **Be feature-gated**, so a host that does not need camera does not link it.
5. **Never be depended on by the core.** Dependencies point core → platform,
   plugin → platform. Platform → plugin is a boundary violation.

## Existing capabilities

`crates/floom-platform-mobile/src/packages/` already follows this shape with the
`mod.rs` + `ios.rs` + `android.rs` dispatch convention (host-inert stubs where a
backend is not wired yet):

`vibration`, `keyboard`, `safe_area`, `system_chrome`, `clipboard`,
`device_info`, `battery`, `connectivity`, `location`, `sensors`, `local_auth`,
`notifications`, `share`.

`camera` is intentionally **not** there yet: it is a streaming media surface
rather than a single-shot query, so it is the first real test of rule 2 — it
wants a Kotlin host library (`CameraX`) and a Swift one (`AVFoundation`), with
frames handed over as an external texture rather than copied per frame.

## Relationship to `floom/src/platform`

The v1 monolith carries its own bridges in `crates/floom/src/platform/`
(clipboard, file pickers, a11y, JNI/ObjC/OHOS shims). That duplicates the
`floom-platform-*` adapters. When the v1 monolith is retired these are deleted;
nothing should be migrated from them into a plugin — the `floom-platform-*`
adapters already cover the same ground.

## Enforcement

The layering half (core → platform only, never platform → plugin) belongs in
`scripts/check-architecture-deps.sh`, which already enforces the layer table for
every crate under `crates/`. Adding plugins as a layer below the platform
adapters turns an accidental core → plugin edge into a build failure rather than
a review comment.