#!/usr/bin/env bash
# Build the Floem (v1) Android example and package it into an APK.
#
# Thin wrapper: the aapt2 -> bundle -> zipalign -> apksigner pipeline lives in
# `scripts/build-apk.sh` at the workspace root (one copy to maintain). This
# wrapper only pins the example's package name and manifest location — the
# crate lives in `examples/android/` rather than the default
# `examples/<package>/`, and `FloemNativeActivity.java` next to it is picked up
# automatically to produce `classes.dex` (needed for runtime file shares and
# file-dialog results).
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

exec "$ROOT/scripts/build-apk.sh" \
  -p floem-android-example \
  --manifest "$SCRIPT_DIR/../AndroidManifest.xml" \
  "$@"
