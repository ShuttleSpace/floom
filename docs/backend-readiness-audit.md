# Backend readiness audit (2.0 platform layer)

Audit date: 2026-10-15. Scope: the `crates/floom-platform*` adapters that the
2.0 facade (`crates/floom`) and the 2.0 examples actually link, judged against
"does it reach the OS, and is it production-grade".

This file exists because the platform *documents* (`docs/ios.md`, `docs/android.md`,
`docs/ohos.md`, `docs/gates/*`) describe a **different stack** from the one the
1.0/2.0 code ships. Reading them alone produces a false all-clear.

## The two stacks

| Stack | Crates | Who uses it | Verified? |
|---|---|---|---|
| **v1 monolith** | `crates/floom/src/platform/*`, `crates/floom/src/embedder/*` | the old `floom` monolith | Run-verified on device (`docs/gates/*`: Android, iOS, OHOS, macOS) |
| **2.0 adapters** | `crates/floom-platform*`, `crates/floom-renderer-*` | the facade `crates/floom` (`run_desktop/run_tui/run_android/run_web`) and every 2.0 example | Partly — see below |

The `docs/gates/*` evidence was collected against the **v1 monolith** or against
the one 2.0 path that is real (`WinitPlatform`). It is not evidence for the
`floom-platform-ios` / `-harmony` / `-web` / `-embedded` crates.

## Per-backend verdict (evidence)

Method: read the source; `cargo check` / `cargo test` each crate; trace every
capability bit to an implementation.

| Backend | Windowing | Clipboard | IME | a11y | File dialog | Verdict |
|---|---|---|---|---|---|---|
| `floom-platform-desktop` (`WinitPlatform`, `--features winit`) | **real** winit | **real (NSPasteboard) on macOS**, `String` elsewhere | **real** (winit `request_ime_update`) | none | none | **near-production on macOS; not elsewhere** |
| `floom-platform-desktop` (`DesktopPlatform`, default) | `String`/`HashMap` stub | `String` stub | `HashMap` stub, no-op cursor | adapter only | none | **test double** |
| `floom-platform-android` | **real** (winit + JNI) | **real** (JNI `ClipboardManager`) | real show/hide; caret inert | stub | none | **real, partial caps** |
| `floom-platform-ios` | fake (`HostPlatform`) | none | fake | stub | none | **stub — no `objc2`, no cfg** |
| `floom-platform-harmony` | fake + a `NATIVE_VIEW` flag | none | fake | stub | none | **stub** |
| `floom-platform-web` | `HostPlatform` + wasm DOM loop | **none** (no `Clipboard` impl) | fake | adapter only | none | **scaffolding** |
| `floom-platform-tui` | **real** (termios/ioctl, SIGWINCH) | `set_text` real (OSC 52); `get_text` `None` | n/a (correct) | n/a | n/a | **real** |
| `floom-platform-embedded` | fake + in-memory framebuffer | none | none | stub | none | **simulation; `drm` is a flag** |
| `floom-platform-mobile` (`packages/*`) | — | clipboard: Android real / iOS stub | Android real / iOS stub | — | — | **3 real pkgs, 8 stub pkgs** |

`floom-platform-mobile` package tally: real = `vibration` (both OS), `clipboard`
(Android), `keyboard` (Android). Host-inert stubs on **both** arms = `device_info`,
`battery`, `connectivity`, `location`, `sensors`, `local_auth`, `notifications`,
`share`, plus `safe_area` (returns `ZERO`) and `system_chrome` (returns `Err`).

## Confirmed defects (not just gaps)

1. **`DesktopPlatform` advertises `CLIPBOARD` + `IME` while backing them with an
   in-process `String` / `HashMap`** (`crates/floom-platform-desktop/src/lib.rs:157,328-343`).
   The real `NSPasteboard` code (`clipboard.rs`) is wired **only** into
   `WinitPlatform` (`winit_backend.rs:672,677`). `HostPlatform` capability-gates
   these bits; `DesktopPlatform` does not — so a capability query lies.
2. **`desktop_default()` claims `OPEN_URL` and `FILE_DIALOG`**
   (`capabilities.rs:33-43`) — neither has any implementation anywhere in the 2.0
   tree (grep: only the capability bits exist).
   **Fixed 2026-10-22:** both bits, and `DRM_KMS` with them, are **deleted**.
   A bit that nothing implements is a promise nobody keeps, and deleting it
   makes claiming one a compile error rather than a lie.
3. **macOS build is currently broken**: `native_appkit.rs:340` calls
   `NSArray::iter`, removed in the pinned `objc2` — `cargo check -p
   floom-platform-desktop` fails with E0599. The `--features winit` path compiles.
4. **The workspace does not currently build.** The in-progress widget migration
   (widgets moving into `crates/ecosystem/floom-ui`) leaves `floom-accuracy`,
   and the examples `floom-demo` / `floom-capabilities` / `floom-counter` with
   unresolved imports. This is unrelated to the platform layer but blocks any
   real "is it production-ready" claim.
5. **`desktop_default()` a11y adapter declares `NATIVE_BRIDGE`** while no
   in-tree code calls any OS accessibility API (macOS `NSAccessibility` grep is
   empty outside the legacy monolith).
   **Fixed 2026-10-22:** the desktop adapter no longer claims the bridge by
   default. The claim is derived from the bridge's presence
   (`with_native_bridge()`), so the two cannot drift. The bridge itself is
   still missing, which is what `docs/gates/macos.md` item 3 records.

## What is genuinely production-grade today

- `WinitPlatform` windowing + IME + macOS clipboard (gated `--features winit`).
- `native_appkit.rs` / `native.rs` (once the `objc2` error above is fixed).
- `floom-platform-tui` end to end.
- `floom-platform-android` windowing + input + clipboard + IME show/hide.
- `floom-renderer-software` (the shadow reference), all five renderers compile.

## Alignment with the project plan

The scaffolding is `[x]` under **Phase 7 (Platform ABI)**; the real work is
**Phase 19 — Desktop/Mobile/Web/Embedded Completion**, whose 8 tasks (19.1–19.8)
are all `[ ]` except the ones proven above. The plan below is Phase 19 broken
into ordered, verifiable steps, front-loaded with the defects that block it.
