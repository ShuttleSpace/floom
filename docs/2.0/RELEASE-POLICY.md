# Release policy (Floem 2.0)

This is the gate a release has to pass, and the promise it makes once it has.
Every claim here that can be checked mechanically is checked:
`app/floem2/tests/docs_parity.rs` asserts that the test targets and benchmark
targets named below actually exist, and that the golden-example list matches
`docs/2.0/EXAMPLES.md`.

## Versioning

| field | value |
| --- | --- |
| workspace version | `0.2.0` (`[workspace.package]`) |
| edition | 2024 |
| MSRV | **1.91** (`rust-version`; the toolchain used for the numbers below is rustc 1.98.0) |
| licence | MIT |

Pre-1.0, so the usual 0.x reading applies, stated explicitly rather than left
to be inferred:

- **patch** (`0.2.0 → 0.2.1`) — bug fixes only, no public API change.
- **minor** (`0.2 → 0.3`) — new public API, *or* any breaking change to a
  locked surface. Because this is 0.x, breaking changes ship in a minor
  release; each one gets a changelog entry with a pointer to the migration
  section that covers it.
- **major** — reserved for the retirement of the v1 monolith, at which point
  the facade takes the `floem` name.

### What is semver-locked

| surface | guarantee | enforced by |
| --- | --- | --- |
| `floem2::prelude` — the names a beginner gets from one glob | locked: a removal or rename is breaking | `app/floem2/tests/prelude_surface.rs` (inventory of 46 names) |
| `floem2` feature-bundle names (`desktop`, `mobile`, `web`, `embedded`, `renderer-gpu`, `renderer-software`, `devtools`) | locked | `app/floem2/tests/feature_graph.rs` |
| `floem-core::Widget` / `WidgetAny` | locked | the whole workspace compiles against them |
| `floem-renderer::Renderer`, `floem-platform::{Platform, Window, Surface, Clipboard, Ime}`, `floem-native::{NativeViewProvider, NativeViewController}` | locked — these are the plugin ABIs | the ABI contract suites below |
| `floem-app::WidgetHost` | locked | prelude + migration tests |
| `floem-widgets` public widget API | locked | `widgets/floem-widgets` tests |
| `floem2::advanced`, L5 framework crates, design systems | stable: may change in a minor release with a changelog entry | — |
| L8 tooling (`floem-testkit` … `floem-agent`) | **no** compatibility guarantee — dev-time only, never shipped in an app binary | the `default` feature graph test |
| `floem` (the v1 monolith) | compat tier, see [Deprecation](#deprecation-policy) | `tests/compat_prelude.rs` |

## Supported targets and tested OS ranges

"Tested" means the criteria at the bottom of this page were run on that target
for that release. Everything else is a build target the code is written for and
nothing more.

| target | backend | status |
| --- | --- | --- |
| `aarch64-apple-darwin` | `floem-platform-desktop` (winit) + `floem-renderer-vello` (Metal) | **tested**: macOS 26.6.2, Apple M4 Pro, rustc 1.98.0 |
| `x86_64-apple-darwin` | same | **untested** — no Intel Mac on this machine. The window lifecycle is winit's (portable), but nothing here has been measured. |
| `x86_64-pc-windows-msvc` | `floem-platform-desktop` | **device-gated**: windowing goes through winit, but native-view composition on Win32 is Phase 16.2 and incomplete. No Windows machine in CI. |
| `x86_64-unknown-linux-gnu` | `floem-platform-desktop` | **device-gated**: same gap, X11. No Linux machine in CI. |
| `aarch64-linux-android` | `floem-platform-android` | **device-gated** (Phase 17) |
| `aarch64-apple-ios` | `floem-platform-ios` | **device-gated** (Phase 17) |
| OpenHarmony | `floem-platform-harmony` | **device-gated** (Phase 19) |
| `wasm32-unknown-unknown` | `floem-platform-web` | **target-gated** (Phase 19) |
| no-std / bare metal | `floem-platform-embedded` | **target-gated** (Phase 19) |

The headless core (`floem2` with no feature bundle) has no OS dependency at
all: the whole workspace test suite runs without a display, and that is what
makes the headless rows in this table trustworthy.

## Conformance: renderer and platform

A backend is released only if it passes the **ABI's** suite, not just its own —
otherwise "conformance" degenerates into "the tests the author wrote".

Renderer ABI (`floem-renderer`):

- `render/floem-renderer/tests/contract.rs`
- `render/floem-renderer/tests/render_handoff.rs`

Platform ABI (`floem-platform`):

- `platform/floem-platform/tests/contracts.rs`
- `platform/floem-platform-desktop/tests/lifecycle.rs`
- `platform/floem-platform-desktop/tests/accessibility.rs`
- `platform/floem-platform-desktop/tests/winit_capabilities.rs`

Facade and compatibility:

- `app/floem2/tests/prelude_surface.rs` — the beginner surface is pinned by name
- `app/floem2/tests/feature_graph.rs` — the default build links no windowing,
  renderer, GPU or tooling crate; each bundle adds exactly its own
- `app/floem2/tests/migration.rs` — the before/after snippets in
  `docs/2.0/MIGRATION.md` compile and behave as documented
- `app/floem2/tests/docs_parity.rs` — the three docs pages agree with the tree
- `tests/compat_prelude.rs` — `floem::compat::v1_prelude` is a faithful mirror
  of `floem::prelude`

## Performance, size and scale gates

**Size** (release, unstripped, `aarch64-apple-darwin`), recorded per release:

| example | bytes |
| --- | --- |
| `floem2-hello` | 2,694,256 (≈ 2.6 MiB) — the headless floor |
| `floem2-counter` | 2,835,248 (≈ 2.7 MiB) |
| `floem2-window` (`--features real`) | 11,626,912 (≈ 11.1 MiB) — what desktop + GPU costs on top |

A release that grows the headless floor by more than 10 % without a written
reason is a question, not a merge.

**Benchmarks.** `cargo bench`, run on the reference machine (macOS 26.6.2,
Apple M4 Pro). Bench targets at the workspace root: `event_dispatch`,
`style_computation`, `painting`, `layout`, `animation`, `text`,
`view_creation`, `runtime_baseline`, `scene_baseline`, `render_baseline`. Plus
per-crate suites in `floem-layout`, `floem-virtual`, `floem-text`,
`floem-runtime-core`, `floem-renderer-vello`, `floem-native`, `floem-store`,
`floem-animation`, `floem-accessibility`, `floem-widgets-data`.

Gate: **no benchmark may regress by more than 20 % against the previous
release** on the reference machine (macOS 26.6.2, Apple M4 Pro) without a
written reason recorded next to the number. This comparison is **manual
today** — criterion baselines are not yet committed and no CI job enforces the
percentage. It is stated as a policy because the number has to be recorded
somewhere before it can be enforced automatically.

**Idle.** An idle window must not burn a core:

```sh
FLOEM2_WINDOW_IDLE=1 FLOEM2_WINDOW_IDLE_SECS=8 cargo run -p floem2-window --features real
```

Last measured (macOS 26.6.2, Apple M4 Pro): `frames=2 redraws_requested=3
auto_redraws=1 events=4`, process CPU `0.17 s` (2.1 % of one core) over 8 s
idle. The gate is on **frames**: two frames for a window that has been idle for
eight seconds is the whole point. `redraws_requested` and the CPU figure drift
by a redraw or two between runs — they count the startup redraw and whatever the
window server sends in the first second — so they are recorded, not gated. A
window that keeps producing frames while nothing is happening fails the release.

**Scale.** The two virtualization claims are asserted, not asserted-once:
`examples/floem2-virtual-list` proves 38 elements materialized for 100 000 rows
(0.04 %), and `examples/floem2-datagrid` proves equal paint ops (44 vs 44) for
1 000 versus 100 000 rows, with a row pool capped at 16. Both are re-run by
`scripts/run-golden-examples.sh`.

## Deprecation policy

- **The v1 compat tier.** `floem::prelude` and `floem::compat::v1_prelude` are
  guaranteed for the current major version of the `floem` crate. Removing a
  name from it requires: one release carrying `#[deprecated]` with the
  replacement in the message, an entry in `docs/2.0/MIGRATION.md`, and a
  changelog line. The v1 crate is not deleted while any example or test in this
  workspace still imports it (342 files do today).
- **2.0 deprecations.** Mark with `#[deprecated]` and a replacement, then
  remove at the next minor release. No silent removals from a locked surface.
- **The `floem` name.** Held by the monolith. The facade is `floem2` until the
  monolith retires; renaming it is then a one-line change, and it is a major
  event, not a minor one.

## The gate

Every release, in this order:

1. `cargo test --workspace --no-fail-fast` — green, with the delta against the
   previous baseline accounted for test by test (a number that moved without an
   explanation is a failure).
2. `bash scripts/check-architecture-deps.sh` — PASS. No upward dependency edge
   unless allowlisted with a written reason, and no unregistered 2.0 crate.
3. `bash scripts/run-golden-examples.sh` — every headless golden example runs
   and passes. Compiling them is not enough; their assertions are the content.
4. `cargo fmt --package=<each touched package>` — clean. Never `cargo fmt
   --all`: the v1 crate is edition 2024 with pre-existing drift, and
   reformatting it buries the diff.
5. Metal smoke: the window-idle command above, on a real display.
6. Record the four criteria, the release sizes and the test count in
   `.superpowers/sdd/progress-floem-2.md`.
