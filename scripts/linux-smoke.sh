#!/usr/bin/env bash
# The Linux window check: no GPU, no display, no human.
#
# `floom-window-software` opens a real X11 window through winit and draws with the
# software renderer. Under `xvfb-run` that is a window server, so what this proves
# is the window lifecycle and the present path — not the GPU path, which
# `scripts/` can only cover where an adapter exists.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${LOG:-$(mktemp)}"
FRAMES="${FRAMES:-3}"

cd "$ROOT"
cargo build --package=floom-window-software --locked

set +e
FLOOM_SOFTWARE_EXIT_FRAMES="$FRAMES" xvfb-run -a ./target/debug/floom-window-software > "$LOG" 2>&1
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

# A frame with no ink means the surface was presented empty — a white window with
# nothing in it passes "a frame arrived" but fails the only thing that matters.
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
