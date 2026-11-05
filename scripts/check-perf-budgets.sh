#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Performance budget gate (Phase 31 / P4).
#
# The P0 acceptance line "baseline benchmarks pass" and the P4 line "performance
# budgets enforced in CI" both rested on a policy with no numbers behind it:
# `benches/memory_baseline.md` said CI must not fail on absolute Criterion times
# "until calibration numbers are checked in", and the numbers never were. This
# script is the enforcement half, and
# `docs/superpowers/benchmarks/floom-2-budget-baseline.json` is the calibration
# half.
#
# Two things it does that a naive `cargo bench | grep change` does not:
#
# * **Criterion's own default protocol**, never a shortened profile. The
#   2026-09-18 acceptance run measured `update_reuse` at +12% under a shortened
#   profile and +9.9% under the default one, on unchanged code.
# * **Best of N runs** (`--runs`, default 2), taking the *minimum* median per
#   bench. A single run is not stable enough for a 10% budget: `visible_range/
#   10000` read 444.3 / 404.8 / 408.4 ns across three consecutive runs of the
#   same binary. Scheduler contention and thermal state only ever make a
#   measurement *slower*, so the minimum is the robust estimate and the spread
#   is recorded in the baseline file rather than hidden.
#
# Usage:
#   bash scripts/check-perf-budgets.sh                 # measure and compare
#   bash scripts/check-perf-budgets.sh --runs 3        # more samples
#   bash scripts/check-perf-budgets.sh --update        # rewrite the baseline
#
# Deliberately not wired into CI yet: the baseline is one host's numbers and the
# budget is measured against that host. See the support matrix in
# docs/2.0/RELEASE-POLICY.md.

set -euo pipefail

cd "$(dirname "$0")/.."

BASELINE="docs/superpowers/benchmarks/floom-2-budget-baseline.json"
UPDATE=0
RUNS=2
while [ $# -gt 0 ]; do
    case "$1" in
        --update) UPDATE=1; shift ;;
        --runs) RUNS="$2"; shift 2 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

if ! command -v python3 >/dev/null; then
    echo "python3 is required" >&2
    exit 2
fi

# The (package, bench) pairs the budgets name, in file order.
PAIRS="$(python3 - "$BASELINE" <<'PY'
import json, sys
data = json.load(open(sys.argv[1]))
seen, out = set(), []
for b in data["budgets"]:
    key = (b["package"], b["bench"])
    if key not in seen:
        seen.add(key)
        out.append(key)
for pkg, bench in out:
    print(f"{pkg} {bench}")
PY
)"

SAMPLES="$(mktemp -d)"
trap 'rm -rf "$SAMPLES"' EXIT

i=1
while [ "$i" -le "$RUNS" ]; do
    echo "== run $i/$RUNS =="
    while read -r pkg bench; do
        [ -n "$pkg" ] || continue
        cargo bench -p "$pkg" --bench "$bench" >/dev/null
    done <<<"$PAIRS"
    python3 - "$BASELINE" "$SAMPLES" "$i" <<'PY'
import json, os, shutil, sys
baseline, dest, run = json.load(open(sys.argv[1])), sys.argv[2], sys.argv[3]
for b in baseline["budgets"]:
    src = os.path.join("target/criterion", b["id"], "new", "estimates.json")
    if not os.path.exists(src):
        continue
    out = os.path.join(dest, run, b["id"])
    os.makedirs(out, exist_ok=True)
    shutil.copyfile(src, os.path.join(out, "estimates.json"))
PY
    i=$((i + 1))
done

echo
python3 - "$BASELINE" "$SAMPLES" "$RUNS" "$UPDATE" <<'PY'
import json, os, sys

baseline_path, samples, runs, update = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4] == "1"
data = json.load(open(baseline_path))
threshold_default = data["threshold_pct"]

def fmt(ns):
    if ns < 1_000:
        return f"{ns:9.1f} ns"
    if ns < 1_000_000:
        return f"{ns / 1e3:9.2f} µs"
    return f"{ns / 1e6:9.3f} ms"

def samples_for(bid):
    """Every median measured for `bid`, one per run, sorted."""
    out = []
    for run in range(1, runs + 1):
        p = os.path.join(samples, str(run), bid, "estimates.json")
        if os.path.exists(p):
            out.append(json.load(open(p))["median"]["point_estimate"])
    return sorted(out)

rows, failures = [], []
for b in data["budgets"]:
    got = samples_for(b["id"])
    if not got:
        failures.append(f"{b['id']}: no measurement under {samples}")
        continue
    # Best of N: contention only slows a run down.
    measured = got[0]
    spread = (got[-1] - got[0]) / got[0] * 100.0 if len(got) > 1 else 0.0
    threshold = b.get("threshold_pct", threshold_default)
    if update:
        b["median_ns"] = measured
        if len(got) > 1:
            b["observed_spread_pct"] = round(spread, 1)
        rows.append((b["id"], measured, None, None, threshold, spread, len(got)))
        continue
    delta = (measured - b["median_ns"]) / b["median_ns"] * 100.0
    rows.append((b["id"], measured, delta, b["median_ns"], threshold, spread, len(got)))
    if delta > threshold:
        failures.append(
            f"{b['id']}: {fmt(measured)} vs baseline {fmt(b['median_ns'])} "
            f"= {delta:+.1f}% (budget {threshold:+.0f}%)"
        )

if update:
    with open(baseline_path, "w") as fh:
        json.dump(data, fh, indent=2)
        fh.write("\n")
    print(f"baseline rewritten (best of {runs}): {baseline_path}")

print(f"{'bench':34s} {'best-of-N':>12s} {'baseline':>12s} {'delta':>8s}  {'spread':>7s}")
print("-" * 80)
for bid, measured, delta, base, threshold, spread, n in rows:
    base_s = "(updated)" if base is None else fmt(base)
    delta_s = "  --  " if delta is None else f"{delta:+7.1f}%"
    print(f"{bid:34s} {fmt(measured):>12s} {base_s:>12s} {delta_s}  {spread:+6.1f}%")
print()

if failures:
    print(f"FAIL — {len(failures)} over budget:")
    for f in failures:
        print(f"  x {f}")
    sys.exit(1)

print(f"PASS — {len(rows)} budgets, best of {runs}, none over budget")
PY
