// Type surface for `libentry.so`, the HAP's single N-API module.
//
// Two halves share this module:
//   - the C++ shell (entry/src/main/cpp/napi_init.cpp), and
//   - the Rust bridge, compiled into the cdylib `libohos.so` and merged in by
//     napi_init.cpp forwarding to its `napi_register_module_v1`
//     (see ets/FloomBridge.ets and crates/floom/src/platform/ohos_bridge.rs).
//
// napi-rs renames every Rust `snake_case` export to a `camelCase` JS name, so the
// Rust `register_platform_bridge` / `on_file_picked` / `on_shared_files` /
// `on_accessibility_action` / `on_ime_*` appear below — and on the module object
// ArkTS imports — as `registerPlatformBridge` / `onFilePicked` / `onSharedFiles` /
// `onAccessibilityAction` / `onImeCommit` / `onImePreedit` / `onImeCompositionEnd`
// / `onImeDelete`. The originating Rust path is named per member.
declare const bridge: {
  // ---- C++ shell (napi_init.cpp) ---------------------------------------
  getMessage(): string;
  incrementCounter(): number;
  setScaleFactor(scaleFactor: number): void;
  setFontScale(fontScale: number): void;

  // ---- Rust bridge (libohos.so, forwarded) -----------------------------
  // crates/floom/src/platform/ohos_bridge.rs — ArkTS hands Rust one object wrapping every
  // capability (getText/setText/openUrl/openFile/saveFile/setAccessibilityTree/
  // showKeyboard/hideKeyboard + IME caret/purpose), which Rust keeps a
  // reference to and calls back into on demand.
  registerPlatformBridge(bridge: object): void;

  // crates/floom/src/platform/ohos_file.rs — ArkTS answers an openFile/saveFile request.
  onFilePicked(requestId: number, uris: Array<string> | null): void;

  // crates/floom/src/platform/ohos_receive.rs — ArkTS forwards a share / "open with" intent.
  onSharedFiles(
    uris: Array<string>,
    mimeTypes: Array<string | null> | null,
    displayNames: Array<string | null> | null
  ): void;

  // crates/floom/src/accessibility/backend_ohos.rs — a screen-reader action on a floom node.
  onAccessibilityAction(nodeId: number, action: string, payload: string | null): void;

  // crates/floom/src/platform/ohos_ime.rs — Input Method Extension → floom IME queue.
  onImeCommit(text: string): void;
  onImePreedit(text: string, cursorStart: number, cursorEnd: number): void;
  onImeCompositionEnd(): void;
  onImeDelete(before: number, after: number): void;
};

export default bridge;