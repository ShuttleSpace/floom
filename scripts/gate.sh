#!/usr/bin/env bash
# The gate, run where the code actually lives.
#
# Why this exists: the checks in `.github/workflows/ci.yml` never ran. `origin`
# is Gitee, and GitHub Actions does not execute there, so every job in that file
# — `fmt`, `clippy`, `build`, `android-check`, `ios-check`, `probe-check` — was
# a claim with no execution behind it. A check that cannot fail is worse than no
# check, because it reads like evidence.
#
# This script is the same checks, runnable on the machine that holds the code.
# It is deliberately *not* shaped like the CI graph: there, `build` sits behind
# `needs: [fmt, clippy]`, so one red formatting diff silently skipped every test.
# Here each check runs and reports on its own, and the exit status is the number
# of failures — a skip is printed as a skip, never folded into a pass.
#
# Usage:
#   scripts/gate.sh              # everything that can run on this host
#   scripts/gate.sh fmt test     # only the named checks
#   scripts/gate.sh --list       # what the names are
#
# Every check prints one line: PASS, FAIL, or SKIP with the reason. `--verbose`
# passes the underlying command's output through instead of summarising it.

set -uo pipefail

# The gate only needs pass/fail, not debuggable binaries, and the two big
# workspaces here are 306 and 4 members. Measured on this machine:
#
#   cargo test --workspace, full debuginfo      >109 min (the first attempt was
#                                                 killed by a full disk before it
#                                                 finished)
#   same, debuginfo off, cold target              273 s
#   same, debuginfo off, warm target               68 s
#
# The target directory also shrinks from ~46 GB to 3.2 GB, which matters on a
# disk that hit 100% during this work and took every build down with it. Set
# here rather than in a `[profile]` so a normal `cargo test` outside the gate
# keeps its debuginfo.
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
export CARGO_INCREMENTAL=0

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

VERBOSE=0
CHECKS=()
for arg in "$@"; do
  case "$arg" in
    --verbose|-v) VERBOSE=1 ;;
    --list|-l) CHECKS=(--list) ;;
    --help|-h)
      sed -n '2,30p' "$0" | sed 's/^# \{0,1\}//'
      exit 0
      ;;
    *) CHECKS+=("$arg") ;;
  esac
done

# The satellite workspaces, derived rather than listed.
#
# There are three workspaces in this repo, not one: the root, plus every
# `Cargo.toml` directly under `crates/ecosystem/` that opens with `[workspace]`.
# The root manifest's `members` is `crates/*` and `examples/*` with
# `crates/ecosystem` excluded, so root `cargo fmt --all` and `cargo test
# --workspace` stop at that boundary and never reach a satellite.
#
# Derived, because a hand-written list is exactly how one of them was missed.
# `crates/ecosystem/floom-editor` was in the repo for a while before this
# existed: its committed sources were not rustfmt-clean and its 25 tests were run
# by nothing, while `fmt` printed PASS claiming "both workspaces". There were
# three.
SATELLITE_WORKSPACES=()
for _manifest in crates/ecosystem/*/Cargo.toml; do
  [ -f "$_manifest" ] || continue
  grep -qx '\[workspace\]' "$_manifest" && SATELLITE_WORKSPACES+=("$(dirname "$_manifest")")
done
unset _manifest

# The 2.0 track, as the CI job spells it. A v1 regression must not red the
# signal for work that never touches it, and vice versa.
FLOOM2_PACKAGES=(
  --package=floom-types --package=floom-core
  --package=floom-runtime-core --package=floom-scheduler
  --package=floom-scene
  --package=floom-renderer --package=floom-renderer-vello
  --package=floom-renderer-software --package=floom-renderer-skia-abi
  --package=floom-renderer-tui --package=floom-renderer-web
  --package=floom-platform --package=floom-platform-desktop
  --package=floom-platform-android
  --package=floom-platform-ios --package=floom-platform-harmony
  --package=floom-platform-web
  --package=floom-platform-embedded --package=floom-platform-mobile
  --package=floom-text --package=floom-editor --package=floom-ime
  --package=floom-layout --package=floom-style --package=floom-virtual
  --package=floom-native --package=floom-native-ffi
  # The plugins are part of the 2.0 track, not an extra: each one is a crate
  # this branch added, and their `cfg` arms are where the platform work lives.
  # Leaving them out of the scoped clippy run is how `provider()` kept a raw
  # pointer parameter without an `unsafe` signature while the gate stayed green.
  --package=floom-plugin-nativeview
  --package=floom-plugin-battery --package=floom-plugin-connectivity
  --package=floom-plugin-device-info --package=floom-plugin-local-auth
  --package=floom-plugin-location --package=floom-plugin-notifications
  --package=floom-plugin-sensors --package=floom-plugin-share
  --package=floom-plugin-vibration --package=floom-plugin-package-info
  --package=floom-plugin-paths
  --package=floom-plugin-open-url
  # The framework tail of the track. It was missing here while CI listed it, so
  # the gate could go green with `floom-app` and `floom-widgets` unclipped.
  --package=floom-accessibility --package=floom-focus --package=floom-gesture
  --package=floom-animation --package=floom-overlay --package=floom-store
  --package=floom-task --package=floom-forms --package=floom-navigation
  --package=floom-persistence --package=floom-services --package=floom-error
  --package=floom-widgets --package=floom-app --package=floom-testkit
)

# The capability plugins, one crate each. `android-check` and `ios-check` compile
# them for a device target because that is the only place their `cfg` arms exist;
# a host build skips exactly the code most likely to rot.
#
# `floom-plugin-nativeview` is deliberately absent from the mobile lists: it is
# a macOS/Objective-C plugin whose whole body is AppKit, so there is no Android
# or iOS arm for a device check to reach. Listing it there would fail on a
# missing package rather than on missing code.
CAPABILITY_PLUGINS=(
  floom-plugin-battery
  floom-plugin-connectivity
  floom-plugin-device-info
  floom-plugin-local-auth
  floom-plugin-location
  floom-plugin-nativeview
  floom-plugin-notifications
  floom-plugin-sensors
  floom-plugin-share
  floom-plugin-vibration
  floom-plugin-package-info
  floom-plugin-paths
  floom-plugin-open-url
)

MOBILE_CAPABILITY_PLUGINS=(
  floom-plugin-battery
  floom-plugin-connectivity
  floom-plugin-device-info
  floom-plugin-local-auth
  floom-plugin-location
  floom-plugin-notifications
  floom-plugin-sensors
  floom-plugin-share
  floom-plugin-vibration
  floom-plugin-package-info
  floom-plugin-paths
  floom-plugin-open-url
)

# The plugins with real Windows arms. Everything else in the capability list
# compiles for a Windows target through its inert default arm, so checking it
# there proves nothing about code that ships. These are different: the Windows
# arm *is* the product (`ShellExecuteW`, the known folders through `dirs`), and
# an arm no check ever compiles is how `floom-plugin-vibration`'s iOS half kept
# two edition-2024 errors for as long as it did.
WINDOWS_CAPABILITY_PLUGINS=(
  floom-plugin-paths
  floom-plugin-open-url
)

PASS=0
FAIL=0
SKIP=0
FAILED_NAMES=()

# run_check <name> <skip-reason-or-empty> <command...>
#
# An empty skip reason means "run it". A non-empty one is printed verbatim, so a
# skipped check always says why — the whole point is that nobody mistakes a skip
# for a pass.
run_check() {
  local name="$1" skip="$2"
  shift 2

  if [ -n "$skip" ]; then
    printf 'SKIP  %-22s %s\n' "$name" "$skip"
    SKIP=$((SKIP + 1))
    return 0
  fi

  local output status
  output="$("$@" 2>&1)"
  status=$?

  if [ "$status" -eq 0 ]; then
    printf 'PASS  %-22s\n' "$name"
    PASS=$((PASS + 1))
  else
    printf 'FAIL  %-22s\n' "$name"
    FAIL=$((FAIL + 1))
    FAILED_NAMES+=("$name")
    if [ "$VERBOSE" -eq 1 ]; then
      printf '%s\n' "$output" | tail -60
    else
      # The last few lines are usually the actual error; the middle is cargo
      # telling us what it is doing.
      printf '%s\n' "$output" | grep -E '^(error|Error|FAILED|Diff in|assertion|thread)' | head -12
    fi
  fi
}

has_check() {
  [ "${#CHECKS[@]}" -eq 0 ] && return 0
  local wanted="$1" c
  for c in "${CHECKS[@]}"; do
    [ "$c" = "$wanted" ] && return 0
  done
  return 1
}

if [ "${CHECKS[0]:-}" = "--list" ]; then
  cat <<'EOF'
fmt        cargo fmt --all --check, in every workspace
clippy     cargo clippy on the 2.0 track, warnings denied
test       cargo test --workspace
ecosystem  cargo test --workspace in every crates/ecosystem satellite
doc        cargo test --doc
arch       scripts/check-architecture-deps.sh
probe      the device probe compiles with `--features real`
plugins    the capability plugins compile on the host
android    the plugins compile for aarch64-linux-android
ios        the plugins compile for aarch64-apple-ios
windows    the plugins with real Windows arms compile for x86_64-pc-windows-msvc
golden     the pixel-golden examples
EOF
  exit 0
fi

echo "gate: $(uname -s) $(uname -m), $(date '+%Y-%m-%d %H:%M')"
echo

# --- formatting -------------------------------------------------------------
#
# Every workspace in the repo, not just the root. `crates/ecosystem/floom-ui`
# and `crates/ecosystem/floom-editor` each have their own Cargo.toml, and root
# `cargo fmt --all` reaches neither.
#
# `--all` inside a satellite, which is also what the CI job runs. It does reach
# that satellite's local path dependencies, and those point back into the root
# workspace, so a root file that is unformatted is reported twice. Harmless for
# `--check` — the gate never runs the writing form, and `cargo fmt --all` here is
# scoped by the working directory, so it cannot rewrite root sources from a
# satellite. Scoping to `--package=` per member was tried and dropped: the
# package names have to come from `cargo metadata`, and a grep over that JSON
# yields target and dependency names too (`gallery`, `paint`, `ide_demo`), which
# fails the check for a reason that has nothing to do with formatting.
fmt_check() {
  local ws status=0
  cargo fmt --all --check || status=1
  for ws in "${SATELLITE_WORKSPACES[@]}"; do
    (cd "$ws" && cargo fmt --all --check) || status=1
  done
  return "$status"
}
has_check fmt && run_check fmt "" fmt_check

# --- clippy -----------------------------------------------------------------
#
# Scoped to the 2.0 track on purpose. The v1 crates carry pre-existing warnings
# that predate this work; denying them here would report a red gate that says
# nothing about the code being changed.
#
# `--all-features` because the default feature set is not the shipped one:
# `floom-platform-desktop` keeps its whole winit backend behind a feature, and
# a default-feature clippy run checks the headless placeholder instead. The
# first run with `--all-features` found seven errors in code no earlier check
# had ever compiled.
clippy_check() {
  cargo clippy "${FLOOM2_PACKAGES[@]}" --all-features -- --deny warnings
}
has_check clippy && run_check clippy "" clippy_check

# --- tests ------------------------------------------------------------------
#
# `--exclude floom-platform` because that crate's tests need a live display and
# a window; everything else runs headless.
test_check() {
  cargo test --workspace --exclude floom-platform
}
has_check test && run_check test "" test_check

# --- the satellite workspaces -----------------------------------------------
#
# Root `members` is `crates/*` and `examples/*` with `crates/ecosystem` excluded,
# so the `test` check above never reaches a satellite. That gap is not academic.
# The four design-system pixel goldens live in `crates/ecosystem/floom-ui`, and
# they were stale for two commits because nothing ran them: `d131b5af` moved
# `PixelSnapshot::of` behind a trait and taught vello to draw drop shadows, so
# the galleries failed to compile *and* hashed differently, while every check
# here printed PASS. `crates/ecosystem/floom-editor` was in the same position —
# 25 paint-level tests, committed, passing when run by hand, run by nothing.
#
# The whole workspace, not a `-p` list: the clippy scope was truncated once
# already, and a hand-written package list is how a third of a track goes
# unlinted while the gate prints PASS. 306 members is the real shape of the
# `floom-ui` tree, so that is what this runs.
#
# No `--exclude`: every member is a headless test. The galleries render through
# the software renderer into a buffer, so no display and no window is involved —
# unlike the root's `floom-platform`, which is why that one is excluded above.
#
# `--no-fail-fast` per workspace so one red kit does not hide the rest, and the
# loop keeps going so a red `floom-ui` does not hide `floom-editor`.
ecosystem_check() {
  local ws status=0
  for ws in "${SATELLITE_WORKSPACES[@]}"; do
    (cd "$ws" && cargo test --workspace --no-fail-fast) || status=1
  done
  return "$status"
}
has_check ecosystem && run_check ecosystem "" ecosystem_check

doc_check() {
  cargo test --doc --no-fail-fast
}
has_check doc && run_check doc "" doc_check

# --- architecture -----------------------------------------------------------
has_check arch && run_check arch "" ./scripts/check-architecture-deps.sh

# --- the probe --------------------------------------------------------------
#
# macOS only: most of the probe is `#[cfg(target_os = "macos")]`, and a Linux
# check would skip exactly the arms most likely to rot.
if [ "$(uname -s)" = "Darwin" ]; then
  probe_check() {
    cargo check -p floom-probe --features real
  }
  has_check probe && run_check probe "" probe_check
else
  has_check probe && run_check probe "needs macOS (the probe is a macOS native-view readback)" true
fi

# --- capability plugins -----------------------------------------------------
plugins_check() {
  local p
  for p in "${CAPABILITY_PLUGINS[@]}"; do
    cargo check -p "$p" || return 1
  done
}
has_check plugins && run_check plugins "" plugins_check

# Android: `cargo check --target`, exactly what the CI job runs. No NDK and no
# `cargo ndk` — `check` does not link, so the target's std is the only thing
# needed, and requiring the NDK here would turn a real check into a skip on any
# machine that has the target but not the toolchain bundle.
#
# The platform crates are checked too, because their `android.rs` arms are the
# other half of the same story: a plugin calls into the platform's JNI bridge.
android_check() {
  local p
  cargo check --target aarch64-linux-android \
    -p floom-platform-mobile -p floom-platform-android || return 1
  for p in "${MOBILE_CAPABILITY_PLUGINS[@]}"; do
    cargo check --target aarch64-linux-android -p "$p" || return 1
  done
}
if [ -d "$(rustc --print target-libdir --target aarch64-linux-android 2>/dev/null)" ]; then
  has_check android && run_check android "" android_check
else
  has_check android && run_check android "no aarch64-linux-android std in this toolchain (rustup target add aarch64-linux-android)" true
fi

# iOS: the std for aarch64-apple-ios lives only in the pinned toolchain here, so
# the probe below is a real capability test rather than a formality.
ios_check() {
  local p
  for p in "${MOBILE_CAPABILITY_PLUGINS[@]}"; do
    cargo check --target aarch64-apple-ios -p "$p" || return 1
  done
}
if [ -d "$(rustc --print target-libdir --target aarch64-apple-ios 2>/dev/null)" ]; then
  has_check ios && run_check ios "" ios_check
else
  has_check ios && run_check ios "no aarch64-apple-ios std in this toolchain (rustup target add aarch64-apple-ios)" true
fi

# Windows: the same gap, on the desktop nobody was looking at. There is no CI
# job for it either, so this check is the only thing standing between a Windows
# arm and the quiet rot the vibration iOS arm made famous. `check` does not
# link, so the MSVC toolchain itself is not needed — only the target's std.
windows_check() {
  local p
  for p in "${WINDOWS_CAPABILITY_PLUGINS[@]}"; do
    cargo check --target x86_64-pc-windows-msvc -p "$p" || return 1
  done
}
if [ -d "$(rustc --print target-libdir --target x86_64-pc-windows-msvc 2>/dev/null)" ]; then
  has_check windows && run_check windows "" windows_check
else
  has_check windows && run_check windows "no x86_64-pc-windows-msvc std in this toolchain (rustup target add x86_64-pc-windows-msvc)" true
fi

# --- goldens ----------------------------------------------------------------
has_check golden && run_check golden "" ./scripts/run-golden-examples.sh

echo
if [ "$FAIL" -eq 0 ]; then
  echo "gate: $PASS passed, $SKIP skipped, 0 failed"
  exit 0
fi
echo "gate: $PASS passed, $SKIP skipped, $FAIL failed: ${FAILED_NAMES[*]}"
exit "$FAIL"
