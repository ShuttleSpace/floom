# Gate: macOS

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/floem2-macos --features real`, hand-driven on the real window server. Startup: `GPU adapter: Apple M4 Pro (Metal)`, `widget tree mounted (4 nodes)`, window `x:720 y:183 w:480 h:352` on screen (320 pt of content plus a 32 pt title bar). Two clicks on the button produced `focus Some("TextField@2") -> Button@3` — the press hit-tested onto the button, not the field — followed by `pointer -> button click, counter = 1` and `counter = 2`, and the screenshot of the window reads `clicked 2 times`. |
| 2 | IME Latin + one CJK | pass | Both halves in a single run, through the real input method (Pinyin), hand-driven. **Latin**: `text="n"` → `"ni"` → `"nih"` → `"niha"` → `"nihao"`, every line `composing=false`, then backspaced away one character at a time. **CJK**: `preedit="n"` → `"ni"` → `"ni h"` → `"ni ha"` → `"ni hao"` all with `composing=true` (a live composition, correctly *not* the field's text), then the commit — `text="你好" preedit="" composing=false`. That is OS IME → winit `WindowEvent::Ime(Preedit/Commit)` → `PlatformEvent::ImePreedit/ImeCommit` → runner → focused `TextField`: the same seam `docs/gates/android.md` records for Android, reached through a completely different backend. **The characters arrived; they are not drawn — see the open finding below.** |
| 3 | a11y label + Default action | fail | No OS bridge at all. `framework/floem-accessibility` + `DesktopAccessibilityAdapter` retain a semantics tree in-process, and the adapter advertises `AccessibilityCapabilities::NATIVE_BRIDGE`, but nothing in the floem-2 tree touches `NSAccessibility`: `grep -rn "NSAccessibility\|accessibilityElement\|setAccessibility" platform/ app/ framework/` is empty, and the AppKit-backed bridge that exists is in the **legacy** `src/accessibility/backend_nsaccessibility.rs`. Assistive tech sees the window and nothing inside it. |
| 4 | Clipboard read/write | pass | On macOS `Clipboard` is `NSPasteboard`, not a `String` field (`platform/floem-platform-desktop/src/clipboard.rs`). The example writes `floem2-macos clipboard probe 3e7b` and reads it back (`clipboard write=true read_back=Some("…") matched=true`), and the same string was then read **out of the process entirely** by the driver: `pasteboard=floem2-macos clipboard probe 3e7b`. That cross-process read is the half a `String` field cannot fake. Two unit tests cover the round trip, including a CJK string, on a private pasteboard so the person's own clipboard is not disturbed. |
| 5 | File pick / share-open as applicable | fail | There is no capability to exercise: `floem-platform` carries no dialog/file-picker trait and `Capability` has no bit for one — the v1 `src/platform/*` bridges were not carried over. True on every desktop target, not just this one. |
| 6 | Safe area insets | n/a | Desktop: the window is the surface. Nothing between the window frame and the content view shrinks it — content is exactly the requested `480x320` inside a `480x352` frame, and the 32 pt is the title bar, which is chrome rather than an inset. |
| 7 | Focus + basic keyboard nav | fail | Focus exists and moves — but only under the pointer. Clicking the button moved it (`TextField@2 -> Button@3`), clicking the field moved it back, and the startup probe's `set_focus` shows as `focus None -> TextField@2`. A **Tab press moved nothing**: the focus trace prints every change and had no line for it. Traversal is implemented (`floem-focus::FocusScope::traverse(Forward/Backward)`, `host.focus_next()`/`focus_previous()`), but nothing binds `Tab`/`Shift+Tab` to it — `grep -rn "focus_next\|move_focus"` finds only the definitions and two headless tests. Related, reported by the person driving: the focused button shows no focus indicator, so even a wired-up traversal would be invisible. |
| 8 | CI/target compile | pass | `cargo check` clean with zero warnings for `floem-platform-desktop --features winit`, `floem2-macos --features real` and `floem2-window --features real` on `aarch64-apple-darwin`. The `--features real` configuration is the one the run above used: same crate, same feature set, same target. |

## Open finding — blocks sign-off, and is not one of the eight rows

**CJK glyphs are not drawn: they render as empty boxes (tofu).**

The text pipeline hands the renderer the right characters (`text field
text="你好世界"` is in the log) and the renderer has nothing to draw them with.
The repro needs no typing: `examples/floem2-macos` preloads that string into the
text field, and the window screenshot shows four empty boxes where the four Han
characters should be — while every ASCII run in the same window (`clicked 0
times`, `click me`, the title bar) renders normally.

What that already rules out, and what it leaves:

- The skip path in `render/floem-renderer-vello/src/gpu.rs` drops a run whose
  `font.data` is empty **silently — nothing at all is drawn**. Boxes were drawn,
  so a font *was* selected and it produced glyphs: either `.notdef`, or a font
  whose Han code points are boxes.
- So the failure is at family selection / fallback for Han script, not at
  upload. The candidates are a font collection that has no CJK-capable family
  when the run is shaped (`floem-text/src/layout.rs` uses `FontContext::new()`,
  which parley documents as *discovering system fonts if available*), or a
  fallback that finds one but maps these code points to `.notdef`.

It cannot be recorded as a `fail` row because the eight rows are fixed by
`docs/superpowers/specs/2026-08-31-floem-industrial-1.0-design.md`, and it is a
rendering defect rather than a macOS integration one — but **it blocks 1.0 for
any user who types Chinese, Japanese or Korean**: the characters they typed are
in the field and they cannot read them.

## Sign-off

- Spot-check date: 2026-09-24
- Environment (device/simulator): Apple M4 Pro (arm64), macOS 26 (`swift-driver`
  reports `arm64-apple-macosx26.0`), Metal GPU (vello over wgpu). Window:
  `examples/floem2-macos`, `--features real`, `480x320` content, `scale=1`,
  Pinyin input method (Simplified) alongside ABC.
- Reviewer: spot-check driven **by hand** on the real window server, with
  `scripts/macos-gate/` used for the parts that can run without extra OS
  grants — the clipboard read from another process (item 4) and the target
  compiles (item 8) are machine-checked; items 1, 2 and 7 are recorded from a
  hand-driven run and would have to be re-driven to be reproduced (see
  *Blocked on TCC* for why the driver cannot do it yet). Items 3, 5 and 7 are
  **not** signed off: each reason is written down rather than left blank,
  because "it worked when someone tried it" is not a gate result.

## Blocked on TCC

Driving a macOS app from another process needs **Accessibility** granted to the
process that launches the driver — here `WorkBuddy.app`, the responsible process
for this shell. Measured: `AXIsProcessTrustedWithOptions` is `false`, and both
synthetic-input paths are **silently discarded** — `CGEvent.post(tap:
.cghidEventTap)` and `CGEvent.postToPid` alike. A click aimed at the button's
own reported rect produced no `pointer -> button click` line, and
`NSRunningApplication.activate` did not change the frontmost process either. It
is worth writing down that the failure is *silent*: nothing errors, nothing
logs, the window simply does not react, so a spot-check that does not check the
TCC state would read "the app ignores clicks".

**Screen Recording** was granted during this session (`CGRequestScreenCaptureAccess()`
flipped to `true`), which is how the window, the permission dialog and the tofu
repro were captured — `screencapture -x -o -l <window id>` grabs one window
without bringing it to the front, which matters on a machine where the app runs
behind an editor.

Once Accessibility is granted (and the app restarted):

```sh
bash scripts/macos-gate/build-drv.sh          # builds `drv`, prints the TCC state
cargo run -p floem2-macos --features real     # prints its control geometry
drv activate <pid>; drv click <x> <y>         # -> "pointer -> button click, counter = 1"
drv vkey 45; drv vkey 34; ...                 # -> "text field text=… preedit=…"
drv vkey 48                                   # Tab: the focus trace says whether it moved
```

The geometry to aim at comes from the app (`button logical=(0.0,92.0 160.0x40.0)`)
**plus** the window frame from `drv windows` **plus** the 32 pt title bar:
content-relative coordinates are not screen coordinates. That offset is the
macOS version of the trap `docs/gates/android.md` records for the Android status
bar.

## Notes

- Item 4 was a real gap, now closed: the winit backend used to implement
  `Clipboard` as an in-process `String`, which passes every unit test a
  clipboard can be asked to pass while being unusable outside its own process.
  `NSPasteboard` is the fix; the other desktop targets still carry the
  placeholder, and their gate files should say so.
- Item 3's `NATIVE_BRIDGE` claim deserves its own follow-up: a capability bit
  no backend honours is the accessibility version of a gate that cannot fail.
- Item 7 has two halves and they are worth separating: the *traversal* is
  implemented and tested headlessly, the *binding* does not exist, and the
  *indicator* does not exist either. Only the first is covered by a test today.
- The tofu finding came out of this spot-check, not out of a test: the log says
  the characters arrived and only a screenshot says they were not drawn. That is
  the argument for the pixel half of the gate — `floem-snapshot`'s golden frames
  are ASCII, so nothing in CI would have noticed.
- This is the second gate file to be filled in (Android was the first). The
  remaining files in this directory are untouched, which is not a claim that
  they are fine.
