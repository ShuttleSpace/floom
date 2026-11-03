#!/usr/bin/env bash
# One command to build a workspace crate for a target platform.
#
# This is the single entry point the "one command per platform" convention is
# built on. It does not reinvent the packaging steps: Android delegates to
# `scripts/build-apk.sh`, iOS to `scripts/ios-bundle.sh`, and OHOS drives the
# existing hvigor Stage project. What it adds is a uniform spelling, a uniform
# result line, and an honest capability tier — a platform whose backend cannot
# render (or whose toolchain is missing) is reported as `check-only`/`blocked`,
# never as `artifact`.
#
# Usage:
#   scripts/build.sh <crate> <platform> [options]
#
#   <platform> = macos | android | ios | ohos | web | windows | linux
#
#   --release            Release profile (default: debug).
#   --features <list>    Cargo features (repeatable). When omitted, a default is
#                        derived from the crate: its `<platform>` feature if it
#                        has one, else `real` if it has one, else none.
#   --abi <abi>          Android ABI (arm64-v8a default; see build-apk.sh).
#   --manifest <path>    AndroidManifest.xml (Android only).
#   --ohos-app-dir <d>   OHOS hvigor project (default: examples/<crate>/ohos-app).
#   --bin <name>         Bin name override (macOS/web).
#   --device             iOS real-device target (default: simulator).
#   --install            adb install the APK / (Android only).
#   -h, --help
#
# Result line (always printed last):
#   TIER=<artifact|check-only|blocked> PLATFORM=<p> CRATE=<c> ARTIFACT=<path|->

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null)"
[[ -n "$ROOT" ]] || ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

die()  { echo "error: $*" >&2; exit 1; }
info() { echo "==> $*"; }
warn() { echo "warning: $*" >&2; }

# Prefer the rustup toolchain over any Homebrew/system `rustc` earlier on PATH.
# Only the rustup install carries the cross-compilation targets
# (`wasm32-unknown-unknown`, `aarch64-linux-android`, `*-linux-ohos`, ...), and
# `rustup target list --installed` reflects *that* toolchain — so with a
# shadowing `cargo` the target check passes while the real build dies with
# "can't find crate for `core`". Putting rustup's bin first keeps the check and
# the build on the same toolchain.
if command -v rustup >/dev/null 2>&1; then
  rustup_bin="$(rustup which cargo 2>/dev/null || true)"
  rustup_bin="${rustup_bin%/cargo}"
  if [[ -d "$rustup_bin" ]]; then
    export PATH="$rustup_bin:$PATH"
    # rustup's `rust-lld` (used to link wasm32) has a wrong @rpath for
    # libLLVM.dylib on macOS: it looks under lib/rustlib/<host>/lib, but the
    # dylib sits in ../../lib. Point dyld at the toolchain lib dir so the wasm
    # link does not fail with "Library not loaded: @rpath/libLLVM.dylib".
    toolchain_lib="$(dirname "$rustup_bin")/lib"
    if [[ -f "$toolchain_lib/libLLVM.dylib" ]]; then
      export DYLD_FALLBACK_LIBRARY_PATH="$toolchain_lib${DYLD_FALLBACK_LIBRARY_PATH:+:$DYLD_FALLBACK_LIBRARY_PATH}"
    fi
  fi
fi

usage() { awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"; }

CRATE=""
PLATFORM=""
PROFILE="debug"
ABI="arm64-v8a"
MANIFEST=""
OHOS_APP_DIR=""
BIN=""
DEVICE=0
INSTALL=0
FEATURES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)       PROFILE="release"; shift ;;
    --features)      FEATURES+=("$2"); shift 2 ;;
    --abi)           ABI="$2"; shift 2 ;;
    --manifest)      MANIFEST="$2"; shift 2 ;;
    --ohos-app-dir)  OHOS_APP_DIR="$2"; shift 2 ;;
    --bin)           BIN="$2"; shift 2 ;;
    --device)        DEVICE=1; shift ;;
    --install)       INSTALL=1; shift ;;
    -h|--help)       usage; exit 0 ;;
    -*)              die "unknown argument: $1 (try --help)" ;;
    *)
      if [[ -z "$CRATE" ]]; then CRATE="$1"
      elif [[ -z "$PLATFORM" ]]; then PLATFORM="$1"
      else die "unexpected extra argument: $1"
      fi
      shift ;;
  esac
done

[[ -n "$CRATE" ]] || die "<crate> is required"
[[ -n "$PLATFORM" ]] || die "<platform> is required (try --help)"

# ------------------------------------------------------------ crate inspection

METADATA="$(cargo metadata --no-deps --format-version 1 2>/dev/null || true)"

# The argument is usually an *example directory* name (`just build floom2-android
# android`), which does not always equal the Cargo package name (that folder's
# package is `floom2-android-example`). Accept either spelling and resolve to
# the package + its directory.
CRATE_DIR=""
resolve_crate() {
  [[ -n "$METADATA" ]] || return 1
  command -v python3 >/dev/null 2>&1 || return 1
  printf '%s' "$METADATA" | python3 -c '
import json, os, sys
want = sys.argv[1]
d = json.load(sys.stdin)
for p in d["packages"]:
    d0 = os.path.dirname(p["manifest_path"])
    if p["name"] == want or os.path.basename(d0) == want:
        print(p["name"] + "\t" + d0)
        raise SystemExit(0)
raise SystemExit(1)
' "$CRATE" 2>/dev/null
}

if resolved="$(resolve_crate)"; then
  CRATE="${resolved%%$'\t'*}"
  CRATE_DIR="${resolved#*$'\t'}"
else
  die "no workspace package or example directory named '$CRATE'"
fi
info "crate=$CRATE dir=${CRATE_DIR#$ROOT/}"

crate_has_feature() {
  [[ -n "$METADATA" ]] || return 1
  command -v python3 >/dev/null 2>&1 || return 1
  printf '%s' "$METADATA" | python3 -c '
import json, sys
pkg, feat = sys.argv[1], sys.argv[2]
d = json.load(sys.stdin)
for p in d["packages"]:
    if p["name"] == pkg:
        raise SystemExit(0 if feat in p.get("features", {}) else 1)
raise SystemExit(1)
' "$CRATE" "$1" 2>/dev/null
}

crate_target_name() {
  # $1 = target kind substring ("bin" or "lib"/"cdylib")
  [[ -n "$METADATA" ]] || return 1
  command -v python3 >/dev/null 2>&1 || return 1
  printf '%s' "$METADATA" | python3 -c '
import json, sys
pkg, kind = sys.argv[1], sys.argv[2]
d = json.load(sys.stdin)
for p in d["packages"]:
    if p["name"] != pkg:
        continue
    for t in p["targets"]:
        if any(kind in k for k in t["kind"]):
            print(t["name"])
            raise SystemExit(0)
raise SystemExit(1)
' "$CRATE" "$1" 2>/dev/null
}

# Default features: explicit --features wins; else `<platform>`; else `real`.
if [[ ${#FEATURES[@]} -eq 0 ]]; then
  if crate_has_feature "$PLATFORM"; then
    FEATURES=("$PLATFORM")
  elif crate_has_feature "real"; then
    FEATURES=("real")
  fi
fi

PROFILE_FLAG=()
[[ "$PROFILE" == "release" ]] && PROFILE_FLAG=(--release)
FEATURE_FLAG=()
for f in ${FEATURES[@]+"${FEATURES[@]}"}; do
  FEATURE_FLAG+=(--features "$f")
done

tier="artifact"
artifact="-"
note=""

enable_ohos_path() {
  export DEVECO_SDK_HOME="${DEVECO_SDK_HOME:-$HOME/.local/share/ohos}"
  # Version dirs (26.0.0, ...) are globbed rather than pinned so an SDK bump
  # does not silently break the build.
  local toolchains native
  toolchains="$(ls -1d "$DEVECO_SDK_HOME"/*/toolchains 2>/dev/null | sort -V | tail -1)"
  export PATH="$DEVECO_SDK_HOME/command-line-tools/bin${toolchains:+:$toolchains}:$PATH"

  # hvigor resolves the SDK from `sdk.dir` (local.properties) or this env var.
  # The version dirs (26.0.0/...) hang directly off DEVECO_SDK_HOME; pointing
  # this at `command-line-tools/sdk/default` instead makes hvigor reject the
  # layout with a "The SDK management mode has changed" configuration error.
  export OHOS_BASE_SDK_HOME="${OHOS_BASE_SDK_HOME:-$DEVECO_SDK_HOME}"
  [[ -d "$OHOS_BASE_SDK_HOME" ]] || unset OHOS_BASE_SDK_HOME

  # Link with the OHOS NDK clang, not the host `cc`: the host linker rejects the
  # GNU-style flags rustc emits for this target (`--version-script`, `-Bstatic`,
  # `-z relro`), which is the "link gate" documented in examples/ohos/README.md.
  native="$(ls -1d "$DEVECO_SDK_HOME"/*/native 2>/dev/null | sort -V | tail -1)"
  if [[ -n "$native" && -x "$native/llvm/bin/aarch64-unknown-linux-ohos-clang" ]]; then
    export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_OHOS_LINKER="$native/llvm/bin/aarch64-unknown-linux-ohos-clang"
    export OHOS_SYSROOT="$native/sysroot"
  fi
}

case "$PLATFORM" in
  macos)
    info "cargo build -p $CRATE ${PROFILE_FLAG[*]:-} ${FEATURE_FLAG[*]:-}"
    cargo build -p "$CRATE" ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}
    [[ -n "$BIN" ]] || BIN="$(crate_target_name bin || true)"
    [[ -n "$BIN" ]] || BIN="$CRATE"
    artifact="$ROOT/target/$PROFILE/$BIN"
    ;;

  android)
    APK_ARGS=(-p "$CRATE" -t "$ABI" "--$PROFILE" --features "${FEATURES[0]:-}")
    [[ -n "$MANIFEST" ]] && APK_ARGS+=(--manifest "$MANIFEST")
    (( INSTALL )) && APK_ARGS+=(--install)
    # If no features resolved, drop the empty --features.
    if [[ -z "${FEATURES[0]:-}" ]]; then
      APK_ARGS=(-p "$CRATE" -t "$ABI" "--$PROFILE")
      [[ -n "$MANIFEST" ]] && APK_ARGS+=(--manifest "$MANIFEST")
      (( INSTALL )) && APK_ARGS+=(--install)
    fi
    artifact="$("$SCRIPT_DIR/build-apk.sh" "${APK_ARGS[@]}" | tee /dev/stderr | sed -n 's/^ARTIFACT=//p' | tail -1)"
    [[ -n "$artifact" ]] || die "build-apk.sh did not report an artifact"
    ;;

  ios)
    IOS_ARGS=("$CRATE" "--$PROFILE")
    for f in ${FEATURES[@]+"${FEATURES[@]}"}; do IOS_ARGS+=(--features "$f"); done
    [[ -n "$BIN" ]] && IOS_ARGS+=(--bin "$BIN")
    (( DEVICE )) && IOS_ARGS+=(--device)
    artifact="$("$SCRIPT_DIR/ios-bundle.sh" "${IOS_ARGS[@]}" | tee /dev/stderr | sed -n 's/^ARTIFACT=//p' | tail -1)"
    [[ -n "$artifact" ]] || die "ios-bundle.sh did not report an artifact"
    (( DEVICE )) && { tier="blocked"; note="device build needs a real signing identity"; }
    ;;

  ohos)
    enable_ohos_path
    command -v hvigorw >/dev/null || { tier="blocked"; note="hvigorw not found under \$DEVECO_SDK_HOME"; }
    [[ -n "$OHOS_APP_DIR" ]] || OHOS_APP_DIR="$CRATE_DIR/ohos-app"

    if [[ "$tier" == "artifact" ]]; then
      command -v cargo >/dev/null || die "cargo not found on PATH"
      rustup target list --installed 2>/dev/null | grep -qx "aarch64-unknown-linux-ohos" \
        || die "rust target aarch64-unknown-linux-ohos not installed. Run: rustup target add aarch64-unknown-linux-ohos"
      info "cargo build -p $CRATE --target aarch64-unknown-linux-ohos ${PROFILE_FLAG[*]:-}"
      cargo build -p "$CRATE" --target aarch64-unknown-linux-ohos \
        ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}

      LIB="$(crate_target_name lib || true)"
      [[ -n "$LIB" ]] || LIB="${CRATE//-/_}"
      SO_SRC="$ROOT/target/aarch64-unknown-linux-ohos/$PROFILE/lib$LIB.so"
      [[ -f "$SO_SRC" ]] || { tier="blocked"; note="expected $SO_SRC — is the crate a cdylib?"; }

      [[ -d "$OHOS_APP_DIR" ]] || { tier="blocked"; note="OHOS project not found: $OHOS_APP_DIR"; }
    fi

    if [[ "$tier" == "artifact" ]]; then
      # The linked `.so` is itself a real (partial) deliverable: report it as the
      # artifact even if the HAP step below cannot finish.
      artifact="$SO_SRC"
      # Two destinations, both required, and they must stay byte-identical:
      #   * src/main/cpp/libs/ — what CMakeLists.txt links against (libentry.so
      #     records `DT_NEEDED libohos.so`);
      #   * entry/libs/ — what hvigor actually packs into the HAP. Without it
      #     `assembleHap` still succeeds and emits a HAP with no native runtime,
      #     which then fails at load. A *differently built* copy in both places
      #     trips hvigor's "Duplicated files found in module entry".
      DEST_DIR="$OHOS_APP_DIR/entry/src/main/cpp/libs/arm64-v8a"
      PACK_DIR="$OHOS_APP_DIR/entry/libs/arm64-v8a"
      mkdir -p "$DEST_DIR" "$PACK_DIR"
      cp "$SO_SRC" "$DEST_DIR/lib$LIB.so"
      cp "$SO_SRC" "$PACK_DIR/lib$LIB.so"
      info "hvigorw assembleHap"
      if (cd "$OHOS_APP_DIR" && hvigorw --no-daemon clean assembleHap); then
        HAP="$OHOS_APP_DIR/entry/build/default/outputs/default/entry-default-unsigned.hap"
        if [[ -f "$HAP" ]]; then
          artifact="$HAP"
          note="HAP is unsigned"
        else
          tier="blocked"; note="hvigorw succeeded but produced no HAP at $HAP"
        fi
      else
        tier="blocked"
        note="hvigorw failed (see its output above); linked .so staged at $DEST_DIR/lib$LIB.so"
      fi
    fi
    ;;

  web)
    rustup target list --installed 2>/dev/null | grep -qx "wasm32-unknown-unknown" \
      || die "rust target wasm32-unknown-unknown not installed. Run: rustup target add wasm32-unknown-unknown"
    info "cargo build -p $CRATE --target wasm32-unknown-unknown ${PROFILE_FLAG[*]:-}"
    cargo build -p "$CRATE" --target wasm32-unknown-unknown \
      ${PROFILE_FLAG[@]+"${PROFILE_FLAG[@]}"} ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}

    WASM_NAME="$(crate_target_name bin || true)"
    [[ -n "$WASM_NAME" ]] || WASM_NAME="$(crate_target_name lib || true)"
    [[ -n "$WASM_NAME" ]] || WASM_NAME="${CRATE//-/_}"
    WASM="$ROOT/target/wasm32-unknown-unknown/$PROFILE/$WASM_NAME.wasm"
    [[ -f "$WASM" ]] || die "expected $WASM after the wasm build"

    if command -v wasm-bindgen >/dev/null 2>&1; then
      DIST="$ROOT/target/web/$CRATE"
      rm -rf "$DIST"; mkdir -p "$DIST"
      info "wasm-bindgen --target web"
      wasm-bindgen --target web --out-dir "$DIST" --no-typescript "$WASM"
      artifact="$DIST"
    else
      tier="check-only"
      artifact="$WASM"
      note="raw .wasm only; install wasm-bindgen-cli for JS bindings"
    fi
    ;;

  windows|linux)
    if [[ "$PLATFORM" == "windows" ]]; then TARGET="x86_64-pc-windows-msvc"; else TARGET="x86_64-unknown-linux-gnu"; fi
    if rustup target list --installed 2>/dev/null | grep -qx "$TARGET"; then
      info "cargo check -p $CRATE --target $TARGET"
      cargo check -p "$CRATE" --target "$TARGET" \
        ${FEATURE_FLAG[@]+"${FEATURE_FLAG[@]}"}
      note="compiles for $TARGET; no binary produced (check-only tier)"
    else
      note="target $TARGET not installed — run: rustup target add $TARGET"
    fi
    tier="check-only"
    ;;

  tui)
    die "platform 'tui' is not implemented (no TUI backend exists; deferred)"
    ;;

  *)
    die "unknown platform '$PLATFORM' (macos|android|ios|ohos|web|windows|linux)"
    ;;
esac

if [[ -n "$note" ]]; then
  warn "$note"
fi
echo "TIER=$tier PLATFORM=$PLATFORM CRATE=$CRATE ARTIFACT=$artifact"
