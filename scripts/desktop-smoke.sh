#!/usr/bin/env bash
# The window check for machines with no GPU: no display either, no human.
#
# `floom-window-software` opens a real window through winit and draws with the
# software renderer. On Linux the window server comes from `xvfb-run`; on Windows
# the runner has a desktop session, so the binary runs directly. What this proves
# is the window lifecycle and the present path — not the GPU path, which is
# covered where an adapter exists (`gpu-pixels`, `gpu-pixels-lavapipe`).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${LOG:-$(mktemp)}"
FRAMES="${FRAMES:-8}"

cd "$ROOT"
cargo build --package=floom-window-software --locked

# `xvfb-run` when it exists (Linux CI), the runner's own desktop otherwise. The
# two branches are spelled out because `"${arr[@]}"` on an empty array is an
# error under `set -u` in the bash macOS ships.
set +e
if command -v xvfb-run > /dev/null; then
    FLOOM_SOFTWARE_EXIT_FRAMES="$FRAMES" xvfb-run -a ./target/debug/floom-window-software > "$LOG" 2>&1
else
    FLOOM_SOFTWARE_EXIT_FRAMES="$FRAMES" ./target/debug/floom-window-software > "$LOG" 2>&1
fi
status=$?
set -e

echo "--- $LOG"
cat "$LOG"

if [ "$status" -ne 0 ]; then
    echo "the software window left with status $status" >&2
    exit 1
fi

fail=0
want() {
    if grep -qF "$1" "$LOG"; then
        echo "ok: $2"
    else
        echo "MISSING: $2 (expected a line containing: $1)" >&2
        fail=1
    fi
}

want "floom-software: window surface " "a window opened and reported its surface"
want "inked pixels" "a frame reached the window"

# The first frame arrives before the tree has painted anything (on a runner the
# window appears, then the tree paints), so the assertion is over all frames: "no
# frame ever carried ink" is the failure. A window presented empty passes "a frame
# arrived" and fails the only thing that matters.
if grep -E "floom-software: frame .* with [1-9][0-9]* inked pixels" "$LOG" > /dev/null; then
    echo "ok: a frame carried ink"
else
    echo "MISSING: not one frame drew any pixels" >&2
    fail=1
fi

if [ "$fail" -ne 0 ]; then
    echo "the software window ran but did not report everything it must" >&2
    exit 1
fi

echo "ok: the software window opened, drew, and left"
