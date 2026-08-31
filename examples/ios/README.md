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

> Status: **skeleton**. The app compiles for `aarch64-apple-ios` and
> `aarch64-apple-ios-sim`. The full app-bundle / signing / on-device run is the
> remaining integration step (see Build below). Features **not yet implemented**
> are still flagged `false` in `ios_capabilities()`: external URL open (Step 6),
> file dialog (Step 7), receiving shared files (Step 8), and safe-area insets
> (Step 9). See [`docs/ios.md`](../../docs/ios.md).

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

## Run in the Simulator

In Xcode, pick the **iPhone Simulator** destination and press Run, or from the
command line build the simulator target directly:

```bash
cargo xcode build --target aarch64-apple-ios-sim
```

The counter buttons, long-press context menu, and the text field (soft
keyboard) should all respond.

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
