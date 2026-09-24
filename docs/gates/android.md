# Gate: Android

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/floem2-android` (arm64-v8a APK, `--features real`) on the API 34 emulator (`floem2api34`, `-gpu host`): logcat `GPU adapter: Apple M4 Pro (Vulkan)`, `widget tree mounted (3 nodes)`; three `input tap`s → `pointer -> button click, counter = 3` and the label reads `clicked 3 times` in the screenshot. |
| 2 | IME Latin + one CJK | pass | Both halves, on device. **Latin**: typing on the soft keyboard produced `text field text="" preedit="gg" composing=true` (a composition, correctly *not* treated as text), then `text field text="gg " preedit="" composing=false` once space committed it. **CJK**: `text field text="你好世界" preedit="" composing=false` — four Han characters through `commitText`, intact (the screenshot shows them rendered in the field). The path is IME → anchor `EditText.Editable` → `BaseInputConnection.getComposingSpanStart/End` → `ImeCommit`/`ImePreedit` → the focused `TextField`; see *IME text* in `docs/android.md`. |
| 3 | a11y label + Default action | fail | Not exercised on device. |
| 4 | Clipboard read/write | pass | One-shot probe in the example writes `floem2-android clipboard probe 9f3c` and reads it back. The first read is denied (`ClipboardService: Denying clipboard access to com.example.floem2, application is not in focus`) because it runs ~30 ms before `Displayed` — so the example re-reads after the first tap: `clipboard read after focus = Some("floem2-android clipboard probe 9f3c") (probe survived: true)`. Confirmed independently from *outside* the process: `ClipboardService`'s own suggestion path logs a clip item whose text is the probe string, and the screenshot shows Android's paste-suggestion chip above the keyboard reading `floem2-androi…`. Both directions, and the value outlived our process. |
| 5 | File pick / share-open as applicable | fail | SAF `open_file`/`save_as` are code-complete but result delivery needs the optional `FloemNativeActivity` subclass (see `docs/android.md`); not exercised. |
| 6 | Safe area insets | fail | Not exercised. The example's `window_px` numbers are window-relative by design (that is the platform's input space), not an inset report. |
| 7 | Focus + basic keyboard nav | fail | Not exercised. |
| 8 | CI/target compile | pass | `cargo check --package=floem-platform-mobile --package=floem-platform-android --target aarch64-linux-android` clean; the same `--features real` configuration produced the `.so` inside the APK item 1 launched from. Packaged by `scripts/build-apk.sh` (aapt2 → zipalign → apksigner), no Gradle/AGP. |

## Sign-off

- Spot-check date: 2026-09-24
- Environment (device/simulator): Android emulator `floem2api34` (API 34, AVD), launched with `emulator -avd floem2api34 -gpu host` on macOS / Apple M4 Pro (host GPU → MoltenVK → Metal, so vello's compute pipelines run). APK: `floem2-android-example`, arm64-v8a, `--features real`.
- Reviewer: automated spot-check — every line above is verbatim logcat / `dumpsys` / screenshot from the run, not a reading of the source.

## Notes

- Items 1, 2, 4 and 8 are recorded here with evidence rather than left blank:
  "it worked last session" is not a gate result.
- Item 2 was closed the hard way. The channel (keyboard shown) was easy and the
  characters were not: `winit-android` implements no IME events at all, so the
  text had to be read back out of the platform's own anchor view — see *IME
  text* in `docs/android.md` for the two traps that cost the most time (the
  composing span is on `BaseInputConnection`, not `Spannable`; and a soft-keyboard
  tap does not wake the app, so nothing is read until some other event arrives).
- This is the first gate file to be filled in. It is not a claim that the
  others are fine — see the other files in this directory.
