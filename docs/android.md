# Android Backend (Phase 1 — Skeleton)

> **Status:** Skeleton + **GPU surface wired + clipboard JNI bridge + context-menu trigger + external-URL opening + SAF file dialogs**.
> `floem` **compiles for `aarch64-linux-android`**, the Android entry point +
> embedder capability preset are in place, the GPU renderer creates its surface
> from the Android native window (`ANativeWindow*`) on both initial creation and
> the pause/resume/rotation lifecycle, the **system clipboard is reachable via
> JNI**, the **in-UI context menu is triggered** from a secondary pointer
> button, **external URLs open via an `ACTION_VIEW` intent**, and **file
> dialogs use the Storage Access Framework**. On-device APK render is the only
> unverified step (and file-dialog *results* additionally need the optional
> `FloemNativeActivity` `onActivityResult` hook — see below).
>
> **Drag & Drop:** the OS-level *file* drag-and-drop capability (`drag_drop`) is
> `false` on Android **by design** — winit-android emits no drag events and Android
> has no in-window OS file DnD. Floem's **in-app** drag system (`Event::Drag` /
> `DragTracker`) is fully platform-agnostic and **already works on Android** through
> the touch→`PointerEvent` pipeline with no backend code.
> `FloemNativeActivity` `onActivityResult` hook — see below).

This document tracks the Android backend for Floem, built on top of the
experimental [`embedder`](../src/embedder) protocol from
[`docs/superpowers/plans/2026-08-03-embedder-api.md`](../docs/superpowers/plans/2026-08-03-embedder-api.md).

## What already works

- `cargo check -p floem --target aarch64-linux-android` passes (zero errors).
- The lapce **winit fork** (`floem-winit`, rev `133268de`) already ships an
  Android backend (`floem-winit-android`) that translates Android input into
  standard winit `WindowEvent`s. So keyboard/pointer reuse Floem's existing
  winit path — no custom input translation is needed.
- `floem::Application::new_android(app)` builds the event loop with
  `EventLoopBuilderExtAndroid::with_android_app(app)` and reuses the exact same
  `.window(...).run()` flow as desktop (`src/app/mod.rs`).
- **GPU surface is created from the Android native window automatically.** The
  lapce winit fork's Android `Window` implements `raw_window_handle::HasWindowHandle`
  by returning the `ndk` `NativeWindow` handle (`AndroidNdkWindowHandle`). Floem's
  existing `GpuResources::request` → `instance.create_surface(window)` path (used
  by desktop too) therefore builds a Vulkan surface straight from `ANativeWindow*`
  — **no hand-written JNI/NDK surface bridge is needed**. Android defaults to the
  Vulkan backend (`AppConfig::wgpu_backends = Some(Backends::VULKAN)`).
- **Surface lifecycle (pause/resume/rotation) is handled.** Android destroys and
  recreates the native window on every pause/resume and rotation. Floem now
  marks each window's surface `lost` in `destroy_surfaces` and **rebuilds** the
  wgpu surface against the fresh native window in `can_create_surfaces`
  (`WindowHandle::recreate_gpu_surface`, `src/window/handle.rs` /
  `src/app/handle.rs`). Painting is skipped while the surface is lost, so the
  stale surface is never presented.
- `src/embedder/android.rs` provides the embedder adapter-trait skeleton:
  - `android_capabilities()` — the Android [`BackendCapabilities`] preset.
  - `AndroidServices` — `PlatformServices` impl.
  - `AndroidEmbedderWindow` / `AndroidSurfaceHandle` — window + surface
    adapters.
- **System clipboard is reachable via JNI.** `Application::new_android` extracts
  the `JavaVM*` and the `Activity` JNI global reference from the `AndroidApp`
  (`vm_as_ptr()` / `activity_as_ptr()`) and hands them to
  `Clipboard::init_android`. `src/platform/clipboard.rs` then bridges
  `Clipboard::get_contents` / `set_contents` to Android's
  `android.content.ClipboardManager` through the `jni` crate
  (`getPrimaryClip` / `setPrimaryClip` with `ClipData.newPlainText`). No
  desktop clipboard code is touched — the legacy `copypasta` path stays
  untouched on non-Android.
- **Context menu is triggered on Android.** `WindowHandle::show_context_menu`
  now has an Android branch (alongside Linux/FreeBSD/wasm32) that sets the
  in-UI `context_menu` signal; `action::show_context_menu` /
  `UpdateMessage::ShowContextMenu` are no longer gated out on Android, and the
  pointer `handle_menu_events` trigger (secondary button) runs on Android too.
  The existing `context_menu_view` renderer (already compiled for Android)
  draws the menu; clicks dispatch through `AppUpdateEvent::MenuAction`.
- **Touch long-press opens the context menu.** Touch devices have no secondary
  button, so `src/event/long_press.rs` (`LongPressState`, owned by
  `WindowState`) watches touch primary presses in `dispatch.rs` and, after a
  500 ms stationary hold (`action::exec_after` timer, cancelled on move/up/cancel
  beyond a 10 px threshold), opens the context menu of the view under the press
  via `action::show_context_menu_internal(.., had_pointer_down = true)`. The
  Android `event()` dismiss logic was extended (it previously only covered
  Linux/FreeBSD/wasm32) so the finger-lift that ends the long-press does *not*
  dismiss the menu (it flips `had_pointer_down` to `false` to "arm" it); a
  subsequent tap outside the menu, or an item click, closes it. Mouse/pen
  secondary presses still use the original `Secondary`-button path.
- **External URLs open via a JNI `ACTION_VIEW` intent.** `floem::platform::open_url`
  is implemented for Android in `src/platform/android_external.rs`: it builds an
  `Intent.ACTION_VIEW` with a `Uri` parsed from the URL and calls
  `Activity.startActivity`. `android_capabilities().open_external` is now `true`.
  (On desktop, core does not open URLs itself — apps use the `open` crate
  directly — so `open_external` stays `false` there.)
- **File dialogs use the Storage Access Framework (SAF).** `src/platform/android_file.rs`
  provides `open_file` / `save_as` that launch `ACTION_OPEN_DOCUMENT`,
  `ACTION_OPEN_DOCUMENT_TREE`, or `ACTION_CREATE_DOCUMENT` via
  `Activity.startActivityForResult`, and a public
  `floem::platform::dispatch_activity_result` entry that the app's
  `onActivityResult` forwards to. `android_capabilities().file_dialog` is now
  `true`. Selected `content://` URIs are returned as `FileInfo.path` entries
  (see the note in that module). The picker result is delivered through the
  optional `FloemNativeActivity` Java subclass — `examples/android/` ships
  `FloemNativeActivity.java` and a native bridge (`floemOnActivityResult`) for
  exactly this; without it the picker opens but selections are not delivered.
- Example crate at `examples/android/` (`android_main` entry + `AndroidManifest.xml`
  + `FloemNativeActivity.java` + `cargo ndk` build notes).

### Drag & Drop — two distinct features

Floem has **two unrelated** drag-and-drop mechanisms; only one is affected by the
Android `drag_drop` capability flag.

**1. OS-level *file* drag-and-drop — unsupported on Android (`drag_drop: false`).**
This is the `EmbedderEvent::DragDrop` / `FileDragDrop` listener path: dragging a
file from the OS file manager or another app *into* a window. It depends on winit
emitting `WindowEvent::DragDropped`/`DragEntered`/`DragMoved`/`DragLeft`. The
winit-android backend **does not emit these events** (it only implements
`drag_window` for moving the OS window), and Android's per-app storage sandbox has
no notion of dropping an arbitrary file into a window. Flipping `drag_drop` to
`true` would therefore advertise a capability nothing delivers — it must stay
`false`. The Android idiom for ingesting a file from another app is an
`Intent` (`ACTION_SEND` / `ACTION_VIEW` with a `content://` URI) or the Storage
Access Framework picker, both already handled via the `open_external` /
`file_dialog` capabilities (not `DragDropEvent`).

**2. In-app drag-and-drop — fully works on Android, no backend code needed.**
This is Floem's `Event::Drag` / `DragTracker` system (`src/event/drag_state.rs`),
used for reordering list items, moving views, custom drag interactions, etc. It is
driven entirely by `PointerEvent`s (Down/Move/Up) plus Floem's **internal** pointer
capture (`WindowState::set_pointer_capture`, tracked in
`pending_pointer_capture_target`) — it never touches winit's `set_cursor_grab`.
Android feeds touch as `PointerEvent`s through the same `floem_ui_events_winit` reducer
the desktop uses, each with a stable `pointer_id`, so the whole threshold →
`DragStart` → `DragMove` → `Drop`/`Cancel` lifecycle runs unchanged. To make a view
draggable on Android you follow the exact same recipe as desktop:

```rust
fn handle(event: Event, cx: &mut EventCx) -> EventPropagation {
    match event {
        Event::Pointer(PointerEvent::Down(pe)) => {
            if let Some(id) = pe.pointer.pointer_id {
                cx.request_pointer_capture(id);          // internal, backend-agnostic
            }
        }
        Event::PointerCapture(PointerCaptureEvent::Gained(token)) => {
            cx.start_drag(token, DragConfig::default(), true);
        }
        Event::Drag(DragEvent::Source(DragSourceEvent::Start(_))) => { /* … */ }
        Event::Drag(DragEvent::Target(DragTargetEvent::Drop(e))) => {
            cx.prevent_default();
            /* … */
        }
        _ => {}
    }
    EventPropagation::Continue
}
```

No Android-specific code is required; the existing touch→pointer pipeline covers it.

### Receiving shared files (Intent)

Floem lets an Android app accept files shared from another app (the share sheet /
"open with") via the `receive_files` capability. The app registers a handler:

```rust
use floem::platform::{set_shared_files_handler, SharedFile};

set_shared_files_handler(Box::new(|files: Vec<SharedFile>| {
    for f in files {
        // f.uri is a `content://` URI — open it via ContentResolver.
        println!("shared: {} ({:?})", f.uri, f.mime_type);
    }
}));
```

**Why a handler and not a `DragDrop` event?** Android has no OS-level in-window
file DnD (see the *Drag & Drop* section). Sharing is delivered as an Android
`Intent` carrying `content://` URIs, which is a different mechanism from the
desktop `FileDragDrop` / `DragDropEvent` path (those carry `PathBuf`s). So this
uses a dedicated, Android-shaped API rather than pretending to be file DnD.

**Two delivery moments:**

1. **Launch intent** (the app was *started* by a share). Floem reads
   `Activity.getIntent()` in `Application::new_android` (via
   `src/platform/android_intent.rs::read_launch_intent`) and delivers immediately.
   **This works with the default `android.app.NativeActivity`** — no Java subclass
   required. If the handler isn't registered yet, the parsed files are queued and
   flushed the moment `set_shared_files_handler` is called.
2. **Runtime intent** (the app was *already running* and the user shared again).
   The app's `FloemNativeActivity` must override `onNewIntent` and forward the
   `Intent` to `floem::platform::dispatch_android_intent` (same JNI-bridge pattern
   as the file-dialog result hook). See `examples/android/FloemNativeActivity.java`.

**Intent shapes handled:** `ACTION_SEND` (single file in `EXTRA_STREAM`),
`ACTION_SEND_MULTIPLE` (`EXTRA_STREAM` as an `ArrayList`), and `ACTION_VIEW`
(`getData()`). Each becomes a `SharedFile { uri, mime_type, display_name }`. The
`uri` is the `content://` string; `display_name` is the Uri's last path segment
(best-effort label). Open the Uri through `ContentResolver` — it is not a plain
filesystem path.

**Manifest:** to make the app appear in the share sheet, the activity needs
`<intent-filter>`s for `ACTION_SEND` / `ACTION_SEND_MULTIPLE` (category `DEFAULT`)
and `ACTION_VIEW` (categories `DEFAULT` + `BROWSABLE`). Scope the `<data
android:mimeType>` to the file types you actually handle instead of `*/*`. See
`examples/android/AndroidManifest.xml`.

## Architecture

```
android_main(app: AndroidApp)            // winit / native-activity glue
        │
        ▼
Application::new_android(app, ...)      // builds EventLoop with with_android_app
        │  .window(view_fn, None).run()
        ▼
winit-android event loop                // translates input → winit WindowEvents
        │
        ▼
Floem ApplicationHandler                // SAME path as desktop; no fork needed
        │
        ▼
Window creation → GPU renderer          // surface built from ANativeWindow* via winit rwh
        │                                   (Vulkan backend; recreated on pause/resume/rotate)
        ▼
GpuResources::request → instance.create_surface(window)
        │                                   window.handle() → ndk NativeWindow (AndroidNdkWindowHandle)
        ▼
wgpu Vulkan VkSurfaceKHR ← ANativeWindow*   ✅ wired
```

The only Android-specific seams are:

1. **Event loop construction** — must associate the `AndroidApp` (done).
2. **Renderer surface** — created from the Android `ANativeWindow*` via the
   standard winit `HasWindowHandle` → `GpuResources::request` path
   (`src/renderer/gpu_resources.rs`). **This is now wired** — both on initial
   window creation (inside `can_create_surfaces`) and on the
   pause/resume/rotation lifecycle (surface rebuilt in `can_create_surfaces`
   after `destroy_surfaces` marks it lost).

## Android capability preset

`android_capabilities()` starts from "everything off" and enables what Android
supports, differing from `BackendCapabilities::winit_default()` in:

| Capability        | Desktop | Android | Notes                                        |
| ----------------- | ------- | ------- | -------------------------------------------- |
| `window_title`    | ✓       | ✗       | No Android title bar.                        |
| `drag_drop`       | ✓       | ✗       | **OS-level *file* drag-and-drop only.** The winit-android backend emits no `DragDropped`/`DragEntered`/`DragMoved`/`DragLeft` events, and Android's app sandbox has no in-window OS file DnD. Floem's *in-app* drag system (`Event::Drag` / `DragTracker`) is independent of this capability and **already works on Android** (see below). Receiving a shared file from another app is done via `Intent`/SAF, not `DragDropEvent`. |
| `file_dialog`     | ✓       | ✓       | SAF via `startActivityForResult` (needs the optional `FloemNativeActivity` `onActivityResult` hook for results). |
| `receive_files`   | ✗       | ✓       | Android `ACTION_SEND` / `ACTION_VIEW` intents delivered as `content://` URIs via `floem::platform::set_shared_files_handler`. Launch intent works with the default `NativeActivity`; runtime `onNewIntent` needs `FloemNativeActivity`. |
| `clipboard`       | ✓       | ✓       | JNI bridge wired (`Clipboard::init_android`).|
| `context_menu`    | ✓       | ✓       | In-UI renderer (no muda native menu).        |
| `native_menu`     | —       | ✗       | muda does not target Android.                |
| `ime`             | ✓       | ✓       | Soft keyboard.                               |
| `virtual_keyboard`| ✗       | ✓       |                                              |
| `safe_area`       | ✗       | ✓       | Notch / gesture-bar insets.                  |
| `open_external`   | ✗       | ✓       | Android: JNI `ACTION_VIEW` intent; desktop core leaves URL opening to the `open` crate. |
| `gpu_surface`     | ✓       | ✓       | Requires the renderer surface integration.   |

## Dependency isolation (hard constraint)

Per the plan, mobile dependencies must **not** enter default features. This is
enforced with target-gated Cargo sections:

```toml
# Desktop-only deps — excluded from Android builds.
[target.'cfg(not(target_os = "android"))'.dependencies]
rfd = { version = "0.17.1", default-features = true }
copypasta = { version = "0.10", default-features = false, features = ["wayland", "x11"] }

# Android-only deps — excluded from desktop builds.
[target.'cfg(target_os = "android")'.dependencies]
winit = { workspace = true, features = ["android-native-activity"] }
ndk = "0.9"
android_logger = "0.14"
```

Source-level isolation uses `#[cfg(target_os = "android")]` extensively:

- `rfd`/`copypasta`/`muda` native-menu machinery is gated out; Android reuses
  the **wasm stub** menu types (`src/platform/menu_types.rs`) and the in-UI
  context-menu renderer.
- `src/platform/clipboard.rs` bridges the system clipboard via JNI on Android
  (`Clipboard::init_android`); the legacy `copypasta` path stays desktop-only.
- `file_action` (open/save dialogs) and `window_menu` are gated out.

## Build toolchain

```bash
# 1. Rust Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi \
  i686-linux-android x86_64-linux-android

# 2. NDK — point at your SDK/NDK
export ANDROID_HOME=$HOME/Library/Android/sdk
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/<version>

# 3. cargo-ndk
cargo install cargo-ndk

# 4. Type-check the library for Android (fast, no APK needed)
CARGO_BUILD_JOBS=4 cargo check -p floem --target aarch64-linux-android

# 5. Build the example shared lib AND package it into an APK
cd examples/android
scripts/build-apk.sh -t arm64-v8a          # -> target/android/*.apk
```

> **Two things step 5 used to get wrong.** (a) `cargo ndk` never produces an
> APK — it only cross-compiles and copies `*.so` into a `jniLibs/<abi>/`
> layout; APK creation was `cargo-apk`'s job (now unmaintained). (b) The
> example crate was **bin-only**, so no `.so` was produced at all:
> `native-activity` loads the library named by `android.app.lib_name`, so the
> Android entry point must be a `cdylib`. `examples/android/Cargo.toml` now
> declares `[lib] crate-type = ["cdylib"]` and `android_main` + the JNI
> bridges live in `src/lib.rs`.
>
> `scripts/build-apk.sh` does the packaging with SDK build-tools only
> (`aapt2 link` → bundle `lib/` uncompressed → `zipalign -P 16` →
> `apksigner` with an auto-created debug keystore). No Gradle/AGP needed.
> `cargo-apk`/`cargo-ndk` are not required for the type-check above.

## Known gaps / remaining work

1. **Renderer `ANativeWindow` surface — now wired (code-complete, unverified on device).**
   The surface is created from `ANativeWindow*` via the standard winit
   `HasWindowHandle` → `GpuResources::request` path, on both initial creation and
   the pause/resume/rotation lifecycle. What remains is **on-device verification**
   (build the APK with `cargo ndk` and confirm a frame paints, then survives a
   rotation). Edge case: if the app is backgrounded *before* the GPU device is
   acquired, the surface is marked lost and not yet recreated when the device
   later arrives — acceptable for the skeleton, tighten later.
2. **Clipboard JNI bridge — done (code-complete, unverified on device).**
   `Clipboard::get_contents` / `set_contents` now route through
   `android.content.ClipboardManager` via the `jni` crate. `Clipboard::init_android`
   is called from `Application::new_android` with the `JavaVM*` + `Activity` JNI
   handles. The legacy `copypasta` desktop path is untouched. Remaining: on-device
   verification of the copy/paste round-trip.
3. **Context-menu trigger — done (code-complete).**
   `WindowHandle::show_context_menu` gained an Android branch that drives the
   in-UI `context_menu` signal; `action::show_context_menu` /
   `UpdateMessage::ShowContextMenu` are no longer gated out on Android, and the
   secondary-pointer `handle_menu_events` trigger runs on Android. **Touch
   long-press → menu is now implemented** (`src/event/long_press.rs` +
   `LongPressState` in `WindowState` + Android `event()` dismiss handling), so
   touch devices can open context menus without a secondary button.
4. **File dialogs / external URLs — done (code-complete, unverified on device).**
   External URL opening (`open_url` → `ACTION_VIEW` intent) is fire-and-forget
   and needs no extra app wiring. File dialogs (`open_file` / `save_as` via SAF)
   launch the picker without extra wiring, but **receiving the result** requires
   the optional `FloemNativeActivity` Java subclass (shipped in
   `examples/android/FloemNativeActivity.java`) so its `onActivityResult` can
   forward to `floem::platform::dispatch_activity_result`. The default example
   manifest uses `android.app.NativeActivity` (works for everything except
   result delivery); swap to `dev.floem.android.FloemNativeActivity` + add the
   Java file to enable file-dialog results. Selected `content://` URIs are
   returned as `FileInfo.path` entries (not filesystem paths).
5. **`cargo ndk` APK build + device run** is unverified in CI here (no
   `cargo-ndk`, no NDK, no Android rustup target in this environment); the
   example is provided as a template. The **packaging** half
   (`aapt2`/`zipalign`/sign) *is* verified: `scripts/build-apk.sh --skip-build`
   produces a well-formed, page-aligned APK (checked with `aapt2 dump badging`
   + `zipalign -c`). The **Rust** half still needs an NDK.
6. **Drag & Drop — resolved by design, not a gap.** The `drag_drop` capability
   stays `false` on Android because the winit-android backend emits no
   `DragDropped`/`DragEntered`/`DragMoved`/`DragLeft` events and Android has no
   OS-level in-window file DnD (see the *Drag & Drop* section above). Floem's
   in-app `Event::Drag` / `DragTracker` system already works on Android through
   the touch→`PointerEvent` pipeline with **zero backend code** — no action needed.
   The only Android-relevant way to *receive* a file is an `Intent`/SAF, already
   covered by `open_external` / `file_dialog`.

> **Example `cargo check` caveat (historical):** the example used to be
> bin-only, and `cargo check -p floem-android-example --target
> aarch64-linux-android` reported `main function not found`, because with
> `android-activity`'s `native-activity` feature the `main` symbol is supplied
> by native-activity's C glue **at link time**. The crate is now a `cdylib`
> lib target (`src/lib.rs`) plus a host-only placeholder bin (`src/main.rs`),
> so `cargo check -p floem-android-example --target aarch64-linux-android`
> exercises `android_main` and the JNI bridges without needing the link step.

## Verification checklist

- [x] `cargo check -p floem --target aarch64-linux-android` — zero errors.
- [x] `cargo check -p floem` (host) — zero new errors (desktop regression-free).
- [x] `cargo test -p floem-test` — all suites green, 0 failed.
- [x] Android GPU surface path wired: `GpuResources::request` → `create_surface`
      from winit's `HasWindowHandle` (returns `ndk` `AndroidNdkWindowHandle`).
- [x] Android surface lifecycle wired: `destroy_surfaces` marks lost →
      `can_create_surfaces` rebuilds via `WindowHandle::recreate_gpu_surface`.
- [x] Android clipboard JNI bridge wired: `Clipboard::init_android` builds
      `AndroidClipboard` (`get_contents`/`set_contents` →
      `android.content.ClipboardManager` via the `jni` crate).
- [x] Android context-menu trigger wired: `show_context_menu` Android branch +
      `action::show_context_menu` / `UpdateMessage::ShowContextMenu` un-gated +
      `handle_menu_events` secondary-button trigger runs on Android.
- [x] Android external-URL opening wired: `platform::open_url` → JNI `ACTION_VIEW`
      intent (`src/platform/android_external.rs`); `open_external` capability true.
- [x] Android file dialogs wired: `platform::{open_file,save_as}` → SAF intents
      via `startActivityForResult` (`src/platform/android_file.rs`); public
      `dispatch_activity_result` entry + example `FloemNativeActivity.java` native
      bridge for result delivery; `file_dialog` capability true.
- [ ] Android APK build + on-device render (needs `cargo-ndk` + device).
- [ ] On-device verification of external-URL + file-dialog round-trips
      (incl. the optional `FloemNativeActivity` `onActivityResult` hook).
