# Gate: macOS

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | fail | Not yet driven from outside the process. The window is real and rendering (`examples/floem2-macos`: `GPU adapter: Apple M4 Pro (Metal)`, `widget tree mounted (4 nodes)`, window `480x352` including chrome on screen), but this machine's synthetic input path is blocked — see *Blocked on TCC* below. Clicks driven by hand did reach the tree earlier in the session (`pointer -> button click, counter = 1…30`), which is not the same as a reproducible spot-check. |
| 2 | IME Latin + one CJK | fail | Half observed, not reproduced. Hand-driven typing produced a real composition and its commit — `text field text="" preedit="d" composing=true` … `text field text="丁丁丁丁" preedit="" composing=false` — so `ImePreedit`/`ImeCommit` from the OS reach the focused `TextField` (the same path `docs/gates/android.md` records for Android, through a completely different backend). No Latin half and no scripted run yet. |
| 3 | a11y label + Default action | fail | No OS bridge at all. `framework/floem-accessibility` + `DesktopAccessibilityAdapter` retain a semantics tree in-process, and the adapter advertises `AccessibilityCapabilities::NATIVE_BRIDGE`, but nothing in the floem-2 tree touches `NSAccessibility` (`grep -r NSAccessibility platform/ app/ framework/` is empty; the AppKit-backed bridge that exists is in the **legacy** `src/accessibility/backend_nsaccessibility.rs`). Assistive tech therefore sees the window and nothing inside it. |
| 4 | Clipboard read/write | pass | On macOS `Clipboard` is `NSPasteboard`, not a `String` field (`platform/floem-platform-desktop/src/clipboard.rs`). The example writes `floem2-macos clipboard probe 3e7b` and reads it back (`clipboard write=true read_back=Some("…") matched=true`), and the same string was then read **out of the process entirely** by `drv pb-read`: `pasteboard=floem2-macos clipboard probe 3e7b`. That cross-process read is the part a `String` field cannot fake. Two unit tests cover the round trip, including CJK, on a private pasteboard so the person's own clipboard is not disturbed. |
| 5 | File pick / share-open as applicable | fail | No file-dialog capability exists in the floem-2 platform ABI: `floem-platform` has no `Dialog`/`FilePicker` trait, and `Capability` has no bit for it — the v1 `src/platform/*` bridges were not carried over. Nothing to exercise on any desktop target. |
| 6 | Safe area insets | n/a | Desktop: the window is the whole surface, and nothing between the window frame and the content view shrinks it (the content view is exactly the requested `480x320` inside a `480x352` frame — the 32 is the title bar, which is chrome, not an inset). |
| 7 | Focus + basic keyboard nav | fail | Focus is programmable but not reachable from the keyboard: `floem-focus` implements traversal (`FocusScope`, `traverse(Forward/Backward)`) and the host exposes `focus_next()`/`focus_previous()`, but nothing binds `Tab`/`Shift+Tab` to them (`grep -rn "focus_next\|move_focus"` finds only the definitions and two headless tests). A Tab press is delivered to the focused widget as an ordinary key event. |
| 8 | CI/target compile | pass | `cargo check -p floem-platform-desktop --features winit --target aarch64-apple-darwin`, `cargo check -p floem2-macos --features real --target aarch64-apple-darwin` and `cargo check -p floem2-window --features real --target aarch64-apple-darwin` are all clean, zero warnings. The `--features real` configuration is the one the run above used, and the window it opened is the same binary that was checked. |

## Sign-off

- Spot-check date: 2026-09-24
- Environment (device/simulator): Apple M4 Pro (arm64), macOS 26 (`swift-driver` reports `arm64-apple-macosx26.0`), Metal GPU (vello over wgpu). Window: `examples/floem2-macos`, `--features real`, `480x320` content, `scale=1`.
- Reviewer: automated spot-check assisted by `scripts/macos-gate/` (the driver source that synthesises input and reads OS state). Items 1, 2 and 7 are **not** signed off: the reason is written down rather than left blank, because "it worked when someone tried it" is not a gate result.

## Blocked on TCC (why 1, 2 and 7 are not signed off)

Driving a macOS app from another process needs two grants from the process that
launched the driver (here: `WorkBuddy.app`, the responsible process for this
shell):

- **Accessibility** — synthesise key and mouse events, read the accessibility
  tree. Measured: `ax trusted=false`, and both `CGEvent.post(tap: .cghidEventTap)`
  and `CGEvent.postToPid` are **silently discarded** by the window server: a
  click aimed at the button's own reported rect produced no `pointer -> button
  click` line, and `NSRunningApplication.activate` did not change the frontmost
  process either.
- **Screen Recording** — take screenshots. Granted during this session
  (`CGRequestScreenCaptureAccess()` flipped to `true`), which is how the window
  and the permission dialog were observed.

Accessibility was still `false` when this file was written. Grant it to
`WorkBuddy` in *System Settings → Privacy & Security → Accessibility* (and
restart the app), then re-run:

```sh
bash scripts/macos-gate/build-drv.sh          # builds `drv`, prints the TCC state
cargo run -p floem2-macos --features real     # prints its control geometry
drv activate <pid>; drv click <x> <y>         # -> "pointer -> button click, counter = 1"
drv im-select SCIM.ITABC; drv vkey 45; ...    # -> "text field text=… preedit=… composing=…"
```

The geometry to aim at comes from the app itself (`button logical=(0.0,92.0
160.0x40.0)`) **plus** the window frame from `drv windows` **plus** the 32 pt
title bar: content-relative coordinates are not screen coordinates. That offset
is the macOS version of the trap `docs/gates/android.md` records for the
Android status bar.

## Notes

- Item 4 was a real gap, now closed: the winit backend used to implement
  `Clipboard` as an in-process `String`, which passes every unit test a
  clipboard can be asked to pass while being unusable outside its own process.
  `NSPasteboard` is the fix; the other desktop targets still carry the
  placeholder, and their gate files should say so.
- Item 3's `NATIVE_BRIDGE` claim is worth a follow-up on its own: a capability
  bit that no backend honours is the accessibility version of a gate that
  cannot fail.
- This is the second gate file to be filled in (Android was the first). The
  remaining files in this directory are untouched, which is not a claim that
  they are fine.
