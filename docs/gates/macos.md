# Gate: macOS

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | pass | `examples/floom-macos --features real`, hand-driven on the real window server. Startup: `GPU adapter: Apple M4 Pro (Metal)`, `widget tree mounted (4 nodes)`, window `x:720 y:183 w:480 h:352` on screen (320 pt of content plus a 32 pt title bar). Two clicks on the button produced `focus Some("TextField@2") -> Button@3` — the press hit-tested onto the button, not the field — followed by `pointer -> button click, counter = 1` and `counter = 2`, and the screenshot of the window reads `clicked 2 times`. |
| 2 | IME Latin + one CJK | pass | Both halves in a single run, through the real input method (Pinyin), hand-driven. **Latin**: `text="n"` → `"ni"` → `"nih"` → `"niha"` → `"nihao"`, every line `composing=false`, then backspaced away one character at a time. **CJK**: `preedit="n"` → `"ni"` → `"ni h"` → `"ni ha"` → `"ni hao"` all with `composing=true` (a live composition, correctly *not* the field's text), then the commit — `text="你好" preedit="" composing=false`. That is OS IME → winit `WindowEvent::Ime(Preedit/Commit)` → `PlatformEvent::ImePreedit/ImeCommit` → runner → focused `TextField`: the same seam `docs/gates/android.md` records for Android, reached through a completely different backend. The characters arrive **and are drawn**: the field's preloaded `你好世界` now renders as real Han glyphs (screenshot), where it used to be four tofu boxes — see *CJK tofu* below. |
| 3 | a11y label + Default action | fail | No OS bridge at all. `crates/floom-accessibility` + `DesktopAccessibilityAdapter` retain a semantics tree in-process, and nothing in the floom-2 tree touches `NSAccessibility`: `grep -rn "NSAccessibility\|accessibilityElement\|setAccessibility" platform/ app/ framework/` is empty, and the AppKit-backed bridge that exists is in the **legacy** `crates/floom/src/accessibility/backend_nsaccessibility.rs`. Assistive tech sees the window and nothing inside it. **The false claim is fixed (2026-10-22):** the adapter no longer advertises `AccessibilityCapabilities::NATIVE_BRIDGE` by default — it is derived from a bridge's presence via `with_native_bridge()`, so the claim and the implementation cannot drift. The bridge itself is still missing, which is why this item stays `fail`. |
| 4 | Clipboard read/write | pass | On macOS `Clipboard` is `NSPasteboard`, not a `String` field (`crates/floom-platform-desktop/src/clipboard.rs`). The example writes `floom-macos clipboard probe 3e7b` and reads it back (`clipboard write=true read_back=Some("…") matched=true`), and the same string was then read **out of the process entirely** by the driver: `pasteboard=floom-macos clipboard probe 3e7b`. That cross-process read is the half a `String` field cannot fake. Two unit tests cover the round trip, including a CJK string, on a private pasteboard so the person's own clipboard is not disturbed. |
| 5 | File pick / share-open as applicable | fail | There is no capability to exercise: `floom-platform` carries no dialog/file-picker trait and `Capability` has no bit for one — the v1 `crates/floom/src/platform/*` bridges were not carried over. True on every desktop target, not just this one. |
| 6 | Safe area insets | n/a | Desktop: the window is the surface. Nothing between the window frame and the content view shrinks it — content is exactly the requested `480x320` inside a `480x352` frame, and the 32 pt is the title bar, which is chrome rather than an inset. |
| 7 | Focus + basic keyboard nav | partial | Two of the three halves are now done and machine-checked headlessly; the third (re-drive on the real window) is still blocked on TCC. **Binding**: `Tab` / `Shift+Tab` are consumed in `WidgetHost::dispatch` (`crates/floom-app/src/lib.rs`) and routed to `focus_next()` / `focus_prev()`, instead of being left unhandled — `grep -rn "focus_next"` now finds the binding plus `tab_moves_focus_between_controls` and the new `tab_key_moves_focus_and_flags_keyboard_navigation` test. **Indicator**: `Button` paints a 2px focus ring when `state.focused` (`crates/floom-widgets/src/controls.rs`), and the host flips `keyboard_navigation` on Tab so a keyboard-moved focus can be distinguished from a click. **Verified headlessly**: the new test dispatches a real `KeyEvent::Tab`, asserts focus advances, Shift+Tab walks it back, and `keyboard_navigation()` is flagged. **Not yet re-driven on the device** (would need TCC — see *Blocked on TCC*); the focus trace and ring still have to be confirmed on the real window server. |
| 8 | CI/target compile | pass | `cargo check` clean with zero warnings for `floom-platform-desktop --features winit`, `floom-macos --features real` and `floom-window --features real` on `aarch64-apple-darwin`. The `--features real` configuration is the one the run above used: same crate, same feature set, same target. |

## CJK tofu — found by this spot-check, then fixed

**CJK glyphs were drawn as empty boxes (`.notdef` tofu).** The repro needed no
typing: `examples/floom-macos` preloads `你好世界` into the text field, and the
window screenshot showed four empty boxes where the four Han characters should
be, while every ASCII run in the same window (`clicked 0 times`, `click me`, the
title bar) rendered normally.

**Root cause.** Not the renderer, and not the font collection. A diagnostic
(`floom-text/src/layout.rs`, `FontContext` + `shape("Hello 你好世界")`) showed:

- The collection *does* contain Han-capable faces — `Hiragino Sans GB` among 606
  families (`has HiraginoSansGB = true`).
- parley splits the line into two runs (Latin, Han), so the fallback *is*
  consulted — but both runs came back with the **same** font blob
  (`data.len=2369592`) and the Han run's glyph ids were `[0, 0, 0, 0]`, i.e.
  `.notdef`. Naming `Hiragino Sans GB` explicitly gave a different, larger blob
  (`data.len=23522052`) and real glyph ids `[2886, 1914, 3415, 2232]`.

So the failure was **family selection for Han script**: parley's automatic
per-script fallback resolved the Han run back to the same Latin-covered generic
sans (which has no Han coverage) instead of the available CJK face. (The skip
path in `crates/floom-renderer-vello/src/gpu.rs` — drop a run whose `font.data`
is empty — was already ruled out: it draws *nothing*, whereas boxes were drawn.)

**Fix** (`crates/floom-text/src/layout.rs`): the default `FontStack` is now a
*list* — the generic sans-serif first, then explicit CJK fallback families
(`PingFang SC`, `Hiragino Sans GB`, `Microsoft YaHei`, `Noto Sans CJK SC`, …).
This is the ordinary UI-toolkit approach to CJK: do not rely on OS auto-fallback,
name the faces. Names that are not installed are ignored by the resolver, so one
stack covers macOS / Windows / Linux / Android. A regression test
(`cjk_glyphs_are_not_tofu`) asserts no Han glyph resolves to `.notdef`; it would
have been red before the fix, when the ids were `[0, 0, 0, 0]`.

This was not one of the eight rows (they are fixed by
`docs/superpowers/specs/2026-08-31-floom-industrial-1.0-design.md`, and this was a
rendering defect rather than a macOS integration one) — but it **blocked 1.0 for
any user who types Chinese, Japanese or Korean**, so it is recorded here as a
finding this gate surfaced, now closed.

## Sign-off

- Spot-check date: 2026-09-24
- Environment (device/simulator): Apple M4 Pro (arm64), macOS 26 (`swift-driver`
  reports `arm64-apple-macosx26.0`), Metal GPU (vello over wgpu). Window:
  `examples/floom-macos`, `--features real`, `480x320` content, `scale=1`,
  Pinyin input method (Simplified) alongside ABC.
- Reviewer: spot-check driven **by hand** on the real window server, with
  `scripts/macos-gate/` used for the parts that can run without extra OS
  grants — the clipboard read from another process (item 4) and the target
  compiles (item 8) are machine-checked; items 1 and 2 are recorded from a
  hand-driven run and would have to be re-driven to be reproduced (see
  *Blocked on TCC* for why the driver cannot do it yet). Item 7 is now implemented
  and verified headlessly (Tab binding + focus ring, see its row) but still needs a
  real-window re-drive. Items 3 and 5 are **not** signed off: each reason is
  written down rather than left blank, because "it worked when someone tried it"
  is not a gate result.

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
cargo run -p floom-macos --features real     # prints its control geometry
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
- Item 3's `NATIVE_BRIDGE` claim deserved its own follow-up, and has had it
  (2026-10-22): the adapter no longer claims a bridge it does not have, and
  the claim is derived from the bridge's presence. A capability bit no backend
  honours is the accessibility version of a gate that cannot fail, and it
  cannot fail now.
- Item 7: the *traversal* was always implemented and tested headlessly; the
  *binding* (`Tab` → `focus_next` / `focus_prev`) and the *indicator* (a focus ring
  on `Button`) now exist too. The binding and the ring are covered by
  `tab_key_moves_focus_and_flags_keyboard_navigation`, which dispatches a real
  `KeyEvent::Tab` and asserts focus advances, Shift+Tab walks back, and
  `keyboard_navigation()` flips. The one thing still missing is a re-drive on the
  real window server, which needs the Accessibility (TCC) grant — see *Blocked on
  TCC*.
- The tofu finding came out of this spot-check, not out of a test: the log says
  the characters arrived and only a screenshot said they were not drawn. That is
  the argument for the pixel half of the gate — `floom-snapshot`'s golden frames
  are ASCII, so nothing in CI would have noticed. It was then fixed and now has a
  regression test, but the test only exists *because* the pixel check found it.
- The fix is in shared text code (`floom-text`), so it is not macOS-specific:
  any target whose font collection has a named fallback gets CJK for free. The
  macOS gate is simply where it was found.
- This is the second gate file to be filled in (Android was the first). The
  remaining files in this directory are untouched, which is not a claim that
  they are fine.
