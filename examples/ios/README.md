# Floem iOS Example

A minimal Floem counter + text-input app that runs on iOS. It demonstrates the
Phase 2 iOS backend entry point: `floem::Application::new_ios(...)`.

The example deliberately exercises the headline iOS features that are already
implemented (Steps 1–5):

- **Skeleton + Metal GPU surface** — `Application::new_ios` builds a winit
  `EventLoop`; winit calls `UIApplicationMain` and renders through a `CAMetalLayer`.
- **in-UI context menu** — long-press surfaces Floem's platform-agnostic menu.
- **Clipboard** — `UIPasteboard` (`objc2` bridge), usable from the text field.
- **Soft keyboard / IME (Step 5)** — tapping the text field focuses it, which
  makes a hidden `UITextField` first responder and summons the iOS keyboard;
  typed text is re-injected into Floem as `Event::Ime`.

> Status: **runs on the iOS Simulator** (verified 2026-09-27 — see
> [`docs/gates/ios.md`](../../docs/gates/ios.md) for the per-item evidence with
> screenshots). The backend steps 1–10 are implemented and compile for
> `aarch64-apple-ios` and `aarch64-apple-ios-sim`; `scripts/build.sh
> floem-ios-example ios` now assembles a real, installable `.app`. Two
> *rendering/robustness* defects remain and are recorded, not hidden: **CJK
> characters render as tofu** (the text stack finds no Han-capable face on iOS —
> Latin is unaffected), and **a11y elements report zero frames** (labels reach
> the OS but VoiceOver cannot place them). `ios_capabilities().accessibility`
> is still `false` (VoiceOver is Phase 4). See
> [`docs/ios.md`](../../docs/ios.md) for the design log.

## Prerequisites

- macOS with Xcode (the iOS SDK + `xcodebuild` are required to link/run).
- Rust iOS targets:
  ```bash
  rustup target add aarch64-apple-ios aarch64-apple-ios-sim
  ```
- [`cargo-xcode`](https://crates.io/crates/cargo-xcode) to generate an Xcode
  project that links the Rust binary into a real `.app` bundle:
  ```bash
  cargo install cargo-xcode
  ```

## Build a Xcode project

From this directory (or the workspace root):

```bash
cargo xcode
```

This produces an `.xcodeproj` that wraps the Rust `staticlib`/`bin`. Open it in
Xcode:

```bash
open floem-ios-example.xcodeproj
```

## iOS 26+ scene lifecycle is mandatory

Apps built with the iOS 26+ SDK **must** adopt the UIKit scene lifecycle or
they crash at launch (`EXC_BREAKPOINT` in
`___UIApplicationEvaluateRuntimeIssueForNoSceneLifecycleAdoption`). After
generating the Xcode project, make sure the built `.app`'s `Info.plist`
contains:

```xml
<key>UIApplicationSceneManifest</key>
<dict>
  <key>UIApplicationSupportsMultipleScenes</key><false/>
</dict>
```

(`cargo-xcode` does not add this key itself — patch it with `PlistBuddy` or an
Xcode build phase when assembling the bundle.)

The vendored `winit-uikit` already provides the code side: the app delegate
returns a `UISceneConfiguration` with a `SceneDelegate`, and windows created
before the scene connects (iOS 26+ delivers `Resumed` *before*
`scene:willConnectToSession:`) are parked in an early-window registry and
attached to the `UIWindowScene` when UIKit connects it. Without that registry
the window never joins a scene and the app renders a black screen.

## Run in the Simulator

The working path builds the Rust binary for `aarch64-apple-ios-sim` and
assembles + ad-hoc signs an installable `.app` (no Xcode project target needed):

```bash
./scripts/build.sh floem-ios-example ios      # prints ARTIFACT=…/target/ios/floem-ios-example.app
xcrun simctl boot <udid>                       # e.g. xcrun simctl list devices | grep iPhone
xcrun simctl install <udid> target/ios/floem-ios-example.app
xcrun simctl launch  <udid> dev.floem.floem.ios.example
xcrun simctl io <udid> screenshot /tmp/ios-launch.png
```

The counter buttons and the text field (soft keyboard) respond. To *drive* the
simulator from the command line — `simctl` has no tap/type command — use `idb`
(the companion server plus the `fb-idb` CLI):

```bash
idb_companion --udid <udid> --grpc-port 10882 &
idb --companion localhost:10882 ui tap 156 415        # coordinates in POINTS, not pixels
idb --companion localhost:10882 ui text "hello"
idb --companion localhost:10882 ui describe-all       # a11y tree
```

Note the coordinates are in **points** on the 402×874 pt screen (the screenshot
is 1206×2622 px at 3×). The clipboard can be checked from outside the process
with `xcrun simctl pbpaste` / `pbcopy <udid>`.

## Run on a device

Device builds require an Apple developer identity for code signing. In Xcode,
select a real device, set your Team/signing certificate, and Run. (This is the
only fully-unverified step, identical in spirit to Android's APK signing.)

## Verify compile only (no Xcode)

You can confirm the example builds without launching Xcode:

```bash
cargo check -p floem-ios-example --target aarch64-apple-ios-sim
cargo check -p floem-ios-example --target aarch64-apple-ios
```
