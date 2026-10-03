// Floom → OpenHarmony accessibility — ArkTS side (reference / legacy seam).
//
// IMPORTANT (#113, 2026-08-13): the ORIGINAL design pushed floom's neutral
// accessibility tree to ArkTS and asked ArkTS to register an OHOS
// "AccessibilityProvider". That design is INVALID and has been removed:
//
//   • ArkTS / @kit.AccessibilityKit does NOT expose `registerProvider`,
//     `AccessibilityNodeInfo`, `AccessibilityRole`, `ActionType`,
//     `AccessibilityState`, or `invalidate`. Verified against the SDK
//     ets/api/@ohos.accessibility.d.ts — none of those symbols exist.
//   • An app CANNOT register an accessibility provider from ArkTS at all.
//     OpenHarmony exposes a screen-reader tree to a NATIVE (C) window ONLY
//     through the native C API in
//     native/sysroot/usr/include/arkui/native_interface_accessibility.h:
//       - ArkUI_AccessibilityProvider
//       - ArkUI_AccessibilityElementInfo (SetElementId / SetParentId /
//         SetAccessibilityText / SetAccessibilityDescription / SetScreenRect /
//         SetOperationActions / SetClickable / SetFocusable / …)
//       - ArkUI_Accessibility_ActionType (CLICK=0x10, LONG_CLICK=0x20,
//         GAIN_ACCESSIBILITY_FOCUS=0x40, SCROLL_FORWARD=0x100,
//         SET_TEXT=0x4000, …)
//       - registered via OH_ArkUI_AccessibilityProviderRegisterCallback
//     The correct backend therefore lives in Rust
//     (crates/floom/src/accessibility/backend_ohos.rs) calling that C API directly.
//     See docs/ohos.md "Accessibility bridge" and task #121.
//
// This file is now a TYPE-CORRECT reference module: it keeps the floom-role →
// OHOS component-type string and floom-action → native action-bitmap maps that
// the native-C rework will consume, plus the (no-op) seam functions. It no
// longer references any non-existent API and will type-check against the real
// SDK.

import { onAccessibilityAction } from './FloomBridge';

// ─────────────────────────────────────────────────────────────────────
// Role mapping: floom neutral role name → OHOS component-type string.
//
// The native C API sets the component type via
// OH_ArkUI_AccessibilityElementInfoSetComponentType(element, const char*),
// whose value is a component-type string. floom already serializes role names
// via role_name() in backend_ohos.rs, so this is a straight echo — kept so the
// ArkTS surface stays self-describing and the native bridge can forward `role`
// verbatim.
// ─────────────────────────────────────────────────────────────────────
export function roleToComponentType(role: string): string {
  return role;
}

// ─────────────────────────────────────────────────────────────────────
// Action mapping: floom neutral action name → native C action bitmap.
//
// Mirrors ArkUI_Accessibility_ActionType in native_interface_accessibility.h.
// Only actions with a native equivalent are mapped; the rest fall back to 0
// (the native bridge ignores them).
// ─────────────────────────────────────────────────────────────────────
export function actionToNative(action: string): number {
  switch (action) {
    case 'default':         return 0x10;  // ARKUI_ACCESSIBILITY_NATIVE_ACTION_TYPE_CLICK
    case 'showContextMenu': return 0x20;  // LONG_CLICK
    case 'focus':           return 0x40;  // GAIN_ACCESSIBILITY_FOCUS
    case 'scrollIntoView':  return 0x100; // SCROLL_FORWARD
    case 'setValue':        return 0x4000; // SET_TEXT
    case 'select':          return 0x2000; // SELECT_TEXT
    default:                return 0;      // no native equivalent (increment/
                                          // decrement/expand/collapse/…)
  }
}

// Inverse of actionToNative → floom neutral action name. Used by the native
// bridge's execute-action callback before it calls on_accessibility_action.
export function nativeActionToFloom(actionType: number): string {
  switch (actionType) {
    case 0x10:   return 'default';
    case 0x20:   return 'showContextMenu';
    case 0x40:   return 'focus';
    case 0x100:  return 'scrollIntoView';
    case 0x4000: return 'setValue';
    case 0x2000: return 'select';
    default:     return 'default';
  }
}

// Seam used by a future native bridge or an ArkUI-overlay variant: forward a
// screen-reader action on `nodeId` back into floom's napi export.
export function performFloomAction(nodeId: number, actionType: number): void {
  const name = nativeActionToFloom(actionType);
  onAccessibilityAction(nodeId, name, null);
}

// ─────────────────────────────────────────────────────────────────────
// Provider registration — NO-OP on ArkTS.
//
// ArkTS cannot register an accessibility provider (the API does not exist in
// @kit.AccessibilityKit). The real registration happens in Rust via the native
// C ArkUI_AccessibilityProvider. This function is kept so EntryAbility's call
// site continues to compile; it only documents the correct architecture.
// ─────────────────────────────────────────────────────────────────────
export function registerFloomA11yProvider(): void {
  console.info(
    '[floom] a11y: ArkTS provider registration is a no-op. ' +
    'OHOS exposes a screen-reader tree only through the native C ' +
    'ArkUI_AccessibilityProvider (native_interface_accessibility.h); ' +
    'see docs/ohos.md "Accessibility bridge" and task #121.',
  );
}

// Seam retained for API parity with the data path (FloomBridge.setAccessibilityTree
// calls this after each tree push). The native bridge notifies the C provider
// out-of-band; here it is a no-op.
export function refreshFloomA11y(): void {
  // No-op: the native C provider is refreshed by the Rust backend.
}
