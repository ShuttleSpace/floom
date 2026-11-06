<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Cross-platform a11y + IME on-device integration test (#116)

Companion to `../floem-archive/docs/accessibility.md` (neutral protocol + desktop/mobile backends)
and `docs/ohos.md`. This is the **manual on-device** verification plan for the
screen-reader (a11y) and soft-keyboard (IME) paths on the three mobile targets
after the automated tests pass:

```
cargo test -p floom --lib accessibility      # neutral tree build + action routing (green)
cargo check -p floom                          # host build (green)
cargo check -p floom --target <mobile target> # per-target compile (green where applicable)
```

Automated tests cover the **neutral** layer (tree construction, `AccessAction`
→ `PendingAction` routing, per-view `on_action` callbacks). They do **not**
exercise the OS screen reader or IME — that requires a real device/emulator and
a human/ATF driving VoiceOver / TalkBack / the OHOS screen reader. This document
is that on-device plan.

---

## 0. Shared preconditions

For every target, build an app that exercises each role/action the checklist
touches: a labeled `Button`, a `CheckBox`/`Switch`, a `Slider`/`SpinButton`
(exposes `increment`/`decrement`/`setValue`), a `TextField` (IME + `setValue`),
a `TabList`/`Tab`, a scrollable `List`, and a disclosure (`expand`/`collapse`).
Wire `ViewId::accessibility_*` builders and, for stateful controls,
`ViewId::accessibility_action(handler)` so `Increment`/`Decrement`/`SetValue`/
`Expand`/`Collapse` do something visible.

Pass criteria unless noted: screen reader announces the **role + label +
value/state**, and activating the control produces the **same** effect as a
pointer click (routed through the identical `PendingAction` channel).

---

## 1. iOS — VoiceOver + IME

Backend: `crates/core/floom-accessibility/src` (UIAccessibility element mirror).
IME: `crates/platform/floom-platform/src` is OHOS-only; iOS IME is `crates/platform/floom-platform/src…ios_ime.rs`
(UITextField delegate + `UserEvent::IosIme`).

### 1.1 VoiceOver
- [ ] Enable VoiceOver (Settings → Accessibility → VoiceOver, or triple-click
      side button). Swipe-right through the app; every exposed control is
      reached and announced with correct role + label.
- [ ] **Button**: "Activate" (double-tap) fires the same `AccessAction::Default`
      as a tap. `showContextMenu` (if any) maps to the long-press rotor action.
- [ ] **CheckBox/Switch**: state ("selected"/"checked") announced; toggle via
      VoiceOver custom action or double-tap.
- [ ] **Slider/SpinButton**: VoiceOver "increment"/"decrement" rotor actions
      route to `AccessAction::Increment`/`Decrement` and move the value.
- [ ] **Tab**: switching tabs via VoiceOver reaches the correct `AccessNodeId`.
- [ ] **List scroll**: swipe-up/down scrolls; `ScrollIntoView` reachable.
- [ ] **Focus sync**: moving the on-screen focus (keyboard/tap) updates the
      VoiceOver cursor to the matching element (root `focus` id forwarded).
- [ ] Rotors / "Read all" traverse the whole tree without skipping a control.

### 1.2 IME
- [ ] Focus the `TextField`; on-screen keyboard appears.
- [ ] Typing commits text (`ImeEvent::Commit`) into the field.
- [ ] Preedit/composition (e.g. CJK) updates via `ImeEvent::Preedit`; ending
      composition fires `ImeEvent::Disabled`.
- [ ] Backspace/selection delete routes to `ImeEvent::DeleteSurrounding`.
- [ ] Dismissing the keyboard (outside tap / hide) clears IME state without
      leaving the field stuck "editing".
- [ ] `IME purpose` (password → secure entry, number → numeric keypad,
      email/url/search) is reflected by the keyboard type where the
      `UserEvent`/`UITextField` config forwards it.

---

## 2. Android — TalkBack + IME

Backend: `crates/core/floom-accessibility/src` (`AccessibilityNodeInfo` + JNI
`nativePerformAction`). IME: Android uses the system IME via the winit/Android
text-input path (no special floom bridge beyond the focus signal).

### 2.1 TalkBack
- [ ] Enable TalkBack (Settings → Accessibility → TalkBack). Explore-by-touch
      (drag/swipe) announces each control's role + label + state.
- [ ] **Button**: double-tap activates (`AccessAction::Default`); routing
      identical to tap, verified by the same visual change.
- [ ] **CheckBox/Switch**: state announced; toggle works.
- [ ] **Slider/SpinButton**: "increment"/"decrement" custom actions
      (`ACTION_SET_PROGRESS` / custom) route to `Increment`/`Decrement`.
- [ ] **TextField**: `ACTION_SET_TEXT` (from an a11y "set text" action or a
      braille keyboard) routes to `AccessAction::SetValue` and updates the
      field via the `on_action` callback.
- [ ] **Tab/List**: pagination + scroll actions reach the right node.
- [ ] **Focus sync**: on-screen focus changes move the TalkBack cursor.
- [ ] Docked magnifier / switch access drive the same `PendingAction`s.

### 2.2 IME
- [ ] Focus `TextField` → Gboard/selected IME appears.
- [ ] Composition (CJK) + commit + delete behave as on iOS.
- [ ] `inputType` reflects floom `ImePurpose` (password → dots, number →
      numeric, email/url → corresponding variants).
- [ ] Commit text lands in the focused field; no duplicate/garbled input.

---

## 3. OpenHarmony — screen reader + IME

IME: `crates/platform/floom-platform/src` (`showKeyboard`/`hideKeyboard` +
`updateImeCursorArea`/`updateImeConfig`, #112) — wired and `cargo check`
green, but **best-effort** (depends on a focused ArkTS input / IME extension).
**a11y: BLOCKED** — see §3.1.

### 3.1 Screen reader — ⚠️ BLOCKED (task #121)
- [ ] **NOT YET VERIFIABLE.** The ArkTS `AccessibilityProvider` design
      (original #111) was built on a non-existent API and has been removed in
      #113. The OHOS screen reader for a native (winit) window is only
      reachable via the **native C** `ArkUI_AccessibilityProvider`
      (`native_interface_accessibility.h`), which is not yet wired (#121).
- [ ] Until #121 lands, the only a11y signal on-device is the debug feed:
      `setAccessibilityTree` stores `currentA11yTree` (inspect via `hilog`
      `[floom] a11y tree updated: …`). Use this to confirm the tree shape
      (ids/roles/bounds/actions) is correct even though it is not yet exposed.
- [ ] Checklist to run **after #121**: enable the OHOS screen reader
      (Settings → Accessibility → ScreenReader); swipe/touch-explore announces
      each control; activate routes to `AccessAction::Default`; slider
      increment/decrement, tab switch, list scroll, and focus sync behave as on
      iOS/Android; `ExecuteAccessibilityAction` in the C provider maps to
      `on_accessibility_action` and through the shared `PendingAction` channel.

### 3.2 IME (#112, best-effort)
- [ ] Focus the `TextField`; soft keyboard appears via
      `inputMethod.getInputMethodController().showTextInput()`.
- [ ] Typing commits into the field (`on_ime_commit` → `ImeEvent::Commit`).
- [ ] Preedit/composition + delete route as on iOS/Android.
- [ ] **Caret window**: `updateImeCursorArea(x,y,w,h)` calls
      `updateCursor({left,top,right,bottom})` so the candidate panel avoids the
      caret (verify on-device that the panel does not overlap the input).
- [ ] **Purpose**: `updateImeConfig(purpose)` stores `currentImePurpose`
      (0 Text / 1 Password / 2 Number / 3 Email / 4 Url / 5 Search); confirm
      the IME extension / focused ArkTS input reads it and switches keyboard
      type. On a pure native (winit) window this may require the IME extension
      to own visibility — on-device verification item.
- [ ] Hide keyboard (`hideTextInput`) clears IME state cleanly.

---

## 4. Regression guardrails

- [ ] After any a11y backend change, re-run
      `cargo test -p floom --lib accessibility` — must stay green (15 tests).
- [ ] After any IME change, exercise at least one CJK + one Latin field on each
      target (composition path differs from plain commit).
- [ ] Neutral `AccessAction` names must stay in sync between
      `crates/core/floom-accessibility/src{backend_ohos.rs,translate.rs}` and every platform
      backend's name→action map (the `action_name`/`action_from_name` pair and
      the iOS/Android/native enums).
- [ ] New mobile target? Add its backend to the capability matrix in
      `docs/ohos.md` (or the relevant platform doc) and to this checklist.

## 5. Status summary

| Target | a11y on-device | IME on-device |
|---|---|---|
| iOS | 🟡 manual plan (backend done, needs device run) | 🟡 manual plan |
| Android | 🟡 manual plan (backend done, needs device run) | 🟡 manual plan |
| OHOS | 🔴 blocked (#121 native C provider) | 🟡 best-effort (#112, needs device run) |

Automated neutral-layer tests (tree + action routing) are green on host and
compile-checked per target; the rows above track the OS-level integration that
only a device/emulator can confirm.
