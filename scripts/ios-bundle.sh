#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Build a workspace crate for iOS and assemble an installable `.app` bundle.
#
# `cargo xcode` (cargo-xcode) only generates a project whose product type is a
# command-line tool — `xcodebuild` then produces a bare Mach-O, which
# `simctl install` refuses. This script does the missing half: build the Rust
# binary for the right Apple target and assemble + ad-hoc sign a real `.app`.
#
# The Info.plist it writes mirrors the keys `examples/ios/Info.plist` uses
# (LSRequiresIPhoneOS, UILaunchScreen, UIApplicationSceneManifest, orientation
# and device-capability declarations), parameterised for an arbitrary crate.
#
# Usage:
#   scripts/ios-bundle.sh <crate> [options]
#
#   <crate>                 Cargo package to build (required).
#   --device                Target a real device (`aarch64-apple-ios`,
#                           `-sdk iphoneos`). Default is the simulator
#                           (`aarch64-apple-ios-sim`). A device build needs a
#                           real signing identity + provisioning profile; the
#                           ad-hoc signature this script applies is
#                           simulator-only, so a device artifact is reported as
#                           unsigned/blocked.
#   --release               Build the release profile (default: debug).
#   --debug                 Build the debug profile (default).
#   --features <list>       Extra cargo features (may be repeated).
#   --bin <name>            Override the binary name (default: read from
#                           `cargo metadata`).
#   --bundle-id <id>        CFBundleIdentifier (default: dev.floom.<crate>).
#   --display-name <name>   CFBundleDisplayName (default: the crate name).
#   -h, --help
#
# Prints `ARTIFACT=<abs path to .app>` as its last line.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$ROOT" ]] || ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }

# Keep the target check and the build on the same toolchain: `rustup target
# list --installed` describes the rustup install, which a Homebrew `rustc`
# earlier on PATH would otherwise shadow (see scripts/build.sh).
if command -v rustup >/dev/null 2>&1; then
  rustup_bin="$(rustup which cargo 2>/dev/null || true)"
  rustup_bin="${rustup_bin%/cargo}"
  [[ -d "$rustup_bin" ]] && export PATH="$rustup_bin:$PATH"
fi

usage() { awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"; }

CRATE=""
DEVICE=0
PROFILE="debug"
BIN=""
BUNDLE_ID=""
DISPLAY_NAME=""
EXTRA_FEATURES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --device)       DEVICE=1; shift ;;
    --release)      PROFILE="release"; shift ;;
    --debug)        PROFILE="debug"; shift ;;
    --features)     EXTRA_FEATURES+=("$2"); shift 2 ;;
    --bin)          BIN="$2"; shift 2 ;;
    --bundle-id)    BUNDLE_ID="$2"; shift 2 ;;
    --display-name) DISPLAY_NAME="$2"; shift 2 ;;
    -h|--help)      usage; exit 0 ;;
    -*)             die "unknown argument: $1 (try --help)" ;;
    *)              [[ -z "$CRATE" ]] || die "unexpected extra argument: $1"; CRATE="$1"; shift ;;
  esac
done

[[ -n "$CRATE" ]] || die "<crate> is required"

if (( DEVICE )); then
  TARGET="aarch64-apple-ios"
else
  TARGET="aarch64-apple-ios-sim"
fi

if command -v rustup >/dev/null && ! rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
  die "rust target $TARGET not installed. Run: rustup target add $TARGET"
fi

# ------------------------------------------------------------- binary lookup

if [[ -z "$BIN" ]] && command -v python3 >/dev/null 2>&1; then
  BIN="$(cargo metadata --no-deps --format-version 1 2>/dev/null | python3 -c '
import json, sys
pkg = sys.argv[1]
d = json.load(sys.stdin)
for p in d["packages"]:
    if p["name"] != pkg:
        continue
    for t in p["targets"]:
        if "bin" in t["kind"]:
            print(t["name"])
            raise SystemExit(0)
' "$CRATE" 2>/dev/null || true)"
fi
[[ -n "$BIN" ]] || BIN="$CRATE"

[[ -n "$BUNDLE_ID" ]] || BUNDLE_ID="dev.floom.${CRATE//-/.}"
[[ -n "$DISPLAY_NAME" ]] || DISPLAY_NAME="$CRATE"

# ------------------------------------------------------------------ Rust build

PROFILE_FLAG=()
[[ "$PROFILE" == "release" ]] && PROFILE_FLAG=(--release)
FEATURE_FLAG=()
for f in ${EXTRA_FEATURES[@]+"${EXTRA_FEATURES[@]}"}; do
  FEATURE_FLAG+=(--features "$f")
done

info "cargo build -p $CRATE --target $TARGET ${PROFILE_FLAG[*]:-} ${FEATURE_FLAG[*]:-}"
cargo build -p "$CRATE" --target "$TARGET" \
  ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}

EXE="$ROOT/target/$TARGET/$PROFILE/$BIN"
[[ -f "$EXE" ]] || die "expected binary $EXE — does the crate produce a bin named '$BIN'?"

# ------------------------------------------------------------------- bundle

APP="$ROOT/target/ios/$CRATE.app"
rm -rf "$APP"
mkdir -p "$APP"
cp "$EXE" "$APP/$BIN"

cat > "$APP/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$CRATE</string>
    <key>CFBundleDisplayName</key><string>$DISPLAY_NAME</string>
    <key>CFBundleIdentifier</key><string>$BUNDLE_ID</string>
    <key>CFBundleExecutable</key><string>$BIN</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSRequiresIPhoneOS</key><true/>
    <key>UILaunchScreen</key><dict/>
    <key>UIApplicationSupportsIndirectInputEvents</key><true/>
    <key>UIApplicationSceneManifest</key>
    <dict>
        <key>UIApplicationSupportsMultipleScenes</key><false/>
    </dict>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
    <key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>
</dict>
</plist>
PLIST

info "codesign (ad-hoc)"
codesign --force --sign - "$APP"

if (( DEVICE )); then
  echo "warning: ad-hoc signature is simulator-only; install on a device needs a" >&2
  echo "         real identity + provisioning profile. The .app was still produced." >&2
fi

echo "ARTIFACT=$APP"
