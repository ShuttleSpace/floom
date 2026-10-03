import { AbilityConstant, UIAbility, Want } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { window } from '@kit.ArkUI';
import bridge from 'libentry.so';
import {
  setAppContext,
  registerPlatformBridge,
  registerFloomA11yProvider,
  onSharedFiles,
} from '../FloomBridge';

const DOMAIN = 0x3433;

interface SharedFiles {
  uris: Array<string>;
  mimeTypes: Array<string | null>;
  displayNames: Array<string | null>;
}

export default class EntryAbility extends UIAbility {
  private mainWindow?: window.Window;

  private readonly onSystemDensityChange = (density: number): void => {
    bridge.setScaleFactor(this.normalizeDensityScale(density));
  };

  onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    hilog.info(DOMAIN, 'cargo-ohos-app', '%{public}s', 'Ability onCreate');
    this.forwardSharedFiles(want);
  }

  onNewWant(want: Want, launchParam: AbilityConstant.LaunchParam): void {
    this.forwardSharedFiles(want);
  }

  onWindowStageCreate(windowStage: window.WindowStage): void {
    try {
      const mainWindow = windowStage.getMainWindowSync();
      this.mainWindow = mainWindow;
      this.syncDensityScale(mainWindow);
      mainWindow.on('systemDensityChange', this.onSystemDensityChange);
      mainWindow.setWindowLayoutFullScreen(true)
        .then(() => mainWindow.setWindowSystemBarEnable([]))
        .catch((err: Error) => {
          hilog.error(
            DOMAIN,
            'cargo-ohos-app',
            'Failed to configure immersive window: %{public}s',
            JSON.stringify(err)
          );
        });
    } catch (err) {
      hilog.error(
        DOMAIN,
        'cargo-ohos-app',
        'Failed to get main window: %{public}s',
        JSON.stringify(err)
      );
    }

    this.registerBridge();

    windowStage.loadContent('pages/Index', (err) => {
      if (err.code) {
        hilog.error(DOMAIN, 'cargo-ohos-app', 'Failed to load page: %{public}s', JSON.stringify(err));
      }
    });
  }

  onWindowStageDestroy(): void {
    if (this.mainWindow !== undefined) {
      try {
        this.mainWindow.off('systemDensityChange', this.onSystemDensityChange);
      } catch (err) {
        hilog.error(
          DOMAIN,
          'cargo-ohos-app',
          'Failed to unregister density listener: %{public}s',
          JSON.stringify(err)
        );
      }
      this.mainWindow = undefined;
    }
  }

  // Register the ArkTS half of floom's platform bridge (clipboard, file picker,
  // external URL, a11y feed, IME). Rendering does not depend on it: the
  // XComponent page loads libentry.so on its own, so a bridge failure is logged
  // and swallowed rather than allowed to take the surface down with it.
  private registerBridge(): void {
    try {
      setAppContext(this.context);
      registerPlatformBridge();
      registerFloomA11yProvider();
    } catch (err) {
      const e = err as Error;
      hilog.error(
        DOMAIN,
        'cargo-ohos-app',
        'Failed to register platform bridge: %{public}s',
        e?.message ?? String(err)
      );
    }
  }

  private forwardSharedFiles(want: Want): void {
    try {
      const shared = extractSharedFiles(want);
      if (shared.uris.length === 0) {
        return;
      }
      onSharedFiles(shared.uris, shared.mimeTypes, shared.displayNames);
    } catch (err) {
      hilog.error(
        DOMAIN,
        'cargo-ohos-app',
        'Failed to forward shared files: %{public}s',
        JSON.stringify(err)
      );
    }
  }

  private syncDensityScale(mainWindow: window.Window): void {
    try {
      const densityInfo = mainWindow.getWindowDensityInfo();
      bridge.setScaleFactor(
        this.normalizeDensityScale(
          densityInfo.customDensity > 0
            ? densityInfo.customDensity
            : (densityInfo.systemDensity > 0 ? densityInfo.systemDensity : densityInfo.defaultDensity)
        )
      );
    } catch (err) {
      hilog.error(
        DOMAIN,
        'cargo-ohos-app',
        'Failed to read window density: %{public}s',
        JSON.stringify(err)
      );
      bridge.setScaleFactor(1.0);
    }
  }

  private normalizeDensityScale(density: number): number {
    if (typeof density === 'number' && Number.isFinite(density) && density > 0) {
      return density;
    }
    return 1.0;
  }
}

// Extract URIs shared into the app via a share sheet / "open with".
// OHOS delivers a single uri on `want.uri` and batch shares as an array under
// `ability.params.stream`. MIME types / display names are not always present,
// so they are forwarded as null and floom fills them in from the URI.
function extractSharedFiles(want: Want): SharedFiles {
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