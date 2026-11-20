# Merge Floom Editor into One Satellite Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge `crates/widgets/floom-editor-core`, `crates/widgets/floom-editor`, and `crates/ecosystem/floom-editor/floom-editor` into one satellite crate at `crates/ecosystem/floom-editor/floom-editor` (package `floom-editor`) with a single editor API based on `text_editor()/TextEditor + EditorState/Document/Styling`, removing redundant model/widget implementations.

**Architecture:** The satellite crate becomes self-contained: fold `floom-editor-core` into `floom_editor::core::*` (re-export `lapce_xi_rope as xi_rope`), keep the existing satellite view layer (`editor/style/styling/document/text_editor`) as the canonical editor, and port IME preedit/composition, autofocus-on-mount, and semantics hints from the 2.0 `EditorWidget`. Root workspace drops both `widgets/floom-editor*` crates; `examples/floom-showcase` and satellite examples/tests migrate to the unified API.

**Tech Stack:** Rust 2024 workspace/satellite crates, `lapce-xi-rope`, `floom-core`/`floom-text`/`floom-types`/`floom-widgets` 2.0 widget APIs, `Mutex`/`Arc` shared editor state, cargo test/check, repo scripts/CI manifests.

**Spec:** User-approved decisions in conversation: do not move crates as-is; merge into one satellite crate; remove redundant/duplicate parts; keep only one editor; canonical API is satellite `text_editor()/TextEditor + EditorState/Document/Styling`; core exposed as `floom_editor::core::{buffer, selection, cursor, ...}` (option A).

## Global Constraints

- Do not create a second package named `floom-editor` in the same Cargo workspace members set (Cargo rejects duplicate package names in one workspace).
- Preserve satellite/editor capabilities that currently exist: multi-cursor, soft wrap, gutter/line numbers, `Styling` hook, `Document` abstraction, rope-backed `TextDocument` over core `Buffer`.
- Port in 2.0 capabilities that are currently unique: IME preedit/composition handling, autofocus-on-mount, useful semantics/accessibility hints. Do not ship `Event::Ime(_) => false` in the unified editor.
- `floom-editor-core` ceases to exist as a separate crate; its public model must remain available under `floom_editor::core` and `ide` must compile against it.
- `crates/widgets/floom-editor` (2.0 `EditorWidget`/simplified `TextDocument`/`Selection`/`UndoManager`/`Transaction`) is removed as redundant once its needed behavior is ported.
- Keep changes consistent with existing repo conventions (SPDX headers where present, path-dep satellite style, existing CI/gate/script patterns).
- `docs/ecosystem.md` must no longer describe two `floom-editor` packages sharing the root resolve graph.

## Review Focus

- IME preedit not committed to document text and caret/area follows composition: a composing user must not get preedit persisted as document content or a stuck/misplaced OS caret.
- Autofocus/focus regression on navigation remount: opening `/editor` in showcase (and returning to it) must let typing reach the editor without manual focus hacks breaking.
- Showcase toolbar undo/redo/reset still mutates the same document the editor shows: button actions must affect visible text/undo history, not a stale clone.
- `floom_editor::core` re-exports cover `ide`/advanced needs (`buffer::Buffer`, `buffer::rope_text::{RopeText,RopeTextVal}`, `selection::{Selection,SelRegion}`, `cursor::CursorAffinity`, `editor::EditType`, `xi_rope`): downstream examples must not need the deleted `floom-editor-core` crate.
- Root/workspace integrity: root `cargo` metadata/test targets no longer reference removed `widgets/floom-editor*` members, and satellite ecosystem checks/CI still lint+test the merged crate.

---

## File Structure

**Create/reshape in merged satellite crate (`crates/ecosystem/floom-editor/floom-editor/`):**
- Create: `src/core/mod.rs` — exposes `pub mod buffer; pub mod char_buffer; ...; pub use lapce_xi_rope as xi_rope;` mirroring current core crate.
- Create: `src/core/**` — copied/adapted modules from `crates/widgets/floom-editor-core/src/**` (buffer, cursor, selection, editor, movement, word, mode, register, etc.), with `crate::` paths updated as needed.
- Modify: `src/lib.rs` — keep canonical exports (`text_editor`, `TextEditor`, `Editor`, `EditorState`, `Document`, `TextDocument`, `EditorStyle`, `Styling`, `SimpleStyling`) and add `pub mod core;` plus any compatibility re-exports needed by examples/tests.
- Modify: `src/editor.rs` — port IME state/handling, autofocus/focus lifecycle, IME caret area reporting, preedit paint; add editor-level helpers needed by showcase (`undo`, `redo` convenience as appropriate) without regressing multi-cursor/wrap/gutter behavior.
- Modify: `src/text_editor.rs` — add builder/accessors needed by migration (e.g. `.autofocus(...)`, `undo/redo` passthrough if chosen) while keeping `text_editor()` and `TextEditor` canonical.
- Modify: `src/document.rs` — keep `Document`/`TextDocument` over `crate::core::buffer::Buffer` (not deleted core crate path); adjust imports to `crate::core::*`.
- Modify: `src/style.rs`, `src/styling.rs` — import/type updates only if required.
- Modify: `Cargo.toml` — remove path dep on deleted `../../../widgets/floom-editor-core`, add deps/features previously provided by core (`strum`, `strum_macros`, `ui-events`, `itertools`, `bitflags`, `memchr`, `lapce-xi-rope`, `serde` optional) and keep/add `floom-accessibility` if needed for semantics; set `version`/metadata aligned for 0.2.0 consumers.
- Modify: `tests/paint.rs`, `tests/public_api.rs` — update imports to `floom_editor::core::*` where they used `floom_editor_core`, and add/adjust tests for IME/autofocus/undo-redo API as implemented.

**Satellite workspace/examples:**
- Modify: `crates/ecosystem/floom-editor/Cargo.toml` — ensure members/deps support merged crate; version alignment as needed.
- Modify: `crates/ecosystem/floom-editor/examples/editor/Cargo.toml` — depend on local merged crate (`path = "../../floom-editor"` or equivalent), not `widgets/floom-editor`.
- Modify: `crates/ecosystem/floom-editor/examples/syntax-editor/Cargo.toml` — same.
- Modify: `crates/ecosystem/floom-editor/examples/ide/Cargo.toml` and `examples/ide/src/**/*.rs` — depend on merged crate and replace `floom_editor_core::...` with `floom_editor::core::...`.
- Modify: `crates/ecosystem/floom-editor/Cargo.lock` — regenerate/update for merged dependency graph.

**Root workspace / consumers / deletion:**
- Delete: `crates/widgets/floom-editor/` (entire crate, including its `tests/editing.rs` or replace coverage in satellite).
- Delete: `crates/widgets/floom-editor-core/` (entire crate).
- Modify: `Cargo.toml` (root) — remove members/default-members and `[workspace.dependencies]` entries for `floom-editor` and `floom-editor-core`; repoint `floom-editor` workspace dep to satellite path if still needed by root consumers, or have `examples/floom-showcase` use direct path dep.
- Modify: `examples/floom-showcase/Cargo.toml` — depend on satellite merged `floom-editor` (path) and keep features as needed.
- Modify: `examples/floom-showcase/src/state.rs`, `examples/floom-showcase/src/editor.rs`, `examples/floom-showcase/src/pages.rs`, `examples/floom-showcase/src/tests.rs` — migrate from `EditorWidget` to `TextEditor`/`EditorState` (state handle for undo/redo/text), preserve autofocus, typing, undo/redo/reset, and shared-instance-across-navigation behavior.
- Modify: `Cargo.lock` (root) — remove deleted packages, add/update satellite path resolution.
- Modify: `scripts/check-architecture-deps.sh` — remove `floom-editor`/`floom-editor-core` from LAYERS/UPWARD_ALLOW as root workspace members if no longer present (or adjust to reflect new ownership) without breaking checks.
- Modify: `scripts/gate.sh` — remove `--package=floom-editor` / `floom-editor-core` from root package lists; keep satellite tested in ecosystem loop.
- Modify: `.github/workflows/ci-core.yml` and `.github/workflows/ci-plugins.yml` as needed — remove root-package references to deleted crates; keep ecosystem/floom-editor formatting/testing coverage.
- Modify: docs — `docs/ecosystem.md`, `crates/ecosystem/floom-editor/README.md`, `docs/architecture.md`, `docs/workspace-conventions.md`, `docs/examples.md`, `CONTRIBUTING.md`, `CHANGELOG.md` (and any other references found) to describe one satellite `floom-editor` crate with `core` module.

---

### Task 1: Fold `floom-editor-core` into satellite as `floom_editor::core`

**Files:**
- Create: `crates/ecosystem/floom-editor/floom-editor/src/core/**` (all modules from `crates/widgets/floom-editor-core/src/**`)
- Create: `crates/ecosystem/floom-editor/floom-editor/src/core/mod.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/lib.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/Cargo.toml`
- Modify: `crates/ecosystem/floom-editor/Cargo.toml` (if deps/features centralized)
- Test: `crates/ecosystem/floom-editor/floom-editor/tests/public_api.rs` (and compilation of crate)

**Interfaces:**
- Consumes: current core modules and `floom-editor-core` public surface (`buffer`, `selection`, `cursor`, `editor::EditType`, `mode`, `movement`, `register`, `xi_rope`, etc.).
- Produces: `floom_editor::core::*` with `pub use lapce_xi_rope as xi_rope;` and modules importable as `floom_editor::core::buffer::...`, `floom_editor::core::selection::...` for downstream migration.

- [ ] **Step 1: Write/update a failing API test for core module exposure**

In `crates/ecosystem/floom-editor/floom-editor/tests/public_api.rs` add assertions that compile when exposed:
```rust
#[test]
fn core_model_is_exposed() {
    let _ = core::mem::size_of::<floom_editor::core::buffer::Buffer>();
    let _ = core::mem::size_of::<floom_editor::core::selection::Selection>();
    let _: fn() = || {
        let _ = floom_editor::core::xi_rope::Rope::from("");
    };
}
```
(Adjust to exact public types if names differ; test must fail before `core` exists.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p floom-editor --test public_api core_model_is_exposed` (from `crates/ecosystem/floom-editor`)
Expected: FAIL (no `core` module / unresolved path).

- [ ] **Step 3: Copy core sources into `src/core/`, add `core/mod.rs`, wire `pub mod core;`, and move required dependencies/features into satellite `Cargo.toml`**

Implementation notes: preserve module contents/visibility; replace external `crate::` assumptions only as needed; include `pub use lapce_xi_rope as xi_rope;`; add deps `itertools`, `bitflags`, `memchr`, `strum`, `strum_macros`, `ui-events` (and optional `serde`) matching core behavior.

- [ ] **Step 4: Run targeted tests to verify it passes**

Run: `cargo test -p floom-editor --test public_api` (from `crates/ecosystem/floom-editor`)
Expected: PASS for new and existing public API checks.

- [ ] **Step 5: Commit**

```bash
git add crates/ecosystem/floom-editor/floom-editor/src/core crates/ecosystem/floom-editor/floom-editor/src/lib.rs crates/ecosystem/floom-editor/floom-editor/Cargo.toml crates/ecosystem/floom-editor/floom-editor/tests/public_api.rs crates/ecosystem/floom-editor/Cargo.toml
git commit -m "refactor(editor): fold editor-core model into floom_editor::core"
```

### Task 2: Switch satellite view layer to internal `core` and decouple from deleted core crate

**Files:**
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/document.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/editor.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/text_editor.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/style.rs` (if needed)
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/styling.rs` (if needed)
- Modify: `crates/ecosystem/floom-editor/floom-editor/tests/paint.rs`
- Test: `crates/ecosystem/floom-editor/floom-editor` crate tests

**Interfaces:**
- Consumes: `floom_editor::core::{buffer::Buffer, buffer::rope_text::{RopeText,RopeTextVal}, editor::EditType, selection::{SelRegion,Selection}, cursor::CursorAffinity, xi_rope::Rope}`.
- Produces: same satellite editor API (`Document`, `TextDocument`, `Editor`, `EditorState`, `TextEditor`, `text_editor`, `EditorStyle`, `Styling`, `SimpleStyling`) compiling without `floom-editor-core` dependency.

- [ ] **Step 1: Write the failing compile/test checkpoint**

Update `tests/paint.rs` and any in-crate references from `floom_editor_core::...` to `floom_editor::core::...` (or `crate::core::...` in lib) and remove `floom-editor-core` path dep from crate manifest; this should fail to compile until imports are fixed.

- [ ] **Step 2: Run to verify it fails for the expected reason**

Run: `cargo test -p floom-editor` (from `crates/ecosystem/floom-editor`)
Expected: FAIL with unresolved `floom_editor_core` / missing dependency or path errors.

- [ ] **Step 3: Replace all `floom_editor_core::` imports with `crate::core::`/`floom_editor::core::` and remove external core dependency**

Keep behavior unchanged (text editing, selection, undo/redo via `Document`/`Buffer`).

- [ ] **Step 4: Run tests to verify it passes**

Run: `cargo test -p floom-editor` (from `crates/ecosystem/floom-editor`)
Expected: PASS (or only pre-existing unrelated failures called out explicitly).

- [ ] **Step 5: Commit**

```bash
git add crates/ecosystem/floom-editor/floom-editor
git commit -m "refactor(editor): use internal core model in satellite editor"
```

### Task 3: Port IME, autofocus, semantics, and editor convenience API

**Files:**
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/editor.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/text_editor.rs`
- Modify: `crates/ecosystem/floom-editor/floom-editor/src/lib.rs` (exports if needed)
- Modify/Test: `crates/ecosystem/floom-editor/floom-editor/tests/paint.rs` or Create: `crates/ecosystem/floom-editor/floom-editor/tests/ime.rs`

**Interfaces:**
- Consumes: `floom_core`/`floom_types` IME and focus APIs (`Event::Ime(ImeUpdate { text, caret })`, `UpdateContext::set_ime_cursor_area`, `request_focus`, focus flags) as used in `crates/widgets/floom-editor/src/widget.rs`.
- Produces: unified editor behavior — `Event::Ime` handled (preedit shown, not committed until commit), autofocus option on `TextEditor`/builder, IME caret area updates on focus/caret changes, `SemanticsHint`/role appropriate for editable text; `TextEditor` exposes operations needed by showcase migration (`set_text` exists, add `undo/redo` and/or accessible state methods with exact signatures chosen and documented).

- [ ] **Step 1: Write failing tests for missing capabilities**

Add tests covering at least: preedit rendered/stored without mutating document text; IME commit inserts text and clears preedit; autofocus requests focus on mount when enabled; undo/redo callable via the public API chosen for showcase.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p floom-editor` (target new test names)
Expected: FAIL because IME/autofocus/API hooks not implemented.

- [ ] **Step 3: Implement minimal code to pass**

Port logic from `crates/widgets/floom-editor/src/widget.rs`: preedit/caret state, dirty tracking, `sync_ime_area` equivalent in editor update/paint path, handle `Event::Ime` in `event()`, draw preedit near caret, add autofocus flag wired through `TextEditor` builder and `update`, implement undo/redo convenience on `TextEditor`/`Editor` delegating to `Document::undo/redo` and bumping generation/revision as needed.

- [ ] **Step 4: Run tests to verify pass**

Run: `cargo test -p floom-editor`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/ecosystem/floom-editor/floom-editor/src crates/ecosystem/floom-editor/floom-editor/tests
git commit -m "feat(editor): add IME, autofocus, and unified editor helpers"
```

### Task 4: Migrate satellite examples (including `ide`) to merged crate and `core` path

**Files:**
- Modify: `crates/ecosystem/floom-editor/examples/editor/Cargo.toml`
- Modify: `crates/ecosystem/floom-editor/examples/syntax-editor/Cargo.toml`
- Modify: `crates/ecosystem/floom-editor/examples/ide/Cargo.toml`
- Modify: `crates/ecosystem/floom-editor/examples/ide/src/**/*.rs` (imports/usages of `floom_editor_core`)
- Modify: `crates/ecosystem/floom-editor/examples/editor/src/main.rs` and others only if API drift requires
- Test: example crates in `crates/ecosystem/floom-editor`

**Interfaces:**
- Consumes: `floom_editor::{text_editor, TextEditor, EditorStyle, Styling, Document, TextDocument, EditorState, core::*}`.
- Produces: all satellite examples depend on local merged `floom-editor` and compile; `ide` uses `floom_editor::core::{buffer::rope_text::RopeText, cursor::CursorAffinity, selection::Selection, ...}`.

- [ ] **Step 1: Write failing compile checkpoint**

Point examples to merged local crate and replace `floom_editor_core` with `floom_editor::core` in source/Cargo manifests.

- [ ] **Step 2: Run to verify failures**

Run: `cargo check -p editor -p syntax-editor -p ide` (from `crates/ecosystem/floom-editor`)
Expected: FAIL until paths/imports fixed.

- [ ] **Step 3: Fix manifests and imports**

Use `path = "../../floom-editor"` for the merged crate; update Rust imports; adjust any API differences introduced in Task 3 only as necessary.

- [ ] **Step 4: Verify**

Run: `cargo test -p floom-editor -p editor -p syntax-editor -p ide` or `cargo check`/targeted tests available in that workspace.
Expected: PASS/compiles.

- [ ] **Step 5: Commit**

```bash
git add crates/ecosystem/floom-editor/examples crates/ecosystem/floom-editor/Cargo.toml crates/ecosystem/floom-editor/Cargo.lock
git commit -m "refactor(editor): migrate satellite examples to merged floom-editor"
```

### Task 5: Migrate `examples/floom-showcase` to unified `TextEditor` API

**Files:**
- Modify: `examples/floom-showcase/Cargo.toml`
- Modify: `examples/floom-showcase/src/state.rs`
- Modify: `examples/floom-showcase/src/editor.rs`
- Modify: `examples/floom-showcase/src/pages.rs`
- Modify: `examples/floom-showcase/src/tests.rs`
- Modify: `examples/floom-showcase/src/main.rs` (docs/comments if needed)
- Test: `examples/floom-showcase` tests

**Interfaces:**
- Consumes: `floom_editor::{text_editor, TextEditor, EditorState, Document}` and any added convenience methods from Task 3; state must remain shareable across page mounts (`Send + Sync` builder constraints respected).
- Produces: showcase `/editor` page with autofocus, typing, undo/redo/reset working via unified editor; `SharedEditor` widget wrapper delegates to the unified editor instance; tests assert behavior (text changes after navigation/focus, devtools drawer non-interference) using new API.

- [ ] **Step 1: Write/adapt failing tests for new API**

Update `src/tests.rs` to read/mutate text and invoke undo/reset through `TextEditor`/`EditorState`/`Document` (exact calls matching Task 3 API) instead of `EditorWidget`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p floom-showcase`
Expected: FAIL (type/import/API mismatch).

- [ ] **Step 3: Implement migration**

Change dependency to merged satellite crate path; replace `Arc<Mutex<EditorWidget>>` with appropriate shared `TextEditor` (or `Arc<Mutex<Editor>>`/state+widget split) so one document survives navigation; keep `autofocus(true)` semantics; wire toolbar undo/redo/reset and `SharedEditor` to delegate Widget/WidgetAny methods to unified editor.

- [ ] **Step 4: Verify**

Run: `cargo test -p floom-showcase`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add examples/floom-showcase
git commit -m "refactor(showcase): use unified satellite floom-editor"
```

### Task 6: Remove redundant root crates and update root workspace/build metadata

**Files:**
- Delete: `crates/widgets/floom-editor/`
- Delete: `crates/widgets/floom-editor-core/`
- Modify: `Cargo.toml` (root)
- Modify: `Cargo.lock` (root)
- Modify: `scripts/check-architecture-deps.sh`
- Modify: `scripts/gate.sh`
- Modify: `.github/workflows/ci-core.yml`
- Modify: `.github/workflows/ci-plugins.yml` (if references remain)

**Interfaces:**
- Consumes: decisions from Tasks 1-5 (no remaining consumers of deleted crates).
- Produces: root workspace has no `widgets/floom-editor*` members/deps; architecture/gate/CI scripts no longer `--package` test removed crates but still validate merged satellite editor in ecosystem flow.

- [ ] **Step 1: Write the failing/checkpoint state**

Remove directories and root references; expect metadata/tests/scripts that still name deleted crates to fail until fully cleaned.

- [ ] **Step 2: Run to verify failures are understood**

Run: `cargo metadata --no-deps` (root) and/or targeted script checks as applicable.
Expected: FAIL on stale member/package refs until fixed.

- [ ] **Step 3: Implement cleanup**

Update `Cargo.toml` members/default-members/workspace deps, script package arrays and layer tables, CI fmt/clippy/test package lists and comments; regenerate locks as needed.

- [ ] **Step 4: Verify**

Run: `cargo metadata --no-deps`, `cargo test -p floom-showcase`, and `cargo test -p floom-editor --manifest-path crates/ecosystem/floom-editor/Cargo.toml` (or ecosystem-equivalent commands). Run `./scripts/check-architecture-deps.sh` if environment allows.
Expected: PASS/no stale crate references.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock scripts .github crates/widgets/floom-editor crates/widgets/floom-editor-core crates/ecosystem/floom-editor
git commit -m "chore(editor): remove redundant editor crates and update workspace tooling"
```

### Task 7: Documentation and final verification

**Files:**
- Modify: `docs/ecosystem.md`
- Modify: `crates/ecosystem/floom-editor/README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/workspace-conventions.md`
- Modify: `docs/examples.md`
- Modify: `CONTRIBUTING.md`
- Modify: `CHANGELOG.md`
- Modify: any other files with stale `floom-editor-core`/dual-package references found by search

**Interfaces:**
- Consumes: final merged structure and commands.
- Produces: docs describe one satellite `floom-editor` crate exposing `core` model + `TextEditor` API; no claims of two same-named packages.

- [ ] **Step 1: Write/update doc checks (grep-based)**

Run: `rg -n "widgets/floom-editor|floom-editor-core|Two packages are named .floom-editor" docs crates/ecosystem/floom-editor README* CONTRIBUTING.md CHANGELOG.md scripts .github -S`
Expected: matches found (to be removed/updated).

- [ ] **Step 2: Run to confirm stale references exist**

Capture list and ensure only intentional historical mentions remain (e.g. CHANGELOG entry describing this refactor, if desired).

- [ ] **Step 3: Update docs/comments**

Reflect merged crate, `floom_editor::core`, example paths, and ecosystem build instructions.

- [ ] **Step 4: Final verification**

Run: `rg -n "widgets/floom-editor|floom-editor-core" --glob '!docs/superpowers/plans/2026-11-16-merge-floom-editor-satellite.md'` (expect none outside historical changelog if kept), plus targeted builds/tests: `cargo test -p floom-showcase`, `cargo test --manifest-path crates/ecosystem/floom-editor/Cargo.toml`.
Expected: clean references and passing targeted tests.

- [ ] **Step 5: Commit**

```bash
git add docs crates/ecosystem/floom-editor/README.md CONTRIBUTING.md CHANGELOG.md
git commit -m "docs(editor): document merged satellite floom-editor crate"
```

## Notes for Implementer

- `crates/widgets/floom-editor/tests/editing.rs` covers simplified `Transaction/Selection/TextDocument` semantics that do not map 1:1 to `Buffer`/`Document`. Replace with equivalent coverage on `floom_editor::core` and/or `floom_editor::TextDocument`+`Document` APIs rather than preserving deleted types.
- `EditorWidget` in `widgets/floom-editor` is the reference for IME/focus behavior but uses a different paint/layout model (simple `TextLayout` per paint) than satellite `Editor` (cached line layouts, soft wrap, gutter). Port behavior, not structure.
- Showcase currently relies on `EditorWidget::text()`/`undo()`/`redo()`/`set_text()` and autofocus. Ensure the unified `TextEditor` (or exposed `EditorState`/`Document`) provides equivalent operations before deleting the old crate.
- Prefer small, verified commits per task; if a task must be split for buildability, keep each sub-commit compiling for its target workspace.
