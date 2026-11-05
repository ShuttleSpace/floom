# Floom Industrial 1.0 Design

Date: 2026-08-31

## Summary

Floom targets a **Rust desktop/editor-grade industrial GUI framework**: stable core API, equal platform commitment across Windows / macOS / Linux / Android / iOS / OHOS, written gate checklists, and first-class developer tooling (headless testing, inspector, jj workflow).

**flome-ui** (Material / Cupertino / Fluent) and **floom-editor** remain **independent official satellite libraries**. They are **not** part of the floom 1.0 commitment statement. Vendor forks currently living in `floom_packages` move **in-tree** under `platform-compat/`.

This design deliberately does **not** copy Slint’s DSL. Floom stays Rust-first (signals + views). Industrial maturity is borrowed from Slint’s engineering surface: platform abstraction, a11y, gates, tooling—not markup language.

## Goals

- Ship **floom 1.0** with a frozen, documented commitment surface (reactive, view/style/layout, embedder, platform services, a11y, basic widgets, renderers, six platforms).
- Gate 1.0 on **written per-platform checklists** (simulator allowed; critical items spot-checked on device)—no `fail` entries.
- Move `floom_packages` into `floom/platform-compat/` and rewire Cargo `[patch]` to in-repo paths.
- Extract **floom-editor** from `editor-core` + editor views; delete the core `editor` feature when migration completes (no long-lived compatibility shim).
- Keep **flome-ui** separate; document ecosystem version matrix only.
- Build DevEx: Headless Testing API, upgraded inspector, gate scripts, preview host (experimental OK), jj-based progress tracking.
- Upstream what can be upstreamed (esp. iOS openURL → lapce/winit); keep only necessary OHOS vendor fork long-term.

## Non-Goals

- Do not put flome-ui or floom-editor into the floom 1.0 promise / release guarantee.
- Do not require full on-device certification matrix (gate tier C) before tagging 1.0.
- Do not add a `.floom` DSL, full LSP live-preview, or Figma→page codegen in this phase.
- Do not keep long-term deprecated re-exports for editor APIs after extraction.
- Do not implement design-system semantics (MD/Cupertino/Fluent) inside floom core widgets.
- Do not make floom depend on flome-ui or floom-editor.

## Decisions (locked)

| Topic | Choice |
|-------|--------|
| Delivery shape | floom 1.0 core + independent official kit (flome-ui) |
| Platforms | Desktop + Android + iOS + OHOS **equal** commitment |
| Release gate | Written checklist; simulator OK; critical spot-checks on device |
| flome-ui vs 1.0 | Kit **not** in floom 1.0 commitment |
| Editor | Independent official library (peer of flome-ui) |
| Vendor layout | In-tree `platform-compat/` (not external `floom_packages`) |
| VCS for progress | **jujutsu (jj)**; git push when syncing remotes |

## Architecture

### Repository layout

```
floom/                          # 1.0 commitment lives here
  src/                          # core, embedder, platform bridges, a11y, basic views
  platform-compat/              # vendor windowing / packaging forks only
    floem-winit/                # iOS openURL fork (exit when upstreamed)
    tgui-winit-ohos/            # OHOS retarget + !Send (long-lived until upstream)
    harmony-app-rs/             # patches + USAGE only (not linked into lib)
    README.md                   # upstream, delta, exit criteria per tree
  docs/gates/<platform>.md
  docs/ecosystem.md             # floom ↔ flome-ui / floom-editor compatibility
  examples/{android,ios,ohos,…}

flome-ui/                       # separate git/jj repo — official DS kit
floom-editor/                   # separate git/jj repo — extracted editor
```

### Dependency direction

```
flome-ui ──────────────► floom
floom-editor ──────────► floom
floom ──✗──► flome-ui / floom-editor
```

### Cargo patches (in-repo)

```toml
[patch.crates-io]
tgui-winit-ohos = { path = "platform-compat/tgui-winit-ohos" }

[patch."https://github.com/lapce/winit"]
floem-winit = { path = "platform-compat/floem-winit/winit" }
```

### Layering rule

- **Product bridges** (IME, clipboard, files, a11y, safe area) stay in `crates/floom/src/platform/*`, `crates/floom/src/embedder/*`, `crates/floom/src/accessibility/*`.
- **`platform-compat/`** holds third-party windowing/packaging forks only—no Floom widgets or reactive logic.

## Section 1 — Crate boundaries

### floom (committed)

- Reactive: signals, effects, scopes; document `create_global_rw_signal` / `Scope` as stable where satellites already rely on them.
- View / Style / Layout / paint / animations / theming classes.
- Embedder protocol + default winit path + headless.
- Platform services via `BackendCapabilities`.
- Accessibility neutral tree + desktop + mobile/OHOS backends; basic widgets ship default roles/labels.
- Basic `views::*` excluding editor.
- Renderers: at least one GPU path + tiny-skia CPU fallback; features documented.

### flome-ui (not committed in floom 1.0)

- Continues as independent workspace (`floom-elements` facade + Material/Cupertino/Fluent crates).
- Depends on `floom ^1`; local patch path must point at this machine’s floom tree (fix broken `/Users/ahs/...`).
- Gallery-grade depth is acceptable for kit releases; deepening is kit’s roadmap.

### floom-editor (not committed in floom 1.0)

- Receives `editor-core/`, `crates/floom/src/views/editor/**`, `text_editor`, related features and examples (`editor`, `syntax-editor`).
- Depends only on **public** floom APIs; missing hooks become documented floom extension points first.
- After extraction, floom **deletes** the `editor` feature (no permanent compatibility layer).

## Section 2 — Commitment surface and gates

### Per-platform gate files

Maintain `docs/gates/{windows,macos,linux,android,ios,ohos}.md` with statuses `pass` / `fail` / `n/a`. Tagging `1.0.0` requires **no `fail`**.

### Shared checklist

1. Official example starts and basic interaction works.
2. IME: Latin + at least one CJK path (simulator OK; device spot-check).
3. a11y: screen reader sees button/field labels; Default action works.
4. Clipboard read/write.
5. File pick (or platform equivalent); share / open URL on mobile.
6. Safe area: mobile insets applied; desktop `n/a`.
7. Focus / basic keyboard navigation (desktop required; mobile as capable).
8. CI: target compiles (OHOS may document known sysroot exceptions; example path must be reproducible).

Spot-check: each of the six platforms signed off at least once (device or official emulator), recorded in the gate file.

### CHANGELOG policy

Distinguish **Core** vs **platform-compat**. Compat churn does not by itself bump floom major unless public embedder/API changes.

## Section 2b — Developer tooling

### Required for 1.0 engineering

| Capability | Approach |
|------------|----------|
| Headless Testing API | Public queries/actions on headless embedder + a11y tree; CI examples |
| Inspector upgrade | Layout + applied style + focus + a11y tree + capabilities readout |
| Gate docs + scripts | `docs/gates/*.md` + `just gate-<platform>` (or equivalent) |
| jj workflow | Daily progress via jj; push via `jj git push` when needed |
| platform-compat README | Upstream, delta, deletion criteria per vendor tree |
| Ecosystem matrix | `docs/ecosystem.md` for satellite compatibility |

### Should-have (may be experimental at tag time)

| Capability | Approach |
|------------|----------|
| Preview host | Rebuild-on-change host for a view factory (no DSL live preview) |
| Bench baselines | Existing criterion benches; regress alerts on critical paths |
| Mobile one-shot scripts | Documented android/ios/ohos example builds using in-tree compat |

### Explicitly out of scope now

- Floom DSL / full LSP live preview
- Figma full-page codegen (token export may land later in flome-ui)
- flome-ui gallery as a floom CI required gate

## Section 3 — platform-compat migration and upstream

### One-time import

| Source (`floom_packages`) | Destination | Notes |
|---------------------------|-------------|-------|
| `floem-winit-ios-fork/` | `platform-compat/floem-winit/` | Drop nested `.git`; track in floom jj/git |
| `tgui-winit-ohos-fork/` | `platform-compat/tgui-winit-ohos/` | Keep retarget + `!Send` |
| `tgui-winit-ohos-patch/` | `platform-compat/tgui-winit-ohos/patch/` | Reproducible rename patch |
| `harmony-app-rs-patch/` | `platform-compat/harmony-app-rs/` | Patches + USAGE only |
| `floem-winit-ios-fork.old0295/` | — | **Do not import; delete** |

After import, archive or remove external `floom_packages` to avoid dual sources. Prefer **not** submodules (conflicts with single-repo jj progress).

### Upstream / exit criteria

| Component | Strategy | Exit when |
|-----------|----------|-----------|
| iOS `set_on_open_url` | PR to lapce/winit | Upstream has equivalent API; share cold-start gate passes |
| tgui-winit-ohos | Long-lived vendor (or PR tgui) | Registry crate works with floem-winit-core and OHOS gate passes |
| harmony-app-rs | PR upstream | Official macOS/Linux hvigor flow works |
| Android safe area | Implement in `src/` (`WindowInsets` → `EmbedderEvent::SafeArea`) | Gate safe-area item `pass` |

## Section 4 — Satellite contracts and extraction order

### Version contract

- floom: semver `1.x`.
- flome-ui / floom-editor: own semver; depend on `floom = "1"`.
- floom CI does **not** require satellite green; satellites CI against floom `1.0` tags / path.
- README / 1.0 notes must not claim DS completeness or IDE-editor completeness.

### Suggested execution order

1. Import `platform-compat`, rewire patches, add gate skeletons + compat README.
2. Headless Testing API, inspector upgrades, `docs/ecosystem.md`.
3. Create floom-editor; move editor code; delete floom `editor` feature.
4. Point flome-ui at floom 1.0-rc; fix demo builds.
5. Complete six-platform gate checklists → tag `floom 1.0.0`.

## Section 5 — jujutsu workflow

- Initialize colocated jj on each product repo (`jj git init --colocate` where needed).
- Daily: `jj new` / `jj describe` / `jj split` / `jj squash`.
- Sync: `jj git fetch`, rebase as needed, `jj git push`.
- Large steps (compat import, editor extract, gate pass) = separate changes for reviewability.
- Do not add agent coauthor trailers to change descriptions.
- CI continues to clone git; jj is a developer progress tool, not a CI requirement.
- Do not force a single jj workspace across floom + flome-ui + floom-editor unless later decided.

## Risks

| Risk | Mitigation |
|------|------------|
| OHOS sysroot / wgpu / HAP linkage | Document exceptions; keep reproducible example path; harden gate item 8 |
| Dual dpi / winit-core identity | Pin rules in platform-compat README; resolve checks when feasible |
| Editor consumers (e.g. Lapce) break on feature removal | Coordinate cutover in one window; no permanent shim |
| flome-ui patch path / API drift | Ecosystem matrix + satellite CI on 1.0 tags |
| Equal platform promise vs incomplete device validation | Tier-B gates + recorded spot-checks; do not claim tier-C |

## Success criteria

- `floom 1.0.0` tagged with frozen commitment surface and six gate files without `fail`.
- `platform-compat/` in-tree; external path patches gone; old0295 gone.
- floom-editor exists as separate official repo; floom has no `editor` feature.
- flome-ui builds against floom 1.x and is documented as official kit **outside** core promise.
- Headless Testing API usable in CI; inspector shows a11y; jj colocated for ongoing work.

## Open follow-ups (post-spec)

- Implementation plan via writing-plans (ordered tasks, test hooks, concrete PRs/changes).
- Whether preview host lands before or after 1.0 tag (experimental allowed).
- Exact floom-editor crate/package naming and repo URL.
