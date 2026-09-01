# Floem 2.0 Agent Execution Package

Start with these two documents:

1. `docs/superpowers/specs/2026-09-02-floem-2-complete-architecture.md`
2. `docs/superpowers/plans/2026-09-02-floem-2-complete-implementation-plan.md`

## Fork overrides

- **No Legacy View adapter** — see `docs/superpowers/decisions/2026-09-04-no-legacy-view-adapter.md`. Phase 8 is skipped. Do not preserve old `View` APIs behind shims when the new runtime lands.
- Progress ledger: `.superpowers/sdd/progress-floem-2.md`

Execution rules:

- Treat the architecture spec as normative unless a concrete repository constraint proves otherwise.
- Treat the implementation plan as the ordered execution plan, except where a checked-in decision overrides it.
- Before changing code, inspect the actual Floem repository and reconcile paths/names with the plan; do not invent repository structure without checking.
- Use TDD for each implementation task: failing test → minimal implementation → focused test → broader test → benchmark where relevant → commit (when the user requests commits).
- Use isolated worktrees for parallel feature work.
- Freeze an interface before parallelizing downstream tasks.
- Never claim completion without running the relevant verification commands.
- Update the plan checkboxes as tasks are completed.
- If an implementation choice conflicts with the spec, stop and record the conflict in `docs/superpowers/decisions/` before proceeding.

The plan intentionally spans P0–P4. P0 must be completed and benchmarked before large-scale widget/platform expansion.
