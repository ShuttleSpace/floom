# Memory / allocation baseline (Floem 2.0 Phase 0)

**Date:** 2026-09-04  
**Branch:** `floem-2.0`  
**Status:** Process notes only — automate once `floem-runtime-core` exists.

## Commands (desktop macOS/Linux)

```bash
# Criterion benches (HTML under target/criterion/)
cargo bench -p floem --bench runtime_baseline
cargo bench -p floem --bench scene_baseline
cargo bench -p floem --bench render_baseline

# Optional: instrument with Instruments / heaptrack on a single demo
cargo run -p counter --release
```

## What to record after each meaningful runtime change

| Metric | How | Budget (TBD after calibration) |
|---|---|---|
| Peak RSS idle after mount 1k labels | Instruments / `/usr/bin/time -l` | TBD |
| Allocations per signal update | `dhat` or Instruments Allocations | TBD |
| Frame paint of 1k labels | `render_baseline` Criterion | ≤ +10% vs this Phase 0 run |

## CI regression policy

Until calibration numbers are checked in under `docs/superpowers/benchmarks/`, CI **must not** fail on absolute Criterion times (too noisy across runners).

After calibration:

1. Store JSON summaries from `cargo bench -- --save-baseline floem2-p0`.
2. Compare with `--baseline floem2-p0` and fail on >10% for named hot benches only.
3. Wire that job in `.github/workflows/ci.yml` on `macos-latest` only.

## Known gaps

- No Scene graph → `scene_baseline` is a layout stand-in.
- No separate render thread → paint is synchronous headless.
- 100k-element arena benches wait for Task 2.1.
