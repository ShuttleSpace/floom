#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Build any workspace crate for Android and package it into an APK.
#
# Generalized from the per-example `examples/*/scripts/build-apk.sh`. Those
# scripts are now thin wrappers around this one, so there is a single copy of
# the aapt2 -> bundle -> zipalign -> apksigner pipeline to keep correct.
#
# `cargo ndk` ONLY compiles Rust and copies the resulting `*.so` into a
# `jniLibs/<abi>/` layout — it never produces an APK (that was `cargo-apk`,
# now unmaintained). This script does the missing half: aapt2 link, bundle
# `lib/` (and an optional `classes.dex`), zipalign, apksigner — using nothing
# but the Android SDK build-tools. No Gradle, no AGP download.
#
# Usage:
#   scripts/build-apk.sh -p <crate> [options]
#
#   -p, --package <crate>   Cargo package to build (required), e.g.
#                           `floom2-android-example`.
#   --manifest <path>       AndroidManifest.xml (default:
#                           examples/<crate>/AndroidManifest.xml).
#   --features <list>       Extra cargo features (may be repeated). Pass the
#                           crate's own feature if it gates its `android_main`
#                           (the floom 2.0 example needs `real`; the v1 example
#                           needs none).
#   -t, --target <abi>      arm64-v8a (default) | armeabi-v7a | x86 | x86_64
#   --debug | --release     Profile (default: release).
#   --skip-build            Reuse the `.so` already staged (packaging only).
#   --install               `adb install -r` the resulting APK.
#   --lib <name>            Override the android `[lib] name` (default: read
#                           from `cargo metadata`, falling back to the package
#                           name with `-` -> `_`).
#   --java <file>           Optional Java activity source to compile into a
#                           `classes.dex` (needed for runtime file shares /
#                           file-dialog results).
#   --no-plugins            Do not package the Java halves of the plugins the
#                           crate depends on (they are found through
#                           `cargo metadata`; default is to include them).
#   -h, --help
#
# Requires: Android SDK (build-tools + a platform), `cargo-ndk`, the matching
# `rustup target`, and a JDK for `apksigner` (the APK is left *unsigned* and
# clearly named `-unsigned.apk` when no JDK is found).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$ROOT" ]] || ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

PKG=""
MANIFEST=""
ABI="arm64-v8a"
PROFILE="release"
MIN_SDK=24
TARGET_SDK=34
SKIP_BUILD=0
INSTALL=0
LIB=""
JAVA_SRC=""
NO_PLUGINS=0
EXTRA_FEATURES=()

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }
warn() { echo "warning: $*" >&2; }

# Keep the target check and the build on the same toolchain: `rustup target
# list --installed` describes the rustup install, which a Homebrew `rustc`
# earlier on PATH would otherwise shadow (see scripts/build.sh).
if command -v rustup >/dev/null 2>&1; then
  rustup_bin="$(rustup which cargo 2>/dev/null || true)"
  rustup_bin="${rustup_bin%/cargo}"
  [[ -d "$rustup_bin" ]] && export PATH="$rustup_bin:$PATH"
fi

usage() { awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"; }

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--package) PKG="$2"; shift 2 ;;
    --manifest)   MANIFEST="$2"; shift 2 ;;
    --features)   EXTRA_FEATURES+=("$2"); shift 2 ;;
    -t|--target)  ABI="$2"; shift 2 ;;
    --debug)      PROFILE="debug"; shift ;;
    --release)    PROFILE="release"; shift ;;
    --skip-build) SKIP_BUILD=1; shift ;;
    --install)    INSTALL=1; shift ;;
    --lib)        LIB="$2"; shift 2 ;;
    --java)       JAVA_SRC="$2"; shift 2 ;;
    --no-plugins) NO_PLUGINS=1; shift ;;
    -h|--help)    usage; exit 0 ;;
    *)            die "unknown argument: $1 (try --help)" ;;
  esac
done

[[ -n "$PKG" ]] || die "-p/--package <crate> is required"

case "$ABI" in
  arm64-v8a)      RUST_TARGET="aarch64-linux-android" ;;
  armeabi-v7a)    RUST_TARGET="armv7-linux-androideabi" ;;
  x86)            RUST_TARGET="i686-linux-android" ;;
  x86_64)         RUST_TARGET="x86_64-linux-android" ;;
  *) die "unsupported ABI '$ABI' (expected arm64-v8a, armeabi-v7a, x86 or x86_64)" ;;
esac

# ------------------------------------------------------- package / lib lookup

# One `cargo metadata` dump, parsed by python3 when available; the fallback
# (`-` -> `_`) matches every crate in this workspace today.
METADATA="$(cargo metadata --no-deps --format-version 1 2>/dev/null || true)"
CRATE_DIR=""
if [[ -n "$METADATA" ]] && command -v python3 >/dev/null 2>&1; then
  read -r CRATE_DIR LIB_FROM_META < <(printf '%s' "$METADATA" | python3 -c '
import json, sys, os
pkg = sys.argv[1]
d = json.load(sys.stdin)
for p in d["packages"]:
    if p["name"] != pkg:
        continue
    for t in p["targets"]:
        if any(k in t["kind"] for k in ("lib", "cdylib", "rlib", "staticlib")):
            print(os.path.dirname(p["manifest_path"]), t["name"])
            raise SystemExit(0)
' "$PKG" 2>/dev/null || true)
fi
[[ -n "$LIB" ]] || LIB="${LIB_FROM_META:-${PKG//-/_}}"
[[ -n "$CRATE_DIR" ]] || CRATE_DIR="$ROOT/examples/$PKG"
[[ -n "$MANIFEST" ]] || MANIFEST="$CRATE_DIR/AndroidManifest.xml"
[[ -f "$MANIFEST" ]] || die "manifest not found: $MANIFEST (pass --manifest)"

# ------------------------------------------------------- plugin Java modules
#
# A plugin's Java half is not compiled by Cargo and not linked into the `.so`,
# so the packaging step has to find it. The list comes from the dependency
# graph (`scripts/android-plugin-modules.py`), not from a hand-written list: a
# plugin is packaged exactly when the app links it, which is also the only way
# it is reachable at runtime.
PLUGIN_NAMES=()
PLUGIN_DIRS=()
if (( ! NO_PLUGINS )) && command -v python3 >/dev/null 2>&1; then
  PLUGIN_LIST="$(python3 "$SCRIPT_DIR/android-plugin-modules.py" \
    --manifest-path "$ROOT/Cargo.toml" --package "$PKG")" \
    || die "could not read the dependency graph for $PKG"
  while IFS=$'\t' read -r plugin_name plugin_dir; do
    [[ -n "$plugin_name" ]] || continue
    PLUGIN_NAMES+=("$plugin_name")
    PLUGIN_DIRS+=("$plugin_dir")
  done <<< "$PLUGIN_LIST"
fi
if (( ${#PLUGIN_NAMES[@]} > 0 )); then
  info "plugin Java modules: ${PLUGIN_NAMES[*]}"
fi

info "package=$PKG lib=$LIB abi=$ABI profile=$PROFILE"

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

OUT_DIR="$ROOT/target/android"
STAGE="$OUT_DIR/stage/$PKG-$ABI-$PROFILE"
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

  FEATURE_FLAG=()
  for f in ${EXTRA_FEATURES[@]+"${EXTRA_FEATURES[@]}"}; do
    FEATURE_FLAG+=(--features "$f")
  done

  info "cargo ndk -t $ABI -P $MIN_SDK build $PROFILE_FLAG -p $PKG --lib ${FEATURE_FLAG[*]:-}"
  cargo ndk -t "$ABI" -P "$MIN_SDK" -o "$STAGE/lib" build $PROFILE_FLAG -p "$PKG" --lib \
    ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}
fi

# ------------------------------------------- Java halves -> DEX
# A plugin's capability lives in its Java half (`<crate>/android/src/main/java`),
# and Rust reaches it through JNI. `cargo ndk` never touches Java, so every
# source the app links is compiled here with `javac` and turned into one
# `classes.dex` with `d8` (falling back to the legacy `dx`). One dex, not one
# per plugin: the classes are in the same package and the same class loader.
#
# `--java` stays for a single Java activity subclass (e.g.
# `FloomNativeActivity`, which forwards onActivityResult / onNewIntent to
# Floom); it is compiled together with the plugin sources.
if [[ -z "$JAVA_SRC" && -f "$CRATE_DIR/FloomNativeActivity.java" ]]; then
  JAVA_SRC="$CRATE_DIR/FloomNativeActivity.java"
fi

JAVA_SOURCES=()
if [[ -n "$JAVA_SRC" ]]; then
  JAVA_SOURCES+=("$JAVA_SRC")
fi
for plugin_dir in ${PLUGIN_DIRS[@]+"${PLUGIN_DIRS[@]}"}; do
  while IFS= read -r source; do
    JAVA_SOURCES+=("$source")
  done < <(find "$plugin_dir/android/src/main/java" -type f -name '*.java' | sort)
done

HAS_CODE=false
if (( ${#JAVA_SOURCES[@]} > 0 )); then
  info "compiling ${#JAVA_SOURCES[@]} Java source(s) -> classes.dex"
  command -v javac >/dev/null || die "javac not found on PATH (required for the Java halves)"
  D8="$BUILD_TOOLS/d8"
  [[ -x "$D8" ]] || D8="$BUILD_TOOLS/dx"
  [[ -x "$D8" ]] || die "neither d8 nor dx found under $BUILD_TOOLS (required for classes.dex)"
  JAVAC_OUT="$STAGE/java"
  rm -rf "$JAVAC_OUT"; mkdir -p "$JAVAC_OUT"
  # `--release 17` pins the class-file version (61) so the bundled `d8` (R8)
  # accepts it; the host JDK may be far newer and emit a class version d8
  # rejects. The sources only use basic Android/Java APIs, so 17 is safe.
  javac --release 17 -cp "$ANDROID_JAR" -d "$JAVAC_OUT" "${JAVA_SOURCES[@]}" \
    || die "javac failed"
  CLASSES=()
  while IFS= read -r class_file; do
    CLASSES+=("$class_file")
  done < <(find "$JAVAC_OUT" -name '*.class')
  [[ ${#CLASSES[@]} -gt 0 ]] || die "javac produced no .class files"
  "$D8" --lib "$ANDROID_JAR" --output "$STAGE" "${CLASSES[@]}" \
    || die "d8 failed to build classes.dex"
  HAS_CODE=true
fi

# ------------------------------- manifest merge (plugin permissions + hasCode)
# `aapt2 link` takes one manifest, and AGP's manifest merger is not in this
# pipeline, so the parts a plugin contributes (`<uses-permission>`,
# `<uses-feature>`) and the `hasCode` flag that matches the dex are applied
# here. Anything else in a plugin manifest is refused rather than guessed at.
MERGE_ARGS=(--app "$MANIFEST" --out "$STAGE/AndroidManifest.xml" --has-code "$HAS_CODE")
for plugin_dir in ${PLUGIN_DIRS[@]+"${PLUGIN_DIRS[@]}"}; do
  plugin_manifest="$plugin_dir/android/src/main/AndroidManifest.xml"
  [[ -f "$plugin_manifest" ]] || die "plugin manifest not found: $plugin_manifest"
  MERGE_ARGS+=(--plugin "$plugin_manifest")
done
command -v python3 >/dev/null || die "python3 not found on PATH (required to merge Android manifests)"
python3 "$SCRIPT_DIR/android-manifest-merge.py" "${MERGE_ARGS[@]}" \
  || die "manifest merge failed"
MANIFEST="$STAGE/AndroidManifest.xml"

SO="$STAGE/lib/$ABI/lib$LIB.so"
[[ -f "$SO" ]] || die "expected $SO — is the crate a cdylib? (see [lib] crate-type in Cargo.toml)"
info "bundling $(basename "$SO") ($(du -h "$SO" | cut -f1))"

# --------------------------------------------------------------- APK packaging

info "aapt2 link"
(cd "$STAGE" && "$AAPT2" link -I "$ANDROID_JAR" \
  --manifest "$MANIFEST" \
  --min-sdk-version "$MIN_SDK" --target-sdk-version "$TARGET_SDK" \
  -o "$STAGE/unsigned.apk")

# aapt2 has no native-lib flag; add the (uncompressed) libs AND optional dex.
command -v zip >/dev/null || die "zip not found on PATH"
(cd "$STAGE" && zip -0 -q -r unsigned.apk lib)
if [[ -f "$STAGE/classes.dex" ]]; then
  (cd "$STAGE" && zip -0 -q -r unsigned.apk classes.dex)
fi

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

echo "ARTIFACT=$APK"
