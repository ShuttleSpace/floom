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
   shape is one small host library per capability that owns the platform
   objects and exposes a narrow ABI, with Rust calling into it — Swift through
   `@_cdecl` exporting the C ABI, Kotlin/Java through plain static methods called
   over JNI, OpenHarmony through the ArkTS bridge.
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

1. **One capability, one crate**, under
   `crates/ecosystem/plugins/floom-plugin-<capability>/`. The plugins are their own
   workspace: the SDK ships none of them, and adding one to an application is a
   dependency the application opts into.
2. **Reach the OS through a host library, not ad-hoc JNI.** Each plugin owns a
   Kotlin/Java (`.java`) or Swift host module and exposes a narrow ABI over the
   transport its platform gives: Swift through `@_cdecl` exporting the C ABI,
   Kotlin and Java through static methods called over JNI, OpenHarmony through
   the ArkTS bridge. Rust never touches platform objects directly. Crossings
   happen at coarse granularity (open a camera session, deliver one frame),
   never per widget property.
3. **Be host-inert.** On an unsupported target the API must still compile and
   return `Err`/`false`/`ZERO`. A plugin never breaks a desktop or web build.
4. **Be feature-gated**, so a host that does not need camera does not link it.
5. **Never be depended on by the core.** Dependencies point core → platform,
   plugin → platform. Platform → plugin is a boundary violation.

## The C ABI is the contract

The boundary between the two halves is **one header**, and the header says
which side implements what:

```c
/* native/common/floom_<capability>.h */
const char *floom_<capability>_version(void);
int floom_<capability>_enable(void);
void floom_<capability>_disable(void);
int floom_<capability>_capability(void);
int floom_<capability>_abi_version(void);
```

**The native half implements these; the Rust half calls them.** That direction
is what makes one header portable: a C file implements them directly, Swift
implements them with `@_cdecl`, and on Android the Java half is a plain class
with static methods that the plugin's Rust edge (`src/android.rs`) calls over
JNI — JNI *is* the FFI there, so the capability is implemented in Java and the
call crosses into it without any C shim in between. The Rust side takes the
same direction on every platform: it calls into the half that implements the
capability.

The opposite direction — Rust exports, native calls — is what Kotlin's
`external fun` gives you, and it is a *different* contract: it cannot express
a Rust-owned capability, and it makes the Rust crate the thing that has to
know about each platform. One header, one direction, no second contract to
keep in sync.

The ABI version is a **function** as well as a macro, because the Rust half has
to be able to ask a plugin it did not compile what it was built against, and a
compile-time macro cannot answer that.

### The layout

```
crates/ecosystem/plugins/floom-plugin-<capability>/
├── Cargo.toml
├── build.rs                 compiles the C halves, emits the cfg
├── src/lib.rs               the `extern "C"` declarations and the Rust API
├── src/android.rs           the Android edge: JNI calls into the Java half
├── android/                 the Android half (Gradle library module + Java)
└── native/
    ├── common/              the C ABI header — the single source of truth
    ├── desktop/             the desktop half (C)
    ├── ios/                 the iOS half (Swift, exported with `@_cdecl`)
    ├── ohos/                the OpenHarmony half (ArkTS + a C shim)
    ├── web/                 the web half (JavaScript)
    └── tui/                 nothing: the terminal has no native surface
```

`build.rs` compiles the halves that are C (the desktop half for the current
target; the OpenHarmony helper on Harmony) and emits
`cargo:rustc-cfg=floom_plugin_native` **only** when it found and compiled one
for the current target. The Rust half's `extern` block is behind that flag, so
it never names a symbol the linker cannot find.

That flag is not a `target_os` check on purpose. "Which OS am I on" is not the
same question as "does this plugin have a native half for this OS" — a plugin
generated for Android only, compiled on Linux, has no Linux half, and a
`target_os` check would declare the extern block anyway and fail at link time
in a crate that should have compiled. Android is the same exception in the
other direction: its half is Java, compiled into the host app's dex by the
Gradle library module, so `build.rs` lets the crate through without compiling
anything and emits no `floom_plugin_native` cfg — the halves are split by
`#[cfg(target_os = "android")]` instead.

### How each platform reaches its native half

The contract is the same everywhere. Only the transport differs, and the
difference lives in the plugin's own crate — never in the contract.

| Platform | Native half | Transport |
|---|---|---|
| desktop | C | compiled by `build.rs` and linked in |
| Android | Java | the Rust edge (`src/android.rs`) calls the library's `{Name}Plugin` static methods over JNI; no C shim |
| iOS | Swift | `@_cdecl`, linked as a framework |
| web | JavaScript | imported by the wasm module (`wasm-bindgen`) |
| OpenHarmony | ArkTS | a C shim that reaches ArkTS through a Rust-exported helper |
| TUI | — | none: the terminal has no OS capability surface |

Two of these need a word.

**The web cannot link.** There is no native library to link into a wasm
module, so the module *imports* the JavaScript instead, and `wasm-bindgen`
resolves the import to `native/web/js/floom_<name>.js`. The names are the
ABI's names; only the direction differs.

**OpenHarmony's environment lives in Rust.** A HAP links *one* N-API module
and it belongs to the host, so a plugin cannot export its own `#[napi]`
functions — it would be a second module. The plugin therefore owns the
*capability* and the host owns the *transport*: the host hands the plugin an
`ArkTsBridge` at startup, and the plugin's C shim reaches ArkTS through the
one helper Rust exports (`floom_<name>_arkts_call`). The C ABI is still
implemented by the native half; the helper is a platform-internal transport,
not part of the contract.

**The terminal has nothing.** A character grid is not a platform with OS
services: no camera, no sensor, no enclave. A terminal build compiles the
plugin and `enable()` reports that there is no native half for the target,
rather than pretending. A capability that *does* have a terminal meaning
belongs in the desktop C half, because a terminal app is an ordinary program
on the host operating system.

### A plugin that contributes a native view

Some capabilities are not a query but a **view**: a camera preview, a web
view, a video. Those are *native participants* — views the framework does not
paint — and they have a second contract, the **provider ABI**
(`crates/floom-native-ffi/include/floom_native_provider.h`).

The two contracts answer different questions:

| | The plugin's own ABI | The provider ABI |
|---|---|---|
| asks | is the capability available? | how is the native view created, placed and torn down? |
| shape | four functions, the plugin is a singleton | a versioned struct of function pointers, the provider is an instance per window |
| lives in | `native/common/floom_<name>.h` | `native/common/floom_native_provider.h` |

The provider's shape is a struct rather than a set of named symbols because a
provider carries a `self` pointer: there is one per window, not one per
process. The ABI version lives *in* the struct, so the Rust side can ask a
provider it did not compile what it was built against and refuse it.

Everything the provider receives crosses in **whole device pixels**, never
logical ones and never a float. The logical-to-device conversion happens
once, on the Rust side, in `to_device` — and it happens once because a
platform cannot place a view at a fractional pixel, and letting each provider
invent its own rounding is how a half-pixel seam appears on one platform and
not another.

`floom create --plugin` writes the provider header next to the plugin's own,
so a Swift or Java plugin has the contract to implement without having to
find it.

### A worked example

`crates/ecosystem/plugins/floom-plugin-nativeview` is a plugin whose capability
is a **native view**, and its native half is **Objective-C** — the language
AppKit is written in, because a plugin that contributes a view has to reach the
framework it contributes to.

```
crates/ecosystem/plugins/floom-plugin-nativeview/
├── build.rs                      compiles native/macos/floom_nativeview.m
├── native/
│   ├── common/floom_nativeview.h         the plugin's own four-function ABI
│   └── macos/floom_nativeview.m          the Objective-C half
├── src/lib.rs                    the Rust half
├── examples/native_view.rs       the whole chain, on the main thread
└── tests/native_view.rs          the boundary, headless
```

The chain, end to end:

1. the Objective-C half's constructor hands its provider vtable to the Rust
   half — the native code is the only party that knows when it is ready;
2. the app asks the plugin for a provider and hands it the host's window;
3. the framework drives the provider through the ordinary Rust
   `NativeViewProvider` trait, so the controller's lifecycle invariants apply
   to a Swift or Java provider exactly as they apply to a Rust one;
4. the Objective-C half makes a real `NSView`, puts it in the window's
   content view, and places it.

The example prints what happened:

```text
window: <NSWindow: 0x7f9796c04100>
capabilities: modes=[Overlay] snapshot=false focusable=false
bound in mode Overlay
backing scale: 2
placed at 10,15 100x50 (points)
```

That last line is the whole coordinate story in one: the framework snapped to
whole **device** pixels, and the native half divided by the window's backing
scale to get AppKit's **points** — once each, on the side that owns that
coordinate space.

The window half is an **example** rather than a test for one reason: AppKit
wants the main thread, and a test harness does not give one. The
framework-side half — the C ABI, the provider, the lifecycle — is covered by
the test suite on every platform.

### How this is verified

`floom create --plugin` scaffolds the whole thing, and the generated crate's
own tests assert that the version it reports comes from the native half — a
check that fails if the boundary is stubbed. The CLI's tests assert that the
header, the desktop half, the Java half and the Swift half all name the same
symbols, because that is the failure the single-header design exists to
prevent.

For the provider ABI, the proof is stronger, because the failure it prevents
is invisible: the header is compiled by the platform's own C compiler, and
the program it compiles reports every struct's size and every field's offset
— which are compared against the Rust mirror's, field by field. A
hand-written ABI drifts, and the symptom is not a compile error but a
provider writing into a struct the framework reads at the wrong offsets.


## Existing capabilities

The mobile capabilities were **thirteen packages inside the platform adapter**,
and they are now **nine plugin crates**, with the adapter keeping only what is
the platform's own business.

| | before | after |
|---|---|---|
| `floom-platform-mobile` | 13 capability packages, plus the whole iOS dependency tree | 4 packages: `clipboard` and `keyboard` (the two of `floom-platform`'s three traits that mobile implements) and `safe_area` / `system_chrome` (facts about a *window*, not capabilities a plugin provides) |
| the capabilities | linked into every mobile app that touched the platform adapter | one crate each, opt-in by dependency: `floom-plugin-battery`, `-connectivity`, `-device-info`, `-local-auth`, `-location`, `-notifications`, `-sensors`, `-share`, `-vibration` |

That is the whole of "the SDK stays small": a capability in the adapter is
compiled into every app on that platform whether it wants the capability or
not; a capability in its own crate is linked only by the apps that ask for it.
Moving them also removed the adapter's entire iOS dependency tree — the
capability that needed `objc2` brought its own framework bindings with it.

The rule is enforced rather than promised: the architecture gate refuses a
dependency from a `floom-platform-*` crate to a `floom-plugin-*` crate. The
platform reaching *up* to a plugin is the edge that would put the capability
back in.

`camera` is intentionally **not** there yet: it is a streaming media surface
rather than a single-shot query, so it is the first real test of rule 2 — it
wants a Java host library (`CameraX`, which ships as a Java artifact the
host's d8 step compiles) and a Swift one (`AVFoundation`), with frames handed
over as an external texture rather than copied per frame. It would be a
plugin, and it would be the one that contributes a **native view**, so it is
the first thing to need the provider ABI.

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