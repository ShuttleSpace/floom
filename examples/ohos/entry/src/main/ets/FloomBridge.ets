// Floom ↔ OpenHarmony platform bridge (ArkTS application-shell side).
//
// Counterpart to floom's Rust N-API bridges:
//   crates/floom/src/platform/ohos_bridge.rs      -> register_platform_bridge (this file)
//   crates/floom/src/platform/clipboard/ohos_clipboard.rs -> getText / setText
//   crates/floom/src/platform/ohos_external.rs    -> openUrl
//   crates/floom/src/platform/ohos_file.rs        -> openFile / saveFile + on_file_picked
//   crates/floom/src/platform/ohos_receive.rs     -> on_shared_files
//   crates/floom/src/accessibility/backend_ohos.rs-> setAccessibilityTree + on_accessibility_action
//   crates/floom/src/platform/ohos_ime.rs         -> showKeyboard / hideKeyboard +
//                                         updateImeCursorArea / updateImeConfig + on_ime_*
//
// Lifecycle (see EntryAbility.ets):
//   1. EntryAbility.onWindowStageCreate -> setAppContext(this.context)
//   2. then call registerPlatformBridge() once.
//   3. floom stores a reference to the object below and calls its methods
//      (getText/setText/openUrl/openFile/saveFile/setAccessibilityTree/
//       showKeyboard/hideKeyboard) through N-API as needed.
//   4. ArkTS calls BACK into the napi exports (on_file_picked / on_shared_files
//      / on_accessibility_action / on_ime_*) when the OS delivers a result.
//
// NOTE: the floom `#[napi]` surface is compiled into the Rust cdylib
// `libohos.so`, but napi-rs 2.x never calls `napi_module_register` on OHOS, so
// that .so is not an OHOS N-API module and `import … from 'libohos.so'` resolves
// to the wrong module (every export comes back `undefined`). `libentry.so` — the
// HAP's one registered N-API module — forwards to libohos.so's
// `napi_register_module_v1` (see entry/src/main/cpp/napi_init.cpp), so the bridge
// is reached through 'libentry.so'.
//
// NOTE: napi-rs renames every Rust `snake_case` export to a `camelCase` JS name,
// so the Rust `register_platform_bridge` / `on_file_picked` / `on_shared_files` /
// `on_accessibility_action` / `on_ime_*` are reached here as
// `registerPlatformBridge` / `onFilePicked` / `onSharedFiles` /
// `onAccessibilityAction` / `onImeCommit` / `onImePreedit` /
// `onImeCompositionEnd` / `onImeDelete`. The comments below still quote the Rust
// snake_case names because those are what the `crates/floom/src/platform/*.rs` files define.

import { pasteboard } from '@kit.BasicServicesKit';
import { picker } from '@kit.CoreFileKit';
import { Want, common } from '@kit.AbilityKit';
import { inputMethod } from '@kit.IMEKit';

// The default export is the module's N-API exports object — the same one the
// `import bridge from 'libentry.so'` in EntryAbility.ets sees.
import floom from 'libentry.so';

// ArkTS a11y reference seam (see FloomA11y.ets — the real OHOS a11y provider is
// registered from Rust via the native C ArkUI_AccessibilityProvider, NOT here).
import { refreshFloomA11y, registerFloomA11yProvider } from './FloomA11y';

// Re-export so EntryAbility can register the provider at startup.
export { registerFloomA11yProvider };

// The UIAbility context, captured at startup so openUrl can startAbility.
let appContext: common.UIAbilityContext | undefined = undefined;

export function setAppContext(ctx: common.UIAbilityContext): void {
  appContext = ctx;
}

// Shape of the options object Rust (`ohos_file.rs::build_options_object`)
// sends for openFile / saveFile.
interface FloomFileOptions {
  title?: string;
  defaultName?: string;
  selectDirectories: boolean;
  multiSelection: boolean;
  startingDirectory?: string;
}

export function registerPlatformBridge(): void {
  floom.registerPlatformBridge({
    // ---- Clipboard (crates/floom/src/platform/clipboard/ohos_clipboard.rs) ----------
    getText(): string | null {
      try {
        const system = pasteboard.getSystemPasteboard();
        // getData() resolves with a PasteboardData; getPrimaryText() yields ''.
        const data = system.getDataSync();
        const text: string = data.getPrimaryText();
        return text.length > 0 ? text : null;
      } catch (e) {
        console.warn('[floom] getText failed', e);
        return null;
      }
    },

    setText(text: string): void {
      try {
        const system = pasteboard.getSystemPasteboard();
        const data = pasteboard.createData(pasteboard.MIMETYPE_TEXT_PLAIN, text);
        system.setData(data);
      } catch (e) {
        console.warn('[floom] setText failed', e);
      }
    },

    // ---- External URL (crates/floom/src/platform/ohos_external.rs) ------------------
    openUrl(url: string): void {
      const want: Want = {
        // API 26's `wantConstant.Action` enum only carries ACTION_SEND_TO_DATA;
        // the view-data action is still spelled as its raw string.
        action: 'ohos.want.action.viewData',
        uri: url,
      };
      appContext?.startAbility(want).catch((e: Error) => {
        console.warn('[floom] openUrl failed', e);
      });
    },

    // ---- File open / save (crates/floom/src/platform/ohos_file.rs) ------------------
    // NOTE: Rust calls these with (requestId, options). Resolve the OS picker
    // asynchronously and hand the URI(s) back via floom.on_file_picked.
    openFile(requestId: number, options: object): void {
      const opts = options as FloomFileOptions;
      const resolve = (uris: Array<string>): void => {
        floom.onFilePicked(requestId, uris.length > 0 ? uris : null);
      };
      const fail = (): void => {
        floom.onFilePicked(requestId, null);
      };

      if (opts.selectDirectories) {
        const p = new picker.DocumentViewPicker();
        p.select({ selectMode: picker.DocumentSelectMode.FOLDER })
          .then(resolve)
          .catch(fail);
        return;
      }

      const p = new picker.DocumentViewPicker();
      const selectOpts: Record<string, Object> = {};
      if (!opts.multiSelection) {
        selectOpts.maxSelectNumber = 1;
      }
      if (opts.startingDirectory) {
        selectOpts.defaultFilePathUri = opts.startingDirectory;
      }
      p.select(selectOpts as picker.DocumentSelectOptions).then(resolve).catch(fail);
    },

    saveFile(requestId: number, options: object): void {
      const opts = options as FloomFileOptions;
      const p = new picker.DocumentViewPicker();
      const saveOpts: Record<string, Object> = {};
      if (opts.defaultName) {
        saveOpts.newFileNames = [opts.defaultName];
      }
      const resolve = (uris: Array<string>): void => {
        floom.onFilePicked(requestId, uris.length > 0 ? uris : null);
      };
      const fail = (): void => {
        floom.onFilePicked(requestId, null);
      };
      p.save(saveOpts as picker.DocumentSaveOptions).then(resolve).catch(fail);
    },

    // ---- Accessibility (crates/floom/src/accessibility/backend_ohos.rs) -------------
    // `tree` is floom's serialized neutral a11y tree:
    //   { root: number|null, focus: number|null,
    //     nodes: [{ id, role, label, value, description, states,
    //               actions: string[], bounds: {x0,y0,x1,y1}|null,
    //               children: number[] }] }
    // We store it as a debug feed. The REAL screen-reader exposure happens in
    // Rust via the native C ArkUI_AccessibilityProvider (see task #121); this
    // push is retained so the tree is inspectable on-device and can back a
    // future ArkUI-overlay variant if an ArkUI page is added.
    setAccessibilityTree(tree: object): void {
      currentA11yTree = tree;
      console.info('[floom] a11y tree updated:', JSON.stringify(tree));
      // Seam only (no-op on ArkTS — the native C provider is refreshed by the
      // Rust backend). Kept for API parity with the data path.
      refreshFloomA11y();
    },

    // ---- IME: show / hide soft keyboard (crates/floom/src/platform/ohos_ime.rs) -----
    // OHOS shows the keyboard via the InputMethodController. For a native
    // (winit) window with no focused ArkTS input, this is best-effort: it
    // works when a focused input exists (e.g. an ArkTS text field bridging the
    // floom text_input), otherwise the IME extension must drive visibility.
    showKeyboard(): void {
      try {
        const controller = inputMethod.getInputMethodController();
        controller.showTextInput().catch((e: Error) => {
          console.warn('[floom] showTextInput failed', e);
        });
      } catch (e) {
        console.warn('[floom] showKeyboard failed', e);
      }
    },

    hideKeyboard(): void {
      try {
        const controller = inputMethod.getInputMethodController();
        controller.hideTextInput().catch((e: Error) => {
          console.warn('[floom] hideTextInput failed', e);
        });
      } catch (e) {
        console.warn('[floom] hideKeyboard failed', e);
      }
    },

    // ---- IME: caret rect (crates/floom/src/platform/ohos_ime.rs::set_ime_cursor_area) --
    // `updateImeCursorArea` tells the IME where the text caret is (logical px)
    // so its candidate window can avoid overlapping it. Best-effort: works
    // when a focused input exists (e.g. an ArkTS field bridging the floom
    // text_input); on a native (winit) window the IME extension owns the
    // panel and reads this via its own caret query.
    updateImeCursorArea(x: number, y: number, w: number, h: number): void {
      try {
        const controller = inputMethod.getInputMethodController();
        controller.updateCursor({ left: x, top: y, width: w, height: h });
      } catch (e) {
        console.warn('[floom] updateImeCursorArea failed', e);
      }
    },

    // ---- IME: semantic purpose (crates/floom/src/platform/ohos_ime.rs::set_ime_purpose) -
    // `updateImeConfig` forwards the input purpose so the IME can adapt
    // (secure entry, numeric keypad, …). Encoded as an `i64` discriminant:
    //   0 Text, 1 Password, 2 Number, 3 Email, 4 Url, 5 Search
    // Stored for the IME extension / a focused ArkTS input to consume;
    // best-effort on a native (winit) window (see #112 / #113).
    updateImeConfig(purpose: number): void {
      currentImePurpose = purpose;
      console.info('[floom] ime purpose set:', purpose);
    },
  });
}

// ─────────────────────────────────────────────────────────────────────
// Reverse direction: ArkTS → Rust napi exports.
// ─────────────────────────────────────────────────────────────────────

// Latest floom a11y tree (see setAccessibilityTree). Exposed for a real
// Accessibility Kit provider to consume.
let currentA11yTree: object | null = null;

// Latest IME purpose discriminant (see updateImeConfig): 0 Text, 1 Password,
// 2 Number, 3 Email, 4 Url, 5 Search. Consumed by the IME extension / a
// focused ArkTS input when configuring the soft keyboard.
let currentImePurpose: number = 0;

// Exposed for an IME extension / focused ArkTS input to read the active purpose.
export function getImePurpose(): number {
  return currentImePurpose;
}

export function getA11yTree(): object | null {
  return currentA11yTree;
}

// Call from EntryAbility.onCreate(want) / onNewWant(want) after extracting
// shared URIs from the Want (see EntryAbility.ets).
export function onSharedFiles(
  uris: Array<string>,
  mimeTypes: Array<string | null> | null,
  displayNames: Array<string | null> | null,
): void {
  floom.onSharedFiles(uris, mimeTypes, displayNames);
}

// Call from the Accessibility Kit provider's onPerformAction callback.
export function onAccessibilityAction(
  nodeId: number,
  action: string,
  payload: string | null,
): void {
  floom.onAccessibilityAction(nodeId, action, payload);
}

// Call from the Input Method Extension when the user types. Mirror of the
// Rust-side `ImeEvent` variants in crates/floom/src/platform/ohos_ime.rs.
export function imeCommit(text: string): void {
  floom.onImeCommit(text);
}

export function imePreedit(text: string, cursorStart: number, cursorEnd: number): void {
  floom.onImePreedit(text, cursorStart, cursorEnd);
}

export function imeCompositionEnd(): void {
  floom.onImeCompositionEnd();
}

export function imeDelete(before: number, after: number): void {
  floom.onImeDelete(before, after);
}
