#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# Size budget gate — the size half of "performance, size and scale" in
# `docs/release-policy.md`.
#
# The release policy records three binary sizes and says "a release that grows
# the headless floor by more than 10 % without a written reason is a question".
# A question nobody asks is not a check. This script builds each tier into
# `target/release`, measures the binary, and fails when any tier is more than
# `threshold_pct` over the checked-in baseline.
#
# Unlike the Criterion benchmarks, sizes are not noisy run to run, so there is
# no best-of-N: one release build per tier is enough (and cached after the
# first). Like the perf baseline, these are single-host numbers — a local check,
# not a PR merge gate.
#
# Usage:
#   scripts/check-size-budgets.sh            # build each tier, compare
#   scripts/check-size-budgets.sh --update  # rebuild and rewrite the baseline
#
# Exit codes: 0 = within budget, 1 = a tier regressed, 2 = setup error (a build
# failed, the binary is missing, or the baseline is missing). 2 means the gate
# could not run; it must never be read as a pass.

set -euo pipefail

cd "$(dirname "$0")/.."

BASELINE="docs/superpowers/benchmarks/floom-2-size-baseline.json"
UPDATE=0
for arg in "$@"; do
  case "$arg" in
    --update) UPDATE=1 ;;
    *) echo "unknown argument: $arg" >&2; exit 2 ;;
  esac
done

if ! command -v python3 >/dev/null; then
  echo "python3 is required" >&2
  exit 2
fi

HOST="$(uname -s) $(uname -m), rustc $(rustc --version 2>/dev/null | awk '{print $2}')"

# `--update` can rebuild a deleted baseline from the tier list below. The
# committed baseline is authoritative; this fallback only exists so a missing
# file is recoverable without hand-writing JSON.
if [ ! -f "$BASELINE" ]; then
  if [ "$UPDATE" -eq 1 ]; then
    mkdir -p "$(dirname "$BASELINE")"
    python3 - "$BASELINE" <<'PY'
import json, sys
path = sys.argv[1]
tiers = [
    {"id": "headless-floor", "package": "floom-native-plugin",
     "bin": "floom-native-plugin", "features": [], "baseline_bytes": 0},
    {"id": "desktop-gpu", "package": "floom-window",
     "bin": "floom-window", "features": ["real"], "baseline_bytes": 0},
    {"id": "full-app", "package": "floom-showcase",
     "bin": "floom-showcase", "features": [], "baseline_bytes": 0},
]
json.dump({"schema": 1, "threshold_pct": 10.0, "generated": "", "host": "",
           "note": "seed by check-size-budgets.sh --update", "tiers": tiers},
          open(path, "w"), indent=2)
open(path, "a").write("\n")
PY
    echo "created skeleton baseline: $BASELINE" >&2
  else
    echo "size budget baseline missing: $BASELINE" >&2
    echo "restore it from version control, or run: bash $0 --update" >&2
    exit 2
  fi
fi

python3 - "$BASELINE" "$UPDATE" "$HOST" <<'PY'
import json, os, subprocess, sys

baseline_path, update, host = sys.argv[1], sys.argv[2] == "1", sys.argv[3]
data = json.load(open(baseline_path))
threshold = data.get("threshold_pct", 10.0)


def build(pkg, features):
    cmd = ["cargo", "build", "--release",
           "--manifest-path", f"examples/{pkg}/Cargo.toml"]
    if features:
        cmd += ["--features", features]
    r = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    if r.returncode != 0:
        sys.stderr.write(r.stdout)
        sys.exit(2)


rows, failures = [], []
for t in data["tiers"]:
    pkg = t["package"]
    bin_name = t.get("bin", pkg)
    features = ",".join(t.get("features", []))
    build(pkg, features)
    path = os.path.join("target", "release", bin_name)
    if not os.path.isfile(path):
        print(f"size gate: expected binary missing after build: {path}", file=sys.stderr)
        sys.exit(2)
    measured = os.path.getsize(path)
    base = t["baseline_bytes"]
    delta = (measured - base) / base * 100.0 if base else 0.0
    rows.append((t["id"], measured, base, delta, threshold))
    if update:
        t["baseline_bytes"] = measured
    elif delta > threshold:
        failures.append(
            f"{t['id']}: {measured} B vs baseline {base} B = +{delta:.1f}% "
            f"(budget +{threshold:.0f}%)"
        )

print(f"size budget — host {host}")
print(f"{'tier':22s} {'bytes':>12s} {'baseline':>12s} {'delta':>8s}  {'budget':>6s}")
print("-" * 70)
for tid, measured, base, delta, budget in rows:
    print(f"{tid:22s} {measured:12d} {base:12d} {delta:+7.1f}%   +{budget:.0f}%")

if update:
    data["generated"] = subprocess.run(
        ["date", "+%Y-%m-%d"], capture_output=True, text=True).stdout.strip()
    data["host"] = host
    with open(baseline_path, "w") as f:
        json.dump(data, f, indent=2)
        f.write("\n")
    print(f"\nbaseline rewritten: {baseline_path}")
    sys.exit(0)

if failures:
    print("\nFAIL — sizes over budget:")
    for f in failures:
        print(f"  x {f}")
    sys.exit(1)
print("\nPASS — every tier within budget")
PY
