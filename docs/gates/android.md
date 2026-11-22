<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Gate: Android

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/floom-android` (arm64-v8a APK, `--features real`) on the API 34 emulator (`floom2api34`, `-gpu host`): logcat `GPU adapter: Apple M4 Pro (Vulkan)`, `widget tree mounted (3 nodes)`; three `input tap`s → `pointer -> button click, counter = 3` and the label reads `clicked 3 times` in the screenshot. **Re-verified 2026-09-25** on the current example (`examples/android`, package `com.example.floom`, `floom_android_example-arm64-v8a-release.apk` signed by `scripts/build-apk.sh`): launch renders the counter (`+ Value: 0 −`) and the share-receive status line (`../floem-archive/docs/android/screenshots/launch.png`); one `input tap 455 1231` → label reads `Value: 1` (`tap_plus.png`), logcat shows `Displayed … +145ms` and Vulkan layer loading, no fatal lines. |
| 2 | IME Latin + one CJK | pass | Both halves, on device. **Latin**: typing on the soft keyboard produced `text field text="" preedit="gg" composing=true` (a composition, correctly *not* treated as text), then `text field text="gg " preedit="" composing=false` once space committed it. **CJK**: `text field text="你好世界" preedit="" composing=false` — four Han characters through `commitText`, intact (the screenshot shows them rendered in the field). The path is IME → anchor `EditText.Editable` → `BaseInputConnection.getComposingSpanStart/End` → `ImeCommit`/`ImePreedit` → the focused `TextField`; see *IME text* in `../floem-archive/docs/android.md`. |
| 3 | a11y label + Default action | pass | `AccessibilityNodeProvider` half landed 2026-11-22 (`crates/platform/floom-platform-android/src/accessibility.rs` + the scaffold's `FloomA11y.java`). On the API 34 emulator with TalkBack enabled, `uiautomator dump` shows the virtual nodes with device pixels — `text="Count: 0" class="android.widget.TextView" bounds="[0,137][945,179]"`, `text="Increment" class="android.widget.Button" clickable="true" bounds="[0,200][207,266]"` — and three TalkBack activations moved the counter to `Count: 3` (the label lands in the tree only after its first frame; TalkBack intercepts touch, so the path is `ACTION_CLICK` → `Java_dev_floom_android_FloomA11y_activate` → `WidgetHost::activate`). |
| 4 | Clipboard read/write | pass | One-shot probe in the example writes `floom-android clipboard probe 9f3c` and reads it back. The first read is denied (`ClipboardService: Denying clipboard access to com.example.floom, application is not in focus`) because it runs ~30 ms before `Displayed` — so the example re-reads after the first tap: `clipboard read after focus = Some("floom-android clipboard probe 9f3c") (probe survived: true)`. Confirmed independently from *outside* the process: `ClipboardService`'s own suggestion path logs a clip item whose text is the probe string, and the screenshot shows Android's paste-suggestion chip above the keyboard reading `floom-androi…`. Both directions, and the value outlived our process. |
| 5 | File pick / share-open as applicable | device-gated: SAF `open_file`/`save_as` code-complete but result delivery needs `FloomNativeActivity` subclass (see `../floem-archive/docs/android.md`); not exercised on emulator | needs `FloomNativeActivity` + Gradle/AGP integration |
| 6 | Safe area insets | pass (one-arg reading) | Safe-area applied through `WidgetHost::set_content_insets` 2026-11-22: the scaffold's content starts below the status bar (`Count: 0` at y=137 device px) and a tap on the button centre lands (`Count: 1`). Two readings are both correct and indistinguishable from the screenshot alone: a fullscreen window reports non-zero `systemBars()` insets, a non-fullscreen one has the space already yielded by the system and reports zero — the app pads to whichever it gets. |
| 7 | Focus + basic keyboard nav | pass | Verified on device 2026-11-22 (API 34 emulator). `adb shell input keyevent 61` reaches the app — logcat `D floom : floom_platform_android::android: key KeyCode(3) pressed=true` — and the host-built ring follows the traversal: after the first Tab the ring's colour (26,115,232 ±25) fills 8,931 pixels in the label band (device y 120–175) and none in the button band; after the second Tab it is 0 there and 1,106 in the button band, i.e. focus moved from `Count: 0` to `Increment`. The ring itself is `WidgetHost::paint_focus_ring` (`crates/app/floom-app/src/lib.rs`, `FocusRing::DEFAULT`, four filled rects so every backend can draw it); `set_focus`/`move_focus` now mark the frame dirty, which is why focus moves at all. `AccessibilityNodeInfo.isFocused` is still not mapped from `SemanticsState::FOCUSED`, so the a11y side of focus stays open. The crash fixed 2026-09-25 stays fixed: 0 `Fatal signal`/`No pending exception` lines in the run. |
| 8 | CI/target compile | pass | `cargo check --package=floom-platform-android --target aarch64-linux-android` clean; the same `--features real` configuration produced the `.so` inside the APK item 1 launched from. Packaged by `scripts/build-apk.sh` (aapt2 → zipalign → apksigner), no Gradle/AGP. |

## Sign-off

- Spot-check date: 2026-11-22; items 3, 6 and 7 re-verified on the API 34 emulator (a11y provider, safe area, focus ring); item 7's crash had been fixed 2026-09-25
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
