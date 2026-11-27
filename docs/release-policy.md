<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Release policy (Floom 2.0)

This is the gate a release has to pass, and the promise it makes once it has.
Every claim here that can be checked mechanically is checked:
`crates/app/floom/tests/docs_parity.rs` asserts that the test targets and benchmark
targets named below actually exist, and that the golden-example list matches
`docs/examples.md`.

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
- **major** — reserved for a breaking change too large for a minor; the facade
  keeps the `floom` name.

### What is semver-locked

| surface | guarantee | enforced by |
| --- | --- | --- |
| `floom::prelude` — the names a beginner gets from one glob | locked: a removal or rename is breaking | `crates/app/floom/tests/prelude_surface.rs` (inventory of 46 names) |
| `floom` feature-bundle names (`desktop`, `mobile`, `web`, `embedded`, `renderer-gpu`, `renderer-software`, `devtools`) | locked | `crates/app/floom/tests/feature_graph.rs` |
| `floom-core::Widget` / `WidgetAny` | locked | the whole workspace compiles against them |
| `floom-renderer::Renderer`, `floom-platform::{Platform, Window, Surface, Clipboard, Ime}`, `floom-native::{NativeViewProvider, NativeViewController}` | locked — these are the plugin ABIs | the ABI contract suites below |
| `floom-app::WidgetHost` | locked | prelude + host API tests |
| `floom-widgets` public widget API | locked | `crates/widgets/floom-widgets` tests |
| `floom::advanced`, L5 framework crates, design systems | stable: may change in a minor release with a changelog entry | — |
| L8 tooling (`floom-testkit` … `floom-accuracy`) | **no** compatibility guarantee — dev-time only, never shipped in an app binary | the `default` feature graph test |
## Supported targets and tested OS ranges

"Tested" means the criteria at the bottom of this page were run on that target
for that release. Everything else is a build target the code is written for and
nothing more.

| target | backend | status |
| --- | --- | --- |
| `aarch64-apple-darwin` | `floom-platform-desktop` (winit) + `floom-renderer-vello` (Metal) | **tested (CI)**: `macos-window-smoke` opens a real window with the gate probe, asserts a surface and an `NSPasteboard` round trip (`scripts/macos-smoke.sh`); `gpu-pixels` runs the 11 pixel tests on real Metal. Hand-measured on macOS 26.6.2 / M4 Pro / rustc 1.98.0. |
| `x86_64-apple-darwin` | same | **untested** — no Intel Mac on this machine. The window lifecycle is winit's (portable), but nothing here has been measured. |
| `x86_64-pc-windows-msvc` | `floom-platform-desktop` (winit, Win32) + `floom-renderer-software` | **tested (CI, software renderer)**: `Windows app in a window` opens a real window (the runner has a desktop session, no GPU) and asserts a surface plus a frame that carried ink (`scripts/desktop-smoke.sh`, `examples/floom-window-software`). **Not tested**: the GPU presenter inside a window; native-view composition on Win32 is still Phase 16.2. |
| `x86_64-unknown-linux-gnu` | `floom-platform-desktop` (winit, X11) + `floom-renderer-software` | **tested (CI, software renderer)**: `Linux app in a window` opens a real X11 window under `xvfb-run` and asserts a surface plus a frame that carried ink (`scripts/desktop-smoke.sh`, `examples/floom-window-software`); `GPU pixels on lavapipe` runs the 11 vello pixel tests on Mesa's software Vulkan. **Not tested**: the GPU presenter inside a window (CI has no adapter). Native-view composition is still Phase 16.2. |
| `aarch64-linux-android` | `floom-platform-android` | **device-gated (Phase 17)**: emulator-verified start+interaction+IME+CJK+clipboard; a11y/file picker/safe area pending emulator closure |
| `aarch64-apple-ios` | `floom-platform-ios` (UIKit staticlib) | **device-gated (Phase 17)**: device-only — needs a signing identity. The simulator slice below is what runs in CI. |
| `aarch64-apple-ios-sim` | same | **tested (CI)**: `ios-simulator-smoke` scaffolds an app with `floom create`, links it with `swiftc`, installs it on whatever iPhone runtime the runner's Xcode ships, launches it and counts the pixels it drew (`scripts/ios-smoke.sh`). Device behaviour (a11y actions, IME composition, safe area on notched hardware) is still device-gated. |
| OpenHarmony | `floom-platform-harmony` (N-API cdylib) + `examples/floom-ohos` | **emulator-tested for render + touch** (2026-11-27: 2210x2208 frame, `bbox=0x0-540x74`, tap reaches `host.activate`); CI cross-builds and links the example. IME/a11y/clipboard/file/focus still device-gated — nothing forwards them yet |
| `wasm32-unknown-unknown` | `floom-platform-web` | **target-gated** (Phase 19): headless WASM harness pending |
| no-std / bare metal | `floom-platform-embedded` | **target-gated** (Phase 19): `aarch64-unknown-linux-ohos`/`x86_64-unknown-linux-ohos` ELF verified; bare-metal target pending |

The headless core (`floom` with no feature bundle) has no OS dependency at
all: the whole workspace test suite runs without a display, and that is what
makes the headless rows in this table trustworthy.

## Conformance: renderer and platform

A backend is released only if it passes the **ABI's** suite, not just its own —
otherwise "conformance" degenerates into "the tests the author wrote".

Renderer ABI (`floom-renderer`):

- `crates/render/floom-renderer/tests/contract.rs`
- `crates/render/floom-renderer/tests/render_handoff.rs`

Platform ABI (`floom-platform`):

- `crates/platform/floom-platform/tests/contracts.rs`
- `crates/platform/floom-platform-desktop/tests/lifecycle.rs`
- `crates/platform/floom-platform-desktop/tests/accessibility.rs`
- `crates/platform/floom-platform-desktop/tests/winit_capabilities.rs`

Facade and compatibility:

- `crates/app/floom/tests/prelude_surface.rs` — the beginner surface is pinned by name
- `crates/app/floom/tests/feature_graph.rs` — the default build links no windowing,
  renderer, GPU or tooling crate; each bundle adds exactly its own
- `crates/app/floom/tests/docs_parity.rs` — the docs pages agree with the tree

## Performance, size and scale gates

**Size** (release, unstripped, `aarch64-apple-darwin`), recorded per release:

| example | bytes |
| --- | --- |
| `floom-native-plugin` | 485,904 (≈ 0.5 MiB) — the headless floor |
| `floom-showcase` | 12,488,476 (≈ 11.9 MiB) — the application: desktop + GPU + three kit crates + the editor |
| `floom-window` (`--features real`) | 11,626,912 (≈ 11.1 MiB) — desktop + GPU |

A release that grows the headless floor by more than 10 % without a written
reason is a question, not a merge. Since WS-3.2 (2026-11-09) it is also a check:
`just gate size` builds all three tiers in `target/release` and compares them
against the checked-in
`docs/superpowers/benchmarks/floom-2-size-baseline.json` (`threshold_pct 10`,
one budget per feature bundle). A regression >10 % exits 1; a missing baseline
or a failed build exits 2 (a gate that could not run, never a pass). The baseline
takes its three numbers from the table above — keep the two in sync. Refresh
both after a toolchain change with
`bash scripts/check-size-budgets.sh --update`.

**The same gate runs in CI, on its own baseline.** Sizes are host-specific, so
`Size budgets on ubuntu-24.04` compares against
`docs/superpowers/benchmarks/floom-2-size-baseline-linux-x86_64.json` instead of a
macOS number (`FLOOM_SIZE_BASELINE` picks the file). Those numbers are bigger, and
the gap is the interesting part:

| example | macOS (reference host) | Linux (ubuntu-24.04) |
| --- | --- | --- |
| `floom-native-plugin` | 485,904 | 532,072 (+9.5 %) |
| `floom-window --features real` | 11,626,912 | 20,115,008 (**+73 %**) |
| `floom-showcase` | 12,488,476 | 21,654,232 (**+73 %**) |

An unstripped Linux binary carrying both Vulkan and GLES paths, both windowing
stacks and its own libc/libstd copy is plausibly the cause, but nobody has
confirmed it — it is an open question for G11, not a budget change. Both figures
are unstripped; a released binary is stripped, which is where the difference
would shrink. Until that is measured, treat the Linux number as a guard against
regressions, not as the size a user downloads.

**Benchmarks.** `cargo bench`, run on the reference machine (macOS 26.6.2,
Apple M4 Pro). There is no workspace-root bench crate: each suite lives with the
crate it measures, and this list is the set. A target named here that does not
exist fails `the_bench_targets_named_by_the_policy_exist` in
`crates/app/floom/tests/docs_parity.rs`.

| crate | target | what it measures |
| --- | --- | --- |
| `floom-layout` | `incremental` | relayout after a subtree change |
| `floom-runtime-core` | `element_arena` | element lookup and dirty propagation |
| `floom-virtual` | `scroll` | visible-range computation |
| `floom-text` | `shaping` | text shaping |
| `floom-renderer-software` | `self_draw` | CPU rasterisation |
| `floom-renderer-vello` | `scene_render` | GPU scene render |
| `floom-native` | `rows` | native view slot reuse |
| `floom-signal` | `selector` | reactive selector evaluation |
| `floom-animation` | `frame_rates` | animation clock |
| `floom-accessibility` | `tree` | semantics tree build |
| `floom-widgets-data` | `virtual_rows` | virtualised data rows |

Two different gates, and it matters which one is being quoted:

- **The whole set, 20 %, manual.** No benchmark may regress by more than 20 %
  against the previous release on the reference machine without a written reason
  recorded next to the number. This comparison is **manual today**: only the hot
  benches below have committed baselines, and no CI job enforces the percentage.
  It is stated as a policy because the number has to be recorded somewhere
  before it can be enforced automatically.
- **The hot benches, 10 %, enforced by a script.**
  `docs/superpowers/benchmarks/floom-2-budget-baseline.json` carries the
  calibration Criterion medians and `scripts/check-perf-budgets.sh` measures and
  fails over the threshold (exit 1, verified able to fail). It runs Criterion's
  own default protocol and takes the **best of N** runs, because a single run is
  not stable at this budget: `visible_range/10000` measured 444.3 / 404.8 /
  408.4 ns across three consecutive runs of one binary — a 9.8 % swing against a
  10 % budget — and best-of-3 brought the observed spread to ≤2.6 %.
  It is **not wired into CI on purpose**: the baseline is one host's numbers, and
  a job that fails on someone else's runner is worse than a job that does not
  run. Widening the support matrix above is the prerequisite.

  **2026-11-09 已恢复并复验**：基线从 git 历史 `5f4cbcfd` 恢复（sha256 与历史一致），
  `scripts/check-perf-budgets.sh` 实跑 9 条预算 best-of-2 全绿（`exit 0`）；故意改坏
  一条 median 时 `exit 1`，基线缺失时 `exit 2`。上面 "enforced by a script"
  重新成立；步骤与证据见 [`roadmap.md`](roadmap.md) WS-3.1。

**Idle.** An idle window must not burn a core:

```sh
FLOOM2_WINDOW_IDLE=1 FLOOM2_WINDOW_IDLE_SECS=8 cargo run -p floom-window --features real
```

Last measured (macOS 26.6.2, Apple M4 Pro), at the case's own 30 s horizon and
reproduced twice: `frames=2 redraws_requested=3 auto_redraws=1 events=4`,
process CPU `0.15 s` (0.5 % of one core), with `about_to_wait` 82 and 71 — i.e.
the loop parked ~75 times and never asked for a frame. The gate is on **frames**: two frames for a window that has been idle for
eight seconds is the whole point. `redraws_requested` and the CPU figure drift
by a redraw or two between runs — they count the startup redraw and whatever the
window server sends in the first second — so they are recorded, not gated. A
window that keeps producing frames while nothing is happening fails the release.

**Scale.** The virtualization claims are covered by unit and integration tests
in the crates that own row pooling (`floom-widgets`' list and grid suites),
rather than by a standalone example binary. `scripts/run-golden-examples.sh`
runs the remaining headless examples.

## Deprecation policy

- **Deprecations.** Mark a locked-surface member with `#[deprecated]` and a
  replacement, then remove at the next minor release. No silent removals from a
  locked surface.
- **The `floom` name.** The facade is `floom`; it is the public name, and
  renaming it is a major event, not a minor one.

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
   --all`: it buries the diff in crates the release does not touch.
5. Metal smoke: the window-idle command above, on a real display.
6. Record the four criteria, the release sizes and the test count in the
   release notes for the tag.
