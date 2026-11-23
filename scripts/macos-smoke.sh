#!/usr/bin/env bash
# Run the macOS gate probe in a real window and check what it reported.
#
# The desktop half of the "tested" claim: a window has to open, get a surface,
# and reach the OS clipboard, because those are the three things that only work
# on the OS and never in a headless test. The example prints one line per check
# and exits on the watchdog, so the assertion is on its stdout rather than on a
# screenshot — nothing here depends on the runner's screen size.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SECS="${SECS:-20}"
LOG="${LOG:-$(mktemp)}"

cd "$ROOT"
cargo build --package=floom-macos --features real --locked

set +e
FLOOM2_MACOS_EXIT_SECS="$SECS" ./target/debug/floom-macos > "$LOG" 2>&1
set -e

echo "--- $LOG"
cat "$LOG"

fail=0
want() {
    if grep -qF "$1" "$LOG"; then
        echo "ok: $2"
    else
        echo "MISSING: $2 (expected a line containing: $1)" >&2
        fail=1
    fi
}

# A window, a surface, and a frame: at scale 2 a 480×320 point window is 960×640
# pixels, so this cannot pass by inventing numbers.
want "floom-macos: window surface " "a window opened and reported its surface"
want "matched=true" "the system clipboard round-tripped through NSPasteboard"
want "floom-macos: focus None -> TextInput@" "focus moved into the text field"

if [ "$fail" -ne 0 ]; then
    echo "the gate probe ran but did not report everything it must" >&2
    exit 1
fi

echo "ok: the macOS probe opened a window and reached the OS"
