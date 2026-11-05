<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# iOS Backend (Phase 2 — In Progress)

> **Status:** **Step 10 DONE (2026-08-11).** iOS skeleton + Metal surface +
> in-UI context menu + clipboard + **soft keyboard / IME** all compile green
> for `aarch64-apple-ios`, `aarch64-apple-ios-sim`, and the host (regression).
> A runnable `examples/ios` (counter + `TextInput` demoing the Step 5 IME) and
> the `cargo-xcode` build pipeline are in place (see §7 Step 10).
> **Steps 1–10 are complete (2026-08-11).** All ten planned iOS Phase-2 steps
> (skeleton + Metal surface + in-UI context menu + clipboard + IME + external
> URL + file dialog + receive-shared-files + safe-area + example) are
> implemented and compile green for `aarch64-apple-ios`,
> `aarch64-apple-ios-sim`, host, and `floom-ios-example` (device/sim/host).
> **One residual gap remains** (documented, not blocking): `receive_files`
> misses **cold-start** shares — Step 8 needs a `floem-winit` patch for full
> coverage. The former "no core layout consumer for `safe_area`" gap is now
> **closed** by the cross-platform core change tracked as issue **#110** (see
> new **Step 9b** below): `safe_area` insets now flow through the neutral
> `EmbedderEvent::SafeArea` protocol into `WindowState`, and when a window opts
> in via `WindowConfig::apply_safe_area(true)` Floom insets the **root layout**
> by the safe-area padding so content clears the notch / Home Indicator. This
> document was the design/plan; it now also serves as the implementation log.
>
> **Why iOS is tractable:** the lapce **winit fork** (`floem-winit`, rev
> `133268de`) already ships a first-class **iOS backend** (`floem-winit-ios`)
> that translates iOS touch/keyboard into standard winit `WindowEvent`s — so,
> unlike Android's `NativeActivity`/JNI dance, the iOS entry is just a normal
> Rust `fn main` that builds an `EventLoop` and runs it. winit internally calls
> `UIApplicationMain` with its own `UIApplicationDelegate`. Floom's existing
> winit event path (pointer/keyboard/`text_input`) therefore reuses with **no
> custom input translation**. The only backend-specific work is: an
> `embedder/ios.rs` adapter, an `Application::new_ios` entry, a **Metal
> (`UiKit`) GPU-surface branch** in the renderer, iOS `Cargo.toml` cfg, the
> `in-UI` context-menu renderer enabled for iOS, and an example/Xcode wrapper.

This document tracks the iOS backend for Floom, built on top of the
experimental [`embedder`](../src/embedder) protocol from
[`docs/superpowers/plans/2026-08-03-embedder-api.md`](../docs/superpowers/plans/2026-08-03-embedder-api.md).
It mirrors the structure of [`android.md`](./android.md) (Phase 1, done) so the
two mobile backends stay comparable.

## Architecture at a glance

```
Rust fn main()                                (examples/ios/src/main.rs)
  └─ floom::Application::new_ios(config)       (crates/app/floom-app/src — NEW)
       └─ winit EventLoop (floem-winit-ios)
            └─ UIApplicationMain + winit UIApplicationDelegate  (winit-internal)
                 └─ UIWindow / UIView (CAMetalLayer)
                      └─ wgpu Metal surface  ← renderer UiKit branch (NEW)
  └─ .window(view_fn).run()                    (identical to desktop / Android)

floom event path  (dispatch.rs, window/handle.rs — UNCHANGED, platform-agnostic)
  └─ pointer/keyboard/IME → standard winit events → Floom widgets

crates/platform/floom-platform/src/ios  (NEW)  — capabilities + PlatformServices + window + surface
  └─ ios_capabilities() / IosServices / IosEmbedderWindow / IosSurfaceHandle
```

Key point: **no core fork.** iOS plugs into the same `embedder` protocol and the
same `dispatch`/`window` code paths as desktop and Android. iOS differs from
Android only in (a) no JNI — it uses `objc2`/`objc2-ui-kit` FFI instead, and (b)
the GPU API is **Metal**, not Vulkan.

## 1. Application entry — `Application::new_ios`

Mirror `Application::new_android` (`crates/app/floom-app/src:323`), but **simpler** — no
`AndroidApp`/`with_android_app`, no JNI handles:

```rust
#[cfg(target_os = "ios")]
pub fn new_ios(mut config: AppConfig) -> Self {
    // iOS only supports the Metal backend; probing other backends is
    // pointless and can error. Default to Metal unless the caller chose.
    if config.wgpu_backends.is_none() {
        config.wgpu_backends = Some(wgpu::Backends::METAL);
    }

    // NOTE: the lapce winit fork has NO `EventLoopBuilderExtIOS`; a plain
    // `EventLoop::new()` is the correct iOS entry. winit internally calls
    // `UIApplicationMain` with its own `UIApplicationDelegate` when the loop
    // runs. No `AndroidApp`/JNI, no display-handle `Clipboard::init`, no macOS
    // app delegate, and no muda `MenuEvent` (muda has no iOS backend — iOS uses
    // the in-UI context menu like Android).
    let event_loop = EventLoop::new().expect("can't start the ios event loop");

    let event_loop_proxy = event_loop.create_proxy();
    let (sender, receiver) = channel();
    *EVENT_LOOP_PROXY.lock() = Some((event_loop_proxy.clone(), sender));

    let handle = ApplicationHandle::new(config);
    Self { receiver, handle, event_loop: Some(event_loop), initial_windows: Vec::new() }
}
```

The example `examples/ios/src/main.rs` then does the **exact desktop flow**:

```rust
fn main() {
    floom::Application::new_ios(AppConfig::default())
        .window(|_id| my_view(), None)
        .run();
}
```

winit's iOS backend turns this `fn main` into a proper UIKit app (it calls
`UIApplicationMain` with its own delegate when the `EventLoop` is built/run).
No Objective-C `main`/`.m` file is required.

> **Build/packaging (not Rust):** an iOS app still needs an **app bundle** +
> code signing. The recommended path is `cargo-xcode` (or `bones`) to generate
> an Xcode project that links the Rust `staticlib`, or a thin Xcode target that
> calls the Rust `fn main`. A minimal `Info.plist` is needed
> (`UILaunchScreen`, `UIApplicationSupportsIndirectInputEvents = true`,
> `UISupportedInterfaceOrientations`). Framework linking for `UIKit`/`Foundation`
> is handled by winit, but a `.cargo/config.toml` `[target.aarch64-apple-ios]`
> (and `-sim`) may need `rustflags = ["-C", "link-args=-framework UIKit"]` if
> winit doesn't pull it in transitively. This is documented, not implemented.

## 2. GPU surface — Metal (`UiKit` window handle)

**Update (2026-08-10, Step 2 done — no code change required):** The original
plan assumed Floom special-cased `RawWindowHandle::AppKit`/`Win32`/`AndroidNdk`
for surface creation and that iOS needed a new `RawWindowHandle::UiKit` branch.
**This is false.** Floom's surface creation is fully generic:

- `renderer/src/gpu_resources.rs:65` (`GpuResources::request`) and
  `crates/app/floom-app/src:275` (`WindowHandle::new_paint_state`) both call
  `instance.create_surface(Arc::clone(&window))` with an `Arc<dyn winit::window::Window>`
  — **no raw-handle matching at all**. wgpu's `create_surface` internally takes
  `&dyn HasWindowHandle` and dispatches on the raw handle type.
- The only `RawWindowHandle::AppKit` matches in the codebase are for **macOS
  traffic-light constraints** (`crates/app/floom-app/src:646`, gated `target_os =
  "macos"`) and **IME positioning** (`crates/app/floom-app/src:1623/1665/1684/1705`,
  gated macos/windows) — neither touches surface creation.
- This is exactly why **Android needed no surface-code change in Phase 1**: its
  `AndroidNdk` handle flows through the same generic `create_surface` call.

So iOS is covered for free. The chain:

1. floom passes `Arc<dyn winit::window::Window>` → generic `create_surface`.
2. winit fork's iOS `Window::raw_window_handle` returns
   `RawWindowHandle::UiKit { ui_view, ui_window, ui_view_controller }`
   (`~/.cargo/git/checkouts/winit-…/9404575/src/platform_impl/ios/window.rs:334`).
3. wgpu's **Metal** backend consumes the `UiKitWindowHandle` and builds a
   `CAMetalLayer` surface from `ui_view`.

Because Metal surfaces are tied to the `CAMetalLayer` (not a volatile
`ANativeWindow*`), **iOS has no surface-lifecycle rebuild problem** — rotation/
resize just changes the layer's bounds, and winit emits `WindowEvent::Resized`,
which Floom already handles. Therefore **`WindowHandle::recreate_gpu_surface`
stays `#[cfg(target_os = "android")]` only** (Step 1 intentionally kept the
GPU-surface-lifecycle gate out of iOS) — do **not** add a UiKit branch there.

`Metal` is selected by default (`Backends::METAL`) in `new_ios` (Step 1),
matching how Android selects `Backends::VULKAN`.

**Verification:** `cargo check -p floom --target aarch64-apple-ios` and
`--target aarch64-apple-ios-sim` are green with the surface-creation code
compiled (no `UiKit` branch added). Runtime surface build is the usual
compile-only-until-simulator caveat (§8).

## 3. Embedder adapter — `crates/platform/floom-platform/src/ios`

Mirror `crates/platform/floom-platform-android/src/android.rs` (same trait impls). New file,
`#![cfg(target_os = "ios")]`, registered in `crates/platform/floom-platform/src` as
`#[cfg(target_os = "ios")] pub mod ios;`.

| Item | Purpose |
|------|---------|
| `ios_capabilities() -> BackendCapabilities` | the iOS preset (table below) |
| `IosServices` (`PlatformServices`) | clipboard (UIPasteboard), theme, external URL, file dialogs, share |
| `IosEmbedderWindow` (`EmbedderWindow`) | `request_redraw` is a no-op (winit owns the redraw cycle, like Android) |
| `IosSurfaceHandle` (`SurfaceHandle`) | wraps a `SurfaceDescriptor` |
| `ios_surface_descriptor(logical, pixel, scale) -> SurfaceDescriptor` | `SurfaceKind::GpuSurface`, `transparent` per need |

`PlatformServices::capabilities()` returns `ios_capabilities()`.

### Proposed `BackendCapabilities` for iOS

| Capability | iOS | Notes |
|------------|------|-------|
| `clipboard` | **true** | via `UIPasteboard` (`objc2-ui-kit`) — replace the `copypasta` macOS-only path that `clipboard.rs` currently selects for `ios`. Bridge mirrors Android's JNI clipboard but uses `objc2` instead. |
| `cursor` | true | iPad pointer/cursor; hidden on pure touch. |
| `timers` | true | |
| `file_dialog` | **true** | **implemented (Step 7)** via `platform::ios_file::{open_file, save_as}` — a `UIDocumentPickerViewController` presented from the key window's root view controller, with a runtime-declared `UIDocumentPickerDelegate` (`ClassBuilder` subclass, since the Rust `UIDocumentPickerDelegate` trait needs the Rust-2024 `#[unsafe(...)]` attributes this crate can't use) delivering the picked `NSURL`s back through `did_pick_documents` → `FileInfo`. `public.data` selects all files (no `allowed_types`→UTI mapping yet); directory selection (`select_directories`) is ignored (the picker is files-only). |
| `context_menu` | **true** | **in-UI menu** (muda has no iOS backend). Requires enabling the in-UI `context_menu_view` for iOS — see §5. |
| `window_title` | false | iOS is full-screen; no title bar. |
| `accessibility` | false | VoiceOver integration is **Phase 4**; left off for now. |
| `system_theme` | true | `UITraitCollection.userInterfaceStyle` (light/dark). |
| `display_info` | true | |
| `safe_area` | **true** | **implemented (Step 9):** `EmbedderWindow::safe_area()` (new trait method, zero default so other backends are unaffected) reads `UIView.safeAreaInsets` from the key window and converts points → physical pixels via `UIScreen.scale`. **Core consumer now lands in Step 9b (#110):** the insets flow through `EmbedderEvent::SafeArea` → `WindowState`, and when a window opts in via `WindowConfig::apply_safe_area(true)` Floom insets its **root layout** by the safe-area padding so content clears the notch / Home Indicator. The raw insets are also readable any time via `WindowHandle::safe_area()`. |
| `virtual_keyboard` | **true** | iOS soft keyboard shown/hidden via a hidden `UITextField` first-responder bridge (`platform/ios_ime.rs`, Step 5). |
| `open_external` | **true** | `UIApplication.openURL` (`platform/ios_external.rs`, Step 6) — `floom::platform::open_url` now works on iOS like on Android. The non-deprecated `openURL:options:completionHandler:` needs the `block2` feature, so the simpler deprecated form is used (scoped `#[allow(deprecated)]`). |
| `receive_files` | **true** | **implemented (Step 8)** via `platform::ios_receive` — `application:openURL:options:` is injected onto winit's `WinitApplicationDelegate` (a `class_addMethod` on the live class). **Warm-start shares are caught; cold-start shares (app launched via the share sheet) are missed** because the method is installed on the main queue *after* `UIApplicationMain` registers the class. Files become `SharedFile { uri, mime_type: None, display_name }`; `file://` URLs yield a decoded filesystem path. The fully-reliable fix is to patch `floem-winit` to forward `openURL` (documented below). |
| `drag_drop` | **false** | **by design**, like Android — winit-ios does not surface OS file drag-and-drop; revisit if winit gains `UIDragInteraction` events. Floom **in-app** drag works unchanged. |
| `ime` | **true** (partial) | **Step 5 done (partial).** iOS winit fork emits no `WindowEvent::Ime`, so a hidden `UITextField` first-responder + `UITextFieldDelegate` re-injects `Event::Ime` (`Commit`/`Delete`/`Preedit`) into Floom. Latin text entry + backspace work; CJK marked-text composition is best-effort (`markedTextRange` preedit not yet wired). |
| `gpu_surface` | **true** | Metal (§2). |
| `software_surface` | false | |
| `headless` | false | |

## 4. `Cargo.toml` — iOS target cfg

```toml
[target.'cfg(target_os = "ios")'.dependencies]
objc2 = { version = "0.6", default-features = false }
objc2-foundation = { version = "0.3", default-features = false, features = ["NSString", "NSURL"] }
objc2-uniform-type-identifiers = { version = "0.3", default-features = false, features = ["UTType"] }
objc2-ui-kit = { version = "0.3", features = [
  "UIApplication",
  "UIWindow",
  "UIView",
  "UIViewController",
  "UIPasteboard",
  "UIDocumentPickerViewController",
  "UIResponder",
  "objc2-uniform-type-identifiers",
  "block2",
] }

# NOTE: `UIKeyboard` is NOT a feature in objc2-ui-kit 0.3.2 (no such feature
# exists) — it was dropped from the actual Cargo.toml. The iOS soft keyboard is
# driven instead via `UIResponder` first-responder status (see Step 5 / IME).
# `UIViewController` + `objc2-uniform-type-identifiers`(`UTType`) + `block2`
# were added for the Step 7 file dialog (`UIDocumentPickerViewController`
# presentation + `NSURL`/`UTType`/`presentViewController:animated:completion:`);
# `NSURL` was added to `objc2-foundation` for the same.
```

- `copypasta` is already pulled in for `cfg(not(target_os = "android"))`, so iOS
  gets it today — but its macOS `objc` clipboard won't function on iOS; the
  `IosServices` clipboard impl should use `UIPasteboard` directly and bypass
  `copypasta` on iOS (mirrors Android's JNI clipboard taking precedence over
  `copypasta`).
- `winit` workspace dep already covers iOS (no extra feature flag needed, unlike
  Android's `android-native-activity`).
- No `ndk`/`android_logger` on iOS. Use `os_log`/`env_logger` as desired.

## 5. in-UI context menu must be enabled for iOS

**Update (2026-08-10, Step 3 done — no new code):** every cfg listed below was
already extended to `target_os = "ios"` as part of Step 1's `any(android, ios)`
gating (see "Step 1 changelog"). The in-UI context menu is therefore fully
wired on iOS, identical to Android.

The long-press → context-menu feature implemented in Phase 1 is
**platform-agnostic** (`crates/core/floom-types/src` keys off `PointerType::Touch`,
which winit-ios produces), so it works on iOS automatically **once the in-UI
menu renderer is mounted**. Today `context_menu_view` and the in-UI
`show_context_menu` are compiled only for
`any(linux, freebsd, wasm32, android)`. To light up on iOS, extend those cfgs to
include `target_os = "ios"` in:

- `crates/app/floom-app/src` — `context_menu` signal creation + `context_menu_view`
  mount (lines ~147, ~153) and the in-UI `show_context_menu` (the
  `any(linux, freebsd, wasm32, android)` block).
- `crates/app/floom-app/src` — the `event()` dismiss block (`SignalWith` import cfg
  + the `is_pointer_down`/`is_pointer_up` cfg + the dismiss logic). Add iOS to
  the **android** dismiss branch (or unify the android/ios branch) so a
  long-press menu is not dismissed by the finger-lift that ends the gesture.
- `crates/platform/floom-platform/src` is already platform-neutral; no change.

This is a small, contained cfg-extension and is the main "UI" piece for iOS.

## 6. Differences from Android (so we don't copy blindly)

| Area | Android | iOS |
|------|---------|-----|
| Entry | `android_main` + `NativeActivity` + JNI | plain Rust `fn main`; winit calls `UIApplicationMain` |
| FFI | `jni` crate, `JavaVM`/`Activity` global refs | `objc2`/`objc2-ui-kit` (`Retained`, `MainThreadMarker`) |
| GPU | Vulkan (`ANativeWindow*`, volatile → rebuild on rotate) | Metal (`CAMetalLayer`, stable → no rebuild) |
| Clipboard | `ClipboardManager` via JNI | `UIPasteboard` via `objc2` |
| File picker | SAF `DocumentActivity` + `onActivityResult` | `UIDocumentPickerViewController` + delegate callback |
| Open URL | `ACTION_VIEW` intent | `UIApplication.open` |
| Receive share | `Intent` (`ACTION_SEND`/`VIEW`) via JNI | `UIApplicationDelegate.application(_:open:)` / scene URL contexts |
| Context menu | in-UI (no muda Android) | in-UI (no muda iOS) — **same code** |
| Keyboard/IME | soft keyboard via `InputMethodManager` | soft keyboard via `UIResponder` first-responder + `UITextInput` |

## 7. Phased checklist (all pending)

- [x] **Step 1 — Skeleton + compile.** `crates/platform/floom-platform/src/ios`
      (`ios_capabilities`/`IosServices`/`IosEmbedderWindow`/`IosSurfaceHandle`/
      `ios_surface_descriptor`), register in `embedder/mod.rs`,
      `Application::new_ios` in `app/mod.rs`, iOS `Cargo.toml` cfg.
      **Done 2026-08-10:** `cargo check -p floom --target aarch64-apple-ios`,
      `--target aarch64-apple-ios-sim`, and host (`cargo check -p floom`) are
      all green. See "Step 1 changelog" below for the cfg edits required.
- [x] **Step 2 — Metal (`UiKit`) GPU surface.** **No code change required** —
      Floom's surface creation is generic (`instance.create_surface(window)`, no
      raw-handle match), so iOS is covered like Android was in Phase 1. winit
      fork's iOS `Window` returns `RawWindowHandle::UiKit`; wgpu Metal backend
      builds the `CAMetalLayer` surface. `recreate_gpu_surface` stays
      `target_os = "android"`-only. `Backends::METAL` already default in
      `new_ios`. **Done 2026-08-10:** both iOS targets compile green with the
      surface code compiled.
- [x] **Step 3 — Enable in-UI context menu for iOS** (§5 cfgs). **Done
      2026-08-10.** The §5 cfg extensions were already applied during Step 1's
      comprehensive `any(android, ios)` gating (see Step 1 changelog), so no
      new code was needed here. Verified by code review + both iOS targets
      compile green: `context_menu_view` is mounted (`window/handle.rs:159-168`),
      the `UpdateMessage::ShowContextMenu` arm dispatches straight to the in-UI
      renderer (`window/handle.rs:1343-1356`), the long-press dismiss logic
      covers iOS (`window/handle.rs:471-489`), `ios_capabilities().context_menu
      = true`, and `platform/context_menu.rs` is fully platform-neutral (uses
      the `wasm_stubs` `Menu`/`MudaMenu` like Android). Runtime on simulator is
      still gated behind Step 10.
- [x] **Step 4 — Clipboard (`UIPasteboard`).** **Done 2026-08-11.** `objc2`/`objc2-ui-kit` bridge in `crates/platform/floom-platform-desktop/src/clipboard.rs` (new, gated `target_os = "ios"`): `get_contents` reads `UIPasteboard::generalPasteboard().string()` → `String`; `set_contents` builds an `NSString` (`objc2-foundation`'s `NSString::from_str`) and calls `setString`. `Clipboard::get_contents`/`set_contents` (the core API used by `text_input`/`editor`/`label`/`inspector`) now dispatch to this bridge on iOS, exactly mirroring Android's JNI path; `IosServices::read/write_clipboard` delegate to the same `Clipboard` API (like `android.rs`). `ios_capabilities().clipboard = true`. `copypasta` is dropped from iOS (no iOS backend; it would `unwrap()`-panic). Both iOS targets + host compile green.
- [x] **Step 5 — Virtual keyboard / IME.** **Done 2026-08-10.** iOS winit fork
      emits **no** `WindowEvent::Ime` and its `set_ime_allowed` is a no-op, so
      the desktop `request_ime_update` path panics on iOS — that block is
      gated out (`window/handle.rs:1375` `SetImeAllowed` →
      `set_keyboard_visible`). Instead a hidden `UITextField` (zero-size frame
      + alpha 0.0, added to the key window) becomes first-responder on
      `text_input` focus, and a runtime-declared `UITextFieldDelegate`
      (`ClassBuilder`, since `UIKeyboard` is not a feature so `unsafe trait
      UITextFieldDelegate` is unavailable) re-injects `Event::Ime` (`Commit`
      from `shouldChangeCharactersInRange` / Return key; `Delete` from UTF-16
      range → UTF-8 bytes; `Enabled`/`Disabled` from focus) via a new
      `UserEvent::IosIme` injection channel (`app/mod.rs` + `app/handle.rs`).
      Code: `crates/platform/floom-platform/src` (new), `app/mod.rs`, `app/handle.rs`,
      `window/handle.rs:1375`, `embedder/ios.rs` (`ime`/`virtual_keyboard =
      true`). Both iOS targets + host compile green (re-verified 2026-08-11:
      `aarch64-apple-ios` 48s / `-sim` 32s, 0 errors). Known limit: CJK
      marked-text composition is best-effort (preedit not yet wired from
      `markedTextRange`). The `Preedit`/`Enabled` variants are `#[allow(dead_code)]`
      (reserved for that future CJK wiring); the only other 7 warnings are
      pre-existing `headless.rs` visibility warnings, unrelated to iOS.
- [x] **Step 6 — External URL (`UIApplication.openURL`).** **Done 2026-08-11.**
      Mirror Android's `ACTION_VIEW` bridge (`android_external.rs`) as a new
      `crates/platform/floom-platform/src` (`#![cfg(target_os = "ios")]`) using
      `objc2`/`objc2-ui-kit` instead of JNI. `UIApplication::sharedApplication`
      + `NSURL::URLWithString` + the deprecated `openURL:` (the non-deprecated
      `openURL:options:completionHandler:` needs the `block2` feature, so the
      simpler deprecated form is used with a **function-level**
      `#[allow(deprecated)]` — a statement-level allow does *not* suppress the
      warning). `floom::platform::open_url` now dispatches to
      `ios_external::open_url` on iOS via a 3-way cfg in `platform/mod.rs`,
      returning an `Err` on other platforms ("only implemented on Android and
      iOS"). `ios_capabilities().open_external = true`; §3 capability table
      updated. Both iOS targets + host compile green (re-verified 2026-08-11: 0
      errors; the only remaining warnings are the pre-existing `headless.rs`
      visibility ones — `openURL:` and `keyWindow()` deprecation warnings are
      suppressed via function-level `#[allow(deprecated)]` in `ios_external.rs`
      and `ios_ime.rs::ensure_field`).
- [x] **Step 7 — File dialogs (`UIDocumentPickerViewController`).** **Done
      2026-08-11.** Mirror Android's SAF bridge (`android_file.rs`) as a new
      `crates/platform/floom-platform/src` (`#![cfg(target_os = "ios")]`) using `objc2`/
      `objc2-ui-kit` instead of JNI. `floom::platform::{open_file, save_as}`
      now dispatch to `ios_file` on iOS (3-way cfg in `platform/mod.rs`,
      mirroring the `file_action`/Android split — `file_action` stays
      `not(any(wasm32, android, ios))`). The bridge presents a
      `UIDocumentPickerViewController` from the key window's root view
      controller and receives the chosen `NSURL`s through a **runtime-declared
      `UIDocumentPickerDelegate`** (`ClassBuilder` subclass named
      `FloomDocPickerDelegate`, with `documentPicker:didPickDocumentsAtURLs:`
      and `documentPickerWasCancelled:` methods) wired via a raw
      `msg_send![&picker, setDelegate:]` — same bypass as the IME bridge
      (`ios_ime.rs`), because the Rust `UIDocumentPickerDelegate` trait requires
      the Rust-2024 `#[unsafe(...)]` attributes, which this edition-2021 crate
      cannot use. Picked URLs become `PathBuf`s → `FileInfo { path, format:
      None }`, delivered to the caller through a `FnOnce(Option<FileInfo>)`
      callback marshaled onto a reactive scope via `create_ext_action` (like
      Android). `ios_capabilities().file_dialog = true`. **Key objc2 API notes
      (step-internal gotchas, recorded for next time):** (1) `Allocated<T>` does
      *not* implement `Deref`, so init methods (`initForOpeningContentTypes:` /
      `initForExportingURLs:`) that take `this: Allocated<Self>` must be called
      as **associated functions** (`UIDocumentPickerViewController::
      initForOpeningContentTypes(mtm.alloc::<…>(), &types)`) — method syntax
      fails with E0599. (2) `mtm.alloc::<T>()` (the `MainThreadMarker::alloc`
      helper at `objc2-0.6.4/src/main_thread_marker.rs:269`) is the correct
      allocator for `MainThreadOnly` classes; `Type::alloc(mtm)` is not
      in-scope. (3) `UTType::data()` does not exist — use
      `UTType::exportedTypeWithIdentifier(&NSString::from_str("public.data"))`
      (needs the `UTType` feature on `objc2-uniform-type-identifiers`). (4)
      `keyWindow()` is deprecated (multi-scene); suppress with a
      **function-level** `#[allow(deprecated)]` (single-scene app, Info.plist
      `UIApplicationSupportsMultipleScenes=false`). Both iOS targets + host
      compile green (re-verified 2026-08-11: 0 errors).
- [x] **Step 8 — Receive shared files/URLs (`UIApplicationDelegate` open URL).**
      **Done 2026-08-11.** Mirror Android's `ACTION_SEND`/`ACTION_VIEW`
      receiver: a new `crates/platform/floom-platform/src` (`#![cfg(target_os =
      "ios")]`) exposes `set_shared_files_handler(Fn(Vec<SharedFile>))` (same
      signature/thread-local `HANDLER`+`PENDING` queue as `android_intent.rs`,
      since the handler/`RwSignal` is `!Send`) and an `install_openurl_hook()`
      called from `Application::new_ios`. **The hard part: winit's fork owns the
      `UIApplicationDelegate`** (class `WinitApplicationDelegate`, passed by name
      to `UIApplicationMain`) and does **not** implement
      `application:openURL:options:` — and it registers that class lazily
      *inside* `EventLoop::run()` (right before `UIApplicationMain`), so the
      class does not exist at `new_ios` time. Solution: schedule the injection
      on the **main queue** via `dispatch_async_f` (GCD, part of libSystem on
      Apple platforms — no extra link); the block runs after `UIApplicationMain`
      has registered the class, then `class_addMethod(cls,
      sel!(application:openURL:options:), open_url_imp, "c@:@@@")` adds the
      method. `open_url_imp` (an `Imp`, reached via `std::mem::transmute` since
      its real signature can't be cast directly to untyped `Imp`) reads the
      `NSURL`, converts `file://` URLs to a decoded path (via `NSURL::
      to_file_path`) and others to the absolute URL string, and forwards a
      `SharedFile` to the registered handler. `ios_capabilities().receive_files
      = true`.   **Cold-start limitation RESOLVED** — the original limitation was that
  iOS delivers the launch URL inside `UIApplicationMain`'s launch sequence,
  *before* the main-queue `class_addMethod` block runs, so cold-start shares
  were missed (warm-start shares were caught). This is now fixed in **Step 8b
  (2026-08-13)**: the patched `floem-winit` installs a custom
  `FloomWinitApplicationDelegate` that captures the cold-start URL from
  `launchOptions[UIApplicationLaunchOptionsURLKey]` (and the warm-start
  `openURL:`), so **both** cold- and warm-start shares funnel into
  `set_shared_files_handler`. `SharedFile` doc + `set_shared_files_handler` 3-way cfg in
      `platform/mod.rs` updated (Android / iOS / other-no-op). Both iOS targets
      + host + `floom-ios-example` (device/sim/host) compile green
      (re-verified 2026-08-11: 0 errors).

- **Step 8b — Cold-start fix (patch `floem-winit`).** **Done 2026-08-13.**
  Implemented, not just documented. Root cause confirmed: winit 25.10.0
  (lapce/winit@133268de, the git rev floom pins) is **purely
  `NSNotificationCenter`-driven** — `EventLoop::run` calls
  `UIApplication::main(None, None, mtm)` with **no custom delegate**, so the
  cold-start launch URL in `launchOptions[UIApplicationLaunchOptionsURLKey]`
  is never read. An earlier out-of-tree fork was the **wrong base** (a
  0.29.5 published artifact, API-incompatible with the 25.10.0
  `objc2`/`define_class!` world) and its `[patch]` was silently ignored →
  `set_on_open_url` unresolved (E0425). Fix: rebuilt the fork from the cached
  checkout `lapce/winit@133268de` (= 25.10.0 virtual workspace, members
  `dpi`/`winit*`); it now lives at `crates/vendor/floem-winit/`.

  **Wiring** — workspace root `Cargo.toml`:
  ```toml
  # floom #109: same rev, same version (25.10.0) → no [patch] drift risk.
  # The fork root is a *virtual* workspace, so the patch MUST point at the
  # `floem-winit` member package (`winit/`), not the workspace root
  # (which would error "found a virtual manifest").
  [patch."https://github.com/lapce/winit"]
  floem-winit = { path = "crates/vendor/floem-winit/winit" }
  ```

  **Fork changes** (all in `crates/vendor/floem-winit/`):
  - `winit-uikit/src/app_delegate.rs` (new): `define_class!` defines
    `FloomWinitApplicationDelegate`
    (`#[name = "FloomWinitApplicationDelegate"]`,
    `#[thread_kind = MainThreadOnly]`). `unsafe impl UIApplicationDelegate`
    implements `did_finish_launching_with_options` (reads
    `launchOptions[UIApplicationLaunchOptionsURLKey]` → `NSURL`) **and**
    `open_url` (warm-start). A `Mutex<Option<OnOpenUrl>>` registered via
    `pub unsafe fn set_on_open_url(...)`; `forward_open_url` invokes it with a
    raw `&NSURL` pointer — the **same channel** the floom-side trampoline in
    `ios_receive.rs` already drains.
  - `winit-uikit/src/lib.rs`: `pub use self::app_delegate::set_on_open_url;`
    re-exports it from the uikit backend.
  - `winit/src/lib.rs`: `#[cfg(target_os = "ios")] pub use
    winit_uikit::set_on_open_url;` so floom's `winit::set_on_open_url` resolves
    against the patched `floem-winit` crate.
  - `winit-uikit/src/event_loop.rs` (`run_app`): installs the delegate —
    ```rust
    let delegate_class = delegate_class();
    app_state::launch(self.mtm, app, || {
        UIApplication::main(None, Some(&delegate_class), self.mtm)
    })
    ```
    winit's own lifecycle (resume/suspend) is notification-driven, so adding a
    URL-only delegate does **not** interfere with those events.

  **floom consumer** — `crates/platform/floom-platform/src` already calls
  `winit::set_on_open_url(Some(trampoline))` from `install_openurl_hook()`;
  fixed an E0133 (raw-ptr deref inside `unsafe fn` needs an inner `unsafe`
  block) at `open_url_trampoline`.

  **Verification (2026-08-13):** `cargo check` **Finished 0 error** on all
  four targets — `aarch64-apple-ios` (device),
  `aarch64-apple-ios-sim` (simulator), host `x86_64-apple-darwin`, and
  `floom-ios-example`. Only benign warnings (pre-existing
  `block v0.1.6` future-incompat + dead-code) remain.
  **Still UNVERIFIED on-device:** `cargo check` does not link or run, so the
  runtime share-sheet cold-start path needs a real `cargo build` + device/sim
  launched via the share sheet to confirm.
- [x] **Step 9 — Safe-area insets (`UIView.safeAreaInsets`).** **Done
      2026-08-11.** The `safe_area` capability previously existed as a flag but
      had **no API** (and no core consumer — Android also claimed `safe_area:
      true` without implementing it). Added `fn safe_area(&self) ->
      PhysicalInsets<u32>` to the `EmbedderWindow` trait
      (`crates/app/floom-app/src/services.rs`) with a **zero default** so existing backends
      (Android, mock) are unaffected, then implemented it on
      `IosEmbedderWindow` (`crates/platform/floom-platform/src/ios`): fetch the key window via
      `UIApplication::sharedApplication(mtm).keyWindow()` (main-thread only;
      `MainThreadMarker::new()` guards off-main calls → zeros), read
      `UIView.safeAreaInsets` (`UIEdgeInsets` in points), and convert to device
      pixels via `UIScreen::mainScreen(mtm).scale()`. `ios_capabilities().
      safe_area` flipped `false` → `true`. Cargo.toml iOS cfg gained
      `UIGeometry` + `objc2-core-foundation` (both needed for
      `safeAreaInsets` / `scale`). **Key gotchas:** (1) `keyWindow` and
      `UIScreen::mainScreen` are deprecated (multi-scene) → suppressed with a
      **function-level** `#[allow(deprecated)]` (single-scene app, Info.plist
      `UIApplicationSupportsMultipleScenes=false`); (2) `safeAreaInsets` is
      gated `all(feature = "UIGeometry", feature = "objc2-core-foundation")` —
      both features must be on. **Documented limitation:** no core layout
      consumer of `safe_area()` exists yet (same as Android) — wiring the
      insets into layout is a separate follow-up core change; the API is in
      place for it. Both iOS targets + host + `floom-ios-example`
      (device/sim/host) compile green (re-verified 2026-08-11: 0 errors).
- [x] **Step 9b — Safe-area core layout consumer (issue #110).** **Done
      2026-08-12.** Closes the "no core layout consumer" gap from Step 9. The
      neutral `EmbedderEvent` protocol gained a new `SafeArea { window_id,
      insets: PhysicalInsets<u32> }` variant (with a `PartialEq` branch so
      `process_update` dedups it). `WindowState` gained `safe_area:
      RwSignal<PhysicalInsets<u32>>` (init `PhysicalInsets { 0,0,0,0 }`) and an
      opt-in `apply_safe_area: bool` flag (default `false`). When `apply_safe_area`
      is set, `compute_layout()` adds the insets as **root padding** (`insets /
      effective_scale`, i.e. physical px → layout px) so the whole tree clears the
      notch / status bar / Home Indicator. `handle_embedder_event` gained a
      `SafeArea` arm that updates the signal and, only when opted in, marks
      `needs_layout` + re-renders (`WindowHandle::safe_area()` exposes the signal
      read-only for apps that prefer to manage their own padding). `WindowConfig`
      gained `apply_safe_area(bool)` builder + passthrough through
      `app/handle.rs` → `WindowHandle::new`. **iOS emitter (the missing piece):**
      `crates/platform/floom-platform/src/ios` gained a free `read_safe_area() -> PhysicalInsets<u32>`
      (constructs an `IosEmbedderWindow` and reads the key window — id is
      irrelevant), and `crates/app/floom-app/src`'s `WindowEvent::SurfaceResized` arm
      (`target_os = "ios"` only) now calls `read_safe_area()` and pushes an
      `EmbedderEvent::SafeArea` into `handle_embedder_event` — so the insets flow
      on every rotation / orientation change. The `examples/ios` app opts in via
      `.window(view, Some(WindowConfig::default().apply_safe_area(true)))`.
      **Why `apply_safe_area` is opt-in (not automatic):** turning safe-area into
      implicit root padding would silently shift every existing desktop app's
      layout, so the default stays `false` (zero-regression); only windows that
      explicitly opt in get the inset, while all windows can still read the raw
      insets via `WindowHandle::safe_area()`. **Verification:** host
      `cargo check -p floom` Finished (only the pre-existing `headless.rs`
      visibility + `safe_area` "never used" warnings — the latter is expected,
      it's an app-facing API); `cargo check -p floom --target
      aarch64-apple-ios-sim` Finished 0 errors (the only `warning[E0133]` is a
      pre-existing raw-pointer deref in the iOS backend, unrelated to #110);
      `cargo check -p floom-ios-example --target aarch64-apple-ios-sim` Finished
      0 errors. (Device/simulator *runtime* inset behavior is the same
      compile-only-until-simulator caveat as the rest of iOS.)
- [x] **Step 10 — Example + build pipeline + this doc → status.** **Done
      2026-08-11.** Added `examples/ios/`:
      - `Cargo.toml` — package `floom-ios-example`, depends on `floom` (path);
        `edition = "2024"` to match the workspace. No explicit `winit` dep
        needed (unlike Android) because `new_ios` builds/owns the `EventLoop`.
      - `src/main.rs` — real `fn main` (iOS target) calls
        `Application::new_ios(AppConfig::default()).window(app_view).run()`;
        `app_view` is a `Stack` counter + `TextInput` + mirror label that
        exercises the Step 5 soft-keyboard/IME bridge; a `#[cfg(not(ios))]`
        placeholder `main` keeps host builds green (mirrors `examples/android`).
      - `Info.plist` — `CFBundle*`, `LSRequiresIPhoneOS`, `UILaunchScreen`,
        `UIApplicationSupportsIndirectInputEvents` (iPad pointer/keyboard),
        `UISupportedInterfaceOrientations`, `UIRequiredDeviceCapabilities`.
      - `README.md` — `cargo-xcode` → Xcode → simulator/device run, plus
        compile-only `cargo check -p floom-ios-example --target aarch64-apple-ios(-sim)`.
      Verified: `cargo check -p floom-ios-example` for **both iOS targets + host
      all Finished, 0 errors** (re-verified 2026-08-11). This document's Status
      header flipped to "Step 10 DONE" and the implemented subset (Steps 1–5 +
      10) is recorded above; Steps 6–9 remain pending.

### Step 1 changelog (cfg gating that was applied)

These gates mirror Android everywhere **except** the GPU-surface lifecycle
(`gpu_surface_lost` / `recreate_gpu_surface` / `mark_android_surfaces_lost`) —
those stay `target_os = "android"` only, because Metal surfaces are stable and
don't need rebuild-on-rotate.

- **Cargo.toml:** desktop deps split so `rfd` is excluded on iOS (`rfd` has no
  iOS backend), but `copypasta` stays (it no-ops on iOS and `clipboard.rs`
  already has an `ios` branch). Added
  `[target.'cfg(target_os = "ios")'.dependencies]` with `objc2` /
  `objc2-foundation` / `objc2-ui-kit` (features without `UIKeyboard`).
- **`platform/mod.rs`:** `file_action`/`open_file`/`save_as` gated
  `not(any(wasm32, android, ios))`; `context_menu` and `wasm_stubs` modules
  extended to `any(wasm32, android, ios)`.
- **`lib.rs` / `menu_types.rs` / `action.rs`:** all `muda` re-exports and the
  native `set_window_menu`/`window_menu` decorator gated
  `not(any(wasm32, android, ios))` (iOS has no native window menu — in-UI only).
- **`app/mod.rs` / `app/handle.rs` / `window/handle.rs`:** `MenuWrapper`,
  `ShowContextMenu` event, `pending_context_menus`, `window_menu_actions`,
  `window_menu` field, the multi-line `SignalWith` import, the in-UI
  `show_context_menu` dispatch, and the long-press dismiss branch all extended
  to `any(android, ios)`.
- **`embedder/mod.rs`:** `#[cfg(target_os = "ios")] pub mod ios;`.
- **`message.rs`:** `UpdateMessage::WindowMenu` variant gated
  `not(any(wasm32, android, ios))` so it does not exist on iOS (matched the
  already-iOS-excluded match arm — fixes E0004).

## 8. Risks / open questions

- **winit fork iOS completeness.** The lapce fork is Android-focused; confirm
  its iOS backend builds for `aarch64-apple-ios` and emits the expected
  `WindowEvent`s (esp. `Ime`, `Resized`, `Touch`→`Pointer`). First
  `cargo check` in Step 1 will surface this immediately.
- **IME is the hard part.** iOS has no "IME" in the desktop sense; text input
  is the OS keyboard driven by the view's first-responder status. Floom's
  `text_input` assumes winit `Ime` events — Step 5 must bridge
  `UITextInput`. Budget extra time here.
- **`muda` has no iOS backend** → context menu is in-UI only (same as Android);
  do not attempt native `UIMenu` via muda.
- **Code signing / app bundle.** Rust compiles fine, but *running* on a device
  requires an Apple developer identity and Xcode packaging. Verification here is
  limited to `cargo check` (device + simulator) and, at best, launching in the
  **iOS Simulator** via an Xcode-hosted Rust `staticlib`. True on-device render
  is the final unverified step (same constraint class as Android's APK).

## 9. Verification targets

| Check | Command | Notes |
|-------|---------|-------|
| Compile (device) | `cargo check -p floom --target aarch64-apple-ios` | needs Xcode iPhoneOS SDK (present) |
| Compile (sim) | `cargo check -p floom --target aarch64-apple-ios-sim` | run on Apple-silicon Mac |
| Host (regression) | `cargo check -p floom` | must stay green |
| Android (regression) | `cargo check -p floom --target aarch64-linux-android --offline` | must stay green |
| Example (device) | `cargo check -p floom-ios-example --target aarch64-apple-ios` | the `examples/ios` app |
| Example (sim) | `cargo check -p floom-ios-example --target aarch64-apple-ios-sim` | the `examples/ios` app |
| Example (host) | `cargo check -p floom-ios-example` | placeholder `main`, must stay green |
| Runtime | iOS Simulator via Xcode-hosted `staticlib` | on-device signing is the only fully-unverified step |

> All step checkpoints above are **compile-only** until an Xcode-hosted
> simulator run is wired — identical to how Android Phase 1 was
> verified (compile green, on-device render pending). The `examples/ios` app
> compiles for both iOS targets + host (Step 10).
