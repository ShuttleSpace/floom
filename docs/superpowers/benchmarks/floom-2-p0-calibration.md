# Floom 2.0 Phase 0 — desktop calibration (macOS arm64)

**Date:** 2026-09-04  
**Host:** macOS 26.5.1 arm64, rustc 1.98.0  
**Git:** branch `floom-2.0`  
**Command shape:** `cargo bench -p floom --bench <name> -- --sample-size 10 --warm-up-time 1 --measurement-time 2`

| Bench | Case | Median time |
|---|---|---|
| `runtime_baseline` | `mount_1k_labels` | ~18.7 ms |
| `runtime_baseline` | `signal_update_single_label_in_1k` | ~2.74 ms |
| `render_baseline` | `paint_1k_labels_headless` | ~648 µs |
| `scene_baseline` | `relayout_10k_empty_cells` | ~5.10 ms |

These are **indicative** numbers for local regression hunting, not CI gates. See `benches/memory_baseline.md` for the >10% CI policy (deferred until multi-run calibration).
