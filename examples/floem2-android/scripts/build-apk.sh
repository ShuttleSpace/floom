#!/usr/bin/env bash
# Build the Floem 2.0 Android example and package it into an APK.
#
# Thin wrapper: the aapt2 -> bundle -> zipalign -> apksigner pipeline lives in
# `scripts/build-apk.sh` at the workspace root (one copy to maintain). This
# wrapper only pins the example's package name and its mandatory `real` feature
# — the whole `android` module (and the exported `android_main`) is gated on
# `#[cfg(all(target_os = "android", feature = "real"))]`, so a build without it
# produces a `.so` with no entry point that `native-activity` cannot start.
#
# Usage:
#   scripts/build-apk.sh [-t <abi>] [--debug|--release] [--skip-build] [--install]
#
#   -t <abi>       arm64-v8a (default) | armeabi-v7a | x86 | x86_64
#   --debug        Build the debug profile (default: release)
#   --release      Build the release profile
#   --skip-build   Reuse the `.so` already in the staging dir (packaging only)
#   --install      `adb install -r` the resulting APK
#
# Requires: Android SDK (build-tools + a platform), `cargo-ndk`, the matching
# `rustup target`, and a JDK for `apksigner`.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$ROOT" ]] || ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

exec "$ROOT/scripts/build-apk.sh" -p floem2-android-example --features real "$@"
