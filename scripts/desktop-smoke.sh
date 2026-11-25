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

# The agent protocol, against that same window: the binary asks itself the two
# questions an agent would (read the tree, tap a label) once it has painted.
# Same-thread, port-free: the TCP half has its own test in floom-accuracy, and a
# probe thread racing a frame loop is how this failed on a runner once.
AGENT_LOG="${AGENT_LOG:-$(mktemp)}"
set +e
if command -v xvfb-run > /dev/null; then
    xvfb-run -a ./target/debug/floom-window-software --agent-selftest > "$AGENT_LOG" 2>&1
else
    ./target/debug/floom-window-software --agent-selftest > "$AGENT_LOG" 2>&1
fi
agent_status=$?
set -e
echo "--- $AGENT_LOG"
cat "$AGENT_LOG"
if [ "$agent_status" -ne 0 ]; then
    echo "the agent probe left with status $agent_status" >&2
    exit 1
fi
LOG="$AGENT_LOG"
want "agent: tree has button Increment" "an agent read the live widget tree"
want "agent: tap Increment repainted=true" "an agent's tap reached the app and moved it"
want "floom-software: agent self-test passed" "the agent self-test left satisfied"
# --- a rebuild, in the loop that is already open -----------------------------
# The unit tests cover carry_state and remount separately; this is the only place
# the watcher, the in-place entry and remount run together, in a real window.
WATCH_LOG="${WORK:-${TMPDIR:-/tmp}}/floom-watch.log"
SRC="$ROOT/examples/floom-window-software/src/main.rs"
if command -v xvfb-run > /dev/null; then
    ( xvfb-run -a env FLOOM_WATCH_ROOT="$ROOT/examples/floom-window-software/src" \
        ./target/debug/floom-window-software --watch > "$WATCH_LOG" 2>&1 ) &
else
    ( FLOOM_WATCH_ROOT="$ROOT/examples/floom-window-software/src" \
        ./target/debug/floom-window-software --watch > "$WATCH_LOG" 2>&1 ) &
fi
watch_pid=$!
waited=0
while [ "$waited" -lt 30 ] && ! grep -q "rebuilding the tree" "$WATCH_LOG" 2> /dev/null; do
    sleep 1
    waited=$((waited + 1))
done
touch "$SRC"
waited=0
while [ "$waited" -lt 30 ] && [ "$(grep -c "rebuilding the tree" "$WATCH_LOG" || true)" -lt 2 ]; do
    sleep 1
    waited=$((waited + 1))
done
kill "$watch_pid" 2> /dev/null || true
wait "$watch_pid" 2> /dev/null || true
echo "--- $WATCH_LOG"
cat "$WATCH_LOG"
LOG="$WATCH_LOG"
want "floom-software: watching" "the app asked a watcher about its sources"
want "floom-software: rebuilding the tree" "the first tree came through the reload path"
rebuilds=$(grep -c "rebuilding the tree" "$WATCH_LOG" || true)
if [ "$rebuilds" -lt 2 ]; then
    echo "the source file changed but the tree was rebuilt $rebuilds time(s)" >&2
    fail=1
fi

exit "$fail"

if [ "$fail" -ne 0 ]; then
    echo "the software window ran but did not report everything it must" >&2
    exit 1
fi

echo "ok: the software window opened, drew, and left"
