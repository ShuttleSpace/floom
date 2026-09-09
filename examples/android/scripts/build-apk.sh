#!/usr/bin/env bash
# Build the Floem Android example and package it into an APK.
#
# `cargo ndk` ONLY compiles Rust and copies the resulting `*.so` into a
# `jniLibs/<abi>/` layout — it never produces an APK (that was `cargo-apk`,
# now unmaintained). This script does the missing half: aapt2 link ->
# bundle `lib/` -> zipalign -> apksigner, using nothing but the Android SDK
# build-tools. No Gradle, no AGP download, no JVM except for signing.
#
# Usage:
#   scripts/build-apk.sh [-t <abi>] [--debug|--release] [--skip-build] [--install]
#
#   -t <abi>       arm64-v8a (default) | armeabi-v7a | x86 | x86_64
#   --debug        build the debug profile (default: release)
#   --release      build the release profile
#   --skip-build   reuse the `.so` already in the staging dir (packaging only)
#   --install      `adb install -r` the resulting APK
#
# Requires: Android SDK (build-tools + a platform), `cargo-ndk`, the matching
# `rustup target`, and a JDK for `apksigner` (the APK is left *unsigned* and
# clearly named `-unsigned.apk` when no JDK is found).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXAMPLE_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$EXAMPLE_DIR"

PKG="floem-android-example"
LIB="${PKG//-/_}"          # floem_android_example
ABI="arm64-v8a"
PROFILE="release"
SKIP_BUILD=0
INSTALL=0

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }
warn() { echo "warning: $*" >&2; }

usage() { sed -n '2,25p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -t|--target) ABI="$2"; shift 2 ;;
    --debug)     PROFILE="debug"; shift ;;
    --release)   PROFILE="release"; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --install)   INSTALL=1; shift ;;
    -h|--help)   usage; exit 0 ;;
    *)           die "unknown argument: $1 (try --help)" ;;
  esac
done

case "$ABI" in
  arm64-v8a)      RUST_TARGET="aarch64-linux-android" ;;
  armeabi-v7a)    RUST_TARGET="armv7-linux-androideabi" ;;
  x86)            RUST_TARGET="i686-linux-android" ;;
  x86_64)         RUST_TARGET="x86_64-linux-android" ;;
  *) die "unsupported ABI '$ABI' (expected arm64-v8a, armeabi-v7a, x86 or x86_64)" ;;
esac

# ---------------------------------------------------------------- SDK location

ANDROID_HOME="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-}}"
if [[ -z "$ANDROID_HOME" ]]; then
  for cand in "$HOME/Library/Android/sdk" "$HOME/Android/Sdk" "/usr/local/lib/android/sdk"; do
    [[ -d "$cand" ]] && { ANDROID_HOME="$cand"; break; }
  done
fi
[[ -d "${ANDROID_HOME:-}" ]] || die "Android SDK not found; set ANDROID_HOME (or ANDROID_SDK_ROOT)"

BUILD_TOOLS="$(ls -1d "$ANDROID_HOME"/build-tools/*/ 2>/dev/null | sort -V | tail -1 | sed 's:/$::')"
[[ -n "$BUILD_TOOLS" ]] || die "no build-tools under $ANDROID_HOME/build-tools (install one via sdkmanager)"

# Highest installed platform that actually ships android.jar.
ANDROID_JAR=""
for p in $(ls -1d "$ANDROID_HOME"/platforms/android-*/ 2>/dev/null | sort -V); do
  [[ -f "${p}android.jar" ]] && ANDROID_JAR="${p}android.jar"
done
[[ -n "$ANDROID_JAR" ]] || die "no platforms/android-*/android.jar under $ANDROID_HOME"

AAPT2="$BUILD_TOOLS/aapt2"
ZIPALIGN="$BUILD_TOOLS/zipalign"
APKSIGNER="$BUILD_TOOLS/apksigner"
for tool in "$AAPT2" "$ZIPALIGN"; do
  [[ -x "$tool" ]] || die "missing $tool"
done

# ------------------------------------------------------------- output locations

ROOT="$(git -C "$EXAMPLE_DIR" rev-parse --show-toplevel 2>/dev/null || echo "$EXAMPLE_DIR")"
OUT_DIR="$ROOT/target/android"
STAGE="$OUT_DIR/stage/$ABI-$PROFILE"
# `--skip-build` must preserve the staged `.so`, so only wipe when rebuilding.
(( SKIP_BUILD )) || rm -rf "$STAGE"
mkdir -p "$STAGE/lib/$ABI"

# ------------------------------------------------------------------ Rust build

if (( SKIP_BUILD )); then
  info "skipping cargo build (--skip-build)"
else
  command -v cargo >/dev/null || die "cargo not found on PATH"
  cargo ndk --version >/dev/null 2>&1 \
    || die "cargo-ndk not installed. Run: cargo install cargo-ndk
   (note: cargo-ndk builds the .so only — this script turns it into an APK)"
  if command -v rustup >/dev/null \
     && ! rustup target list --installed 2>/dev/null | grep -qx "$RUST_TARGET"; then
    die "rust target $RUST_TARGET not installed. Run: rustup target add $RUST_TARGET"
  fi

  PROFILE_FLAG=""
  [[ "$PROFILE" == "release" ]] && PROFILE_FLAG="--release"

  info "cargo ndk -t $ABI build $PROFILE_FLAG"
  cargo ndk -t "$ABI" -o "$STAGE/lib" build $PROFILE_FLAG
fi

SO="$STAGE/lib/$ABI/lib$LIB.so"
[[ -f "$SO" ]] || die "expected $SO — is the crate a cdylib? (see [lib] crate-type in Cargo.toml)"
info "bundling $(basename "$SO") ($(du -h "$SO" | cut -f1))"

# --------------------------------------------------------------- APK packaging

info "aapt2 link"
(cd "$STAGE" && "$AAPT2" link -I "$ANDROID_JAR" \
  --manifest "$EXAMPLE_DIR/AndroidManifest.xml" \
  --min-sdk-version 24 --target-sdk-version 34 \
  -o "$STAGE/unsigned.apk")

# aapt2 has no native-lib flag; add the (uncompressed) libs to the archive.
command -v zip >/dev/null || die "zip not found on PATH"
(cd "$STAGE" && zip -0 -q -r unsigned.apk lib)

info "zipalign"
"$ZIPALIGN" -f -P 16 4 "$STAGE/unsigned.apk" "$STAGE/aligned.apk"

# ------------------------------------------------------------------- signing

if command -v java >/dev/null && java -version 2>&1 | grep -q '"'; then
  KEYSTORE="$OUT_DIR/debug.keystore"
  if [[ ! -f "$KEYSTORE" ]]; then
    info "generating debug keystore"
    mkdir -p "$OUT_DIR"
    keytool -genkeypair -v -keystore "$KEYSTORE" -storepass android \
      -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 \
      -validity 10000 -dname "CN=Android Debug,O=Android,C=US" >/dev/null
  fi
  APK="$OUT_DIR/$LIB-$ABI-$PROFILE.apk"
  info "apksigner"
  "$APKSIGNER" sign --ks "$KEYSTORE" --ks-pass pass:android \
    --key-pass pass:android --ks-key-alias androiddebugkey \
    --out "$APK" "$STAGE/aligned.apk"
else
  APK="$OUT_DIR/$LIB-$ABI-$PROFILE-unsigned.apk"
  warn "no working JDK found — skipping apksigner."
  warn "the APK is UNSIGNED and will not install until you sign it:"
  warn "  $APKSIGNER sign --ks <keystore> --out signed.apk $APK"
  cp "$STAGE/aligned.apk" "$APK"
fi

info "APK: $APK"

if (( INSTALL )); then
  command -v adb >/dev/null || die "adb not found on PATH"
  info "adb install -r"
  adb install -r "$APK"
fi
