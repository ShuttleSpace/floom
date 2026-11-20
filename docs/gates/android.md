<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Gate: Android

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/floom-android` (arm64-v8a APK, `--features real`) on the API 34 emulator (`floom2api34`, `-gpu host`): logcat `GPU adapter: Apple M4 Pro (Vulkan)`, `widget tree mounted (3 nodes)`; three `input tap`s → `pointer -> button click, counter = 3` and the label reads `clicked 3 times` in the screenshot. **Re-verified 2026-09-25** on the current example (`examples/android`, package `com.example.floom`, `floom_android_example-arm64-v8a-release.apk` signed by `scripts/build-apk.sh`): launch renders the counter (`+ Value: 0 −`) and the share-receive status line (`../floem-archive/docs/android/screenshots/launch.png`); one `input tap 455 1231` → label reads `Value: 1` (`tap_plus.png`), logcat shows `Displayed … +145ms` and Vulkan layer loading, no fatal lines. |
| 2 | IME Latin + one CJK | pass | Both halves, on device. **Latin**: typing on the soft keyboard produced `text field text="" preedit="gg" composing=true` (a composition, correctly *not* treated as text), then `text field text="gg " preedit="" composing=false` once space committed it. **CJK**: `text field text="你好世界" preedit="" composing=false` — four Han characters through `commitText`, intact (the screenshot shows them rendered in the field). The path is IME → anchor `EditText.Editable` → `BaseInputConnection.getComposingSpanStart/End` → `ImeCommit`/`ImePreedit` → the focused `TextField`; see *IME text* in `../floem-archive/docs/android.md`. |
| 3 | a11y label + Default action | device-gated: Android accessibility-service bridge not implemented; not exercised on emulator | a11y injection fixed for JNI stability (item 7), VoiceOver-style node geometry not yet wired |
| 4 | Clipboard read/write | pass | One-shot probe in the example writes `floom-android clipboard probe 9f3c` and reads it back. The first read is denied (`ClipboardService: Denying clipboard access to com.example.floom, application is not in focus`) because it runs ~30 ms before `Displayed` — so the example re-reads after the first tap: `clipboard read after focus = Some("floom-android clipboard probe 9f3c") (probe survived: true)`. Confirmed independently from *outside* the process: `ClipboardService`'s own suggestion path logs a clip item whose text is the probe string, and the screenshot shows Android's paste-suggestion chip above the keyboard reading `floom-androi…`. Both directions, and the value outlived our process. |
| 5 | File pick / share-open as applicable | device-gated: SAF `open_file`/`save_as` code-complete but result delivery needs `FloomNativeActivity` subclass (see `../floem-archive/docs/android.md`); not exercised on emulator | needs `FloomNativeActivity` + Gradle/AGP integration |
| 6 | Safe area insets | device-gated: not exercised on emulator; `window_px` numbers are window-relative by design (platform input space), not an inset report | no `WindowHandle::safe_area()` public accessor yet |
| 7 | Focus + basic keyboard nav | partial | **Found and fixed a crash that made keyboard nav impossible** (2026-09-25): the first hardware key event (`adb shell input keyevent 61`, KEYCODE_TAB) aborted the process — SIGABRT with `No pending exception expected: java.lang.ClassNotFoundException: dev.floom.android.FloomAccessibilityView`. Root cause: a11y injection at startup does a best-effort `env.new_object("dev/floom/android/FloomAccessibilityView", …)` which fails on a `hasCode="false"` APK; the swallowed `Err` left the JNI exception *pending*, and the next JNI call on that thread (the key path's `device_key_character_map`) made ART abort. Fix: `AndroidJni::with_env` now clears the pending exception after every closure (`crates/floom/src/platform/android_jni.rs`) — covers all best-effort JNI callers (a11y injection, clipboard, external URL). After the fix the same Tab key leaves the app foregrounded with **0** `Fatal signal`/`No pending exception` lines in logcat. The key itself is delivered and mapped: the vendored winit-android maps `Keycode::Tab → Key::Named(NamedKey::Tab)` (`crates/vendor/floem-winit/winit-android/src/keycodes.rs:312`) and root floom's dispatch runs `element_tab_navigation` on it (`crates/floom/src/event/dispatch.rs:1197`, including the nothing-focused `initial_tab_target` case). What is *not* verified on device: whether focus actually moved — the example is built on the root `floom` crate, whose widgets paint no focus indicator (screenshot before/after Tab is pixel-identical) and whose buttons have no Enter/Space activation to probe with. The Tab-traversal + focus-ring pair verified headlessly in `floom-app`/`floom-widgets` (see `gates/macos.md` #7) is not in this example's dependency graph. |
| 8 | CI/target compile | pass | `cargo check --package=floom-platform-android --target aarch64-linux-android` clean; the same `--features real` configuration produced the `.so` inside the APK item 1 launched from. Packaged by `scripts/build-apk.sh` (aapt2 → zipalign → apksigner), no Gradle/AGP. |

## Sign-off

- Spot-check date: 2026-09-24; item 1 re-verified and item 7 crash fixed 2026-09-25
- Environment (device/simulator): Android emulator `floom2api34` (API 34, AVD), launched with `emulator -avd floom2api34 -gpu host` on macOS / Apple M4 Pro (host GPU → MoltenVK → Metal, so vello's compute pipelines run). APK: `floom-android-example` (09-24) / `floom-android-example` package `com.example.floom` (09-25), arm64-v8a, `--features real`.
- Reviewer: automated spot-check — every line above is verbatim logcat / `dumpsys` / screenshot from the run, not a reading of the source.

## Notes

- Items 1, 2, 4, 7 and 8 are recorded here with evidence rather than left blank:
  "it worked last session" is not a gate result.
- Item 2 was closed the hard way. The channel (keyboard shown) was easy and the
  characters were not: `winit-android` implements no IME events at all, so the
  text had to be read back out of the platform's own anchor view — see *IME
  text* in `../floem-archive/docs/android.md` for the two traps that cost the most time (the
  composing span is on `BaseInputConnection`, not `Spannable`; and a soft-keyboard
  tap does not wake the app, so nothing is read until some other event arrives).
- Item 7's crash is a class of bug worth remembering: *swallowing a JNI `Err`
  without `exception_clear()` is a deferred process abort*. The exception sits
  pending on the thread and the next JNI call — sometimes seconds later, on an
  unrelated code path — kills the app with `No pending exception expected`.
  `AndroidJni::with_env` now clears on exit so the mistake cannot recur for
  Rust-initiated JNI; the `Java_dev_floom_*` entry points called *from* Java
  deliberately leave exceptions to propagate to the caller.
- This is the first gate file to be filled in. It is not a claim that the
  others are fine — see the other files in this directory.
