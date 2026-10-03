// EntryAbility — OpenHarmony Stage model entry that wires Floom's platform
// bridge into the OS.
//
// This file is the integration point between the ArkTS ability lifecycle and
// floom's Rust N-API bridge (see FloomBridge.ets). To use it:
//
//   1. Register this ability in your module.json5:
//        "abilities": [{
//          "name": "EntryAbility",
//          "srcEntry": "./ets/EntryAbility.ets",
//          "startWindowIcon": "$media:startIcon",
//          "startBackgroundColor": "#FFFFFF"
//        }]
//   2. Make sure the Rust cdylib (libohos.so) is linked by cargo-ohos-app and
//      its `#[napi]` exports are importable from './FloomBridge'.
//   3. Add `pages/Index` (a minimal XComponent surface) if the build needs an
//      ArkTS UI to host the native window; otherwise the Rust/winit window is
//      driven by the export_ohos_winit_app! entry point.

import { UIAbility, Want } from '@kit.AbilityKit';
import { window } from '@kit.WindowKit';
import {
  setAppContext,
  registerPlatformBridge,
  registerFloomA11yProvider,
  onSharedFiles,
} from './FloomBridge';

export default class EntryAbility extends UIAbility {
  onWindowStageCreate(windowStage: window.WindowStage): void {
    // 1. Give the bridge the ability context so openUrl() can startAbility.
    setAppContext(this.context);
    // 2. Register the platform bridge (clipboard / file / external / a11y / IME).
    registerPlatformBridge();
    // 3. ArkTS a11y seam (see FloomA11y.ets). NOTE: the real OHOS screen-reader
    //    provider is registered from Rust via the native C ArkUI_Accessibility
    //    Provider (task #121) — ArkTS cannot register a provider. This call is
    //    a no-op that documents the correct architecture.
    registerFloomA11yProvider();

    // 4. Host the native (Rust/winit) window. With cargo-ohos-app the native
    //    surface is created by libohos.so's export_ohos_winit_app! entry. If
    //    your project needs an ArkTS page to embed an XComponent, load it:
    //      windowStage.loadContent('pages/Index');
  }

  onCreate(want: Want): void {
    this.forwardSharedFiles(want);
  }

  onNewWant(want: Want): void {
    this.forwardSharedFiles(want);
  }

  private forwardSharedFiles(want: Want): void {
    const shared = extractSharedFiles(want);
    if (shared.uris.length === 0) {
      return;
    }
    onSharedFiles(shared.uris, shared.mimeTypes, shared.displayNames);
  }
}

// Extract URIs shared into the app via a share sheet / "open with".
// OHOS delivers a single uri on `want.uri` and batch shares as an array under
// `ability.params.stream`. MIME types / display names are not always present,
// so they are forwarded as null and floom fills them in from the URI.
function extractSharedFiles(want: Want): {
  uris: Array<string>;
  mimeTypes: Array<string | null>;
  displayNames: Array<string | null>;
} {
  const uris: Array<string> = [];
  const mimeTypes: Array<string | null> = [];
  const displayNames: Array<string | null> = [];

  if (want.uri && want.uri.length > 0) {
    uris.push(want.uri);
    mimeTypes.push(null);
    displayNames.push(null);
  }

  const params = want.parameters as Record<string, Object> | undefined;
  const stream = params?.['ability.params.stream'] as Array<string> | undefined;
  if (stream) {
    for (const uri of stream) {
      uris.push(uri);
      mimeTypes.push(null);
      displayNames.push(null);
    }
  }

  return { uris, mimeTypes, displayNames };
}
