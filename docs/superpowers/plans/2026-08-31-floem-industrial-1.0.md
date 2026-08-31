# Floem Industrial 1.0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
>
> **Spec:** `docs/superpowers/specs/2026-08-31-floem-industrial-1.0-design.md`
>
> **Scope note:** This plan is split into **five phases**. Each phase ends with a testable deliverable. Prefer finishing Phase N before starting Phase N+1. Phases 3–5 may be extracted into standalone plan files later if a different agent owns them.

**Goal:** Deliver floem 1.0 as a Rust desktop/editor-grade industrial core: in-tree `platform-compat/`, six-platform gate docs, Headless Testing API + inspector upgrades, editor extracted to a peer official repo, flome-ui kept satellite-only, progress tracked with jj.

**Architecture:** Keep floem thin (runtime + basic widgets + embedder/a11y/platform bridges). Vendor forks live under `platform-compat/`. Satellites (`flome-ui`, `floem-editor`) depend on floem public API only. Release gates are written checklists (tier B), not full device certification.

**Tech Stack:** Rust 2024, floem workspace, jj (colocated), Cargo `[patch]`, existing `HeadlessHarness` / `AccessTree`, criterion benches, platform examples under `examples/{android,ios,ohos}`.

## Global Constraints

- floem 1.0 commitment **excludes** flome-ui and floem-editor.
- Platforms Windows / macOS / Linux / Android / iOS / OHOS are **equal** commitment under tier-B gates.
- Gate status values are exactly `pass` / `fail` / `n/a`; tagging 1.0.0 requires **no `fail`**.
- No long-lived editor compatibility shim after extraction (delete `editor` feature).
- `platform-compat/` holds third-party forks only — no Floem widgets/reactive logic.
- Do not add floem → flome-ui or floem → floem-editor dependencies.
- Do not introduce a Floem DSL / full LSP live-preview in this plan.
- Daily progress: **jj** (`jj describe` / `jj new`); use `jj git push` only when syncing remotes. No agent Co-authored-by trailers.
- No backward-compat shims for removed APIs (project rule).

---

## File Structure (locked)

| Path | Responsibility |
|------|----------------|
| `platform-compat/README.md` | Catalog of vendor trees, upstream, exit criteria |
| `platform-compat/floem-winit/` | iOS openURL winit fork (from `floem_packages/floem-winit-ios-fork`) |
| `platform-compat/tgui-winit-ohos/` | OHOS retarget fork + `patch/` |
| `platform-compat/harmony-app-rs/` | Packaging patches + USAGE only |
| `Cargo.toml` `[patch]` | In-repo paths only |
| `docs/gates/{windows,macos,linux,android,ios,ohos}.md` | Tier-B checklists |
| `docs/ecosystem.md` | floem ↔ satellite version matrix |
| `src/headless.rs` | Public a11y query/action Testing API on harness |
| `src/window/handle.rs` | Expose read access to neutral `AccessTree` for tests |
| `src/inspector/*` | Show a11y tree + capabilities |
| `justfile` (create) | `gate-*` helpers |
| `../floem-editor/` (new repo) | Extracted editor-core + editor views + examples |
| `../flome-ui/Cargo.toml` | Patch path → `../floem` |

---

## Phase map

| Phase | Deliverable | Tasks below |
|-------|-------------|-------------|
| **1** | `platform-compat/` imported; patches rewired; old0295 gone | Tasks 1–3 |
| **2** | Gate skeletons + ecosystem doc + Testing API + just recipes | Tasks 4–7 |
| **3** | Inspector a11y/capabilities panel | Task 8 |
| **4** | floem-editor extracted; floem `editor` feature removed | Tasks 9–11 |
| **5** | flome-ui path fix; Android safe-area; gate fills toward 1.0 | Tasks 12–14 |

---

### Task 1: Import vendor trees into `platform-compat/`

**Files:**
- Create: `platform-compat/README.md`
- Create: `platform-compat/floem-winit/` (copy)
- Create: `platform-compat/tgui-winit-ohos/` (copy, including patch docs)
- Create: `platform-compat/harmony-app-rs/` (copy)
- Delete/ignore: do **not** copy `floem_packages/floem-winit-ios-fork.old0295`

**Interfaces:**
- Consumes: `/Users/humphrey/Project/Rust/floem_packages/{floem-winit-ios-fork,tgui-winit-ohos-fork,tgui-winit-ohos-patch,harmony-app-rs-patch}`
- Produces: in-tree directories listed above; nested `.git` directories removed

- [ ] **Step 1: Create destination directories**

```bash
mkdir -p platform-compat
```

- [ ] **Step 2: Copy forks without nested git metadata**

```bash
# From floem repo root
rsync -a --exclude '.git' \
  ../floem_packages/floem-winit-ios-fork/ \
  platform-compat/floem-winit/

rsync -a --exclude '.git' \
  ../floem_packages/tgui-winit-ohos-fork/ \
  platform-compat/tgui-winit-ohos/

mkdir -p platform-compat/tgui-winit-ohos/patch
rsync -a --exclude '.git' \
  ../floem_packages/tgui-winit-ohos-patch/ \
  platform-compat/tgui-winit-ohos/patch/

rsync -a --exclude '.git' \
  ../floem_packages/harmony-app-rs-patch/ \
  platform-compat/harmony-app-rs/
```

- [ ] **Step 3: Verify expected entrypoints exist**

```bash
test -f platform-compat/floem-winit/winit/Cargo.toml
test -f platform-compat/tgui-winit-ohos/Cargo.toml
test -f platform-compat/tgui-winit-ohos/patch/retarget-to-floem-winit-core.patch
test -f platform-compat/harmony-app-rs/USAGE.md
test ! -d platform-compat/floem-winit-ios-fork.old0295
```

Expected: all `test` commands exit 0.

- [ ] **Step 4: Write `platform-compat/README.md`**

Create `platform-compat/README.md` with exactly these sections (fill URLs/revs from the copied READMEs):

```markdown
# platform-compat

Vendor windowing/packaging trees used by Floem. Not a public Rust API.

| Tree | Upstream | Local delta | Exit criteria |
|------|----------|-------------|---------------|
| `floem-winit/` | lapce/winit @ 133268de | iOS `set_on_open_url` / share cold-start | Upstream merged; iOS share gate `pass`; then delete tree and `[patch]` |
| `tgui-winit-ohos/` | tgui-winit-ohos 0.4.x | `floem_winit_core` retarget + `!Send` on RuntimeApplication | Upstream supports floem-winit-core; OHOS example gate `pass` |
| `harmony-app-rs/` | taoyuan11/harmony-app-rs | macOS/Linux hvigor wrappers | Upstream accepts patch; keep only docs link |

Rules:
- No Floem widgets or reactive code here.
- Pin `dpi` / winit-core identity as documented in each tree README.
- CHANGELOG: label commits under **platform-compat**.
```

- [ ] **Step 5: jj describe**

```bash
jj describe -m "chore: import platform-compat vendor trees from floem_packages"
```

---

### Task 2: Rewire Cargo `[patch]` to in-repo paths

**Files:**
- Modify: `Cargo.toml` (patch section near end of file)
- Modify: any docs that still say `../tgui-winit-ohos-fork` or `../floem-winit-ios-fork` (`docs/ohos.md`, `docs/ios.md`)

**Interfaces:**
- Consumes: Task 1 paths
- Produces: `[patch]` resolving inside this repo

- [ ] **Step 1: Replace patch paths in `Cargo.toml`**

Change the existing block:

```toml
[patch.crates-io]
tgui-winit-ohos = { path = "../tgui-winit-ohos-fork" }

[patch."https://github.com/lapce/winit"]
floem-winit = { path = "../floem-winit-ios-fork/winit" }
```

to:

```toml
[patch.crates-io]
tgui-winit-ohos = { path = "platform-compat/tgui-winit-ohos" }

[patch."https://github.com/lapce/winit"]
floem-winit = { path = "platform-compat/floem-winit/winit" }
```

Also update the long comments above those patches so they reference `platform-compat/` (not `../floem_packages` or sibling forks).

- [ ] **Step 2: Update docs path references**

In `docs/ohos.md` and `docs/ios.md`, replace mentions of `../tgui-winit-ohos-fork`, `../floem-winit-ios-fork`, and `floem_packages` with `platform-compat/...`.

- [ ] **Step 3: Verify desktop resolve**

```bash
cargo metadata --no-deps --format-version 1 >/dev/null
cargo check -p floem
```

Expected: success (no missing path patch errors).

- [ ] **Step 4: jj describe**

```bash
jj describe -m "build: point Cargo patches at platform-compat"
```

---

### Task 3: Archive guidance for external `floem_packages`

**Files:**
- Create: `platform-compat/MIGRATION.md`
- Optional (outside floem): add `DEPRECATED.md` in `floem_packages` if that repo remains

**Interfaces:**
- Produces: single source of truth statement

- [ ] **Step 1: Write migration note**

Create `platform-compat/MIGRATION.md`:

```markdown
# Migration from floem_packages

As of 2026-08-31, Floem vendors live in-tree under `platform-compat/`.

Do not use `/Users/humphrey/Project/Rust/floem_packages` for new work.
Safe to archive or delete that directory after confirming:
- `cargo check -p floem` works
- iOS/OHOS example docs build steps use `platform-compat/` paths
```

- [ ] **Step 2: jj describe**

```bash
jj describe -m "docs: record floem_packages → platform-compat migration"
```

---

### Task 4: Gate skeleton documents (six platforms)

**Files:**
- Create: `docs/gates/windows.md`
- Create: `docs/gates/macos.md`
- Create: `docs/gates/linux.md`
- Create: `docs/gates/android.md`
- Create: `docs/gates/ios.md`
- Create: `docs/gates/ohos.md`
- Create: `docs/gates/README.md`

**Interfaces:**
- Produces: checklist templates with status column; initially all rows `fail` or `n/a` as appropriate (desktop safe-area = `n/a`)

- [ ] **Step 1: Write `docs/gates/README.md`**

```markdown
# Floem 1.0 platform gates (tier B)

Status values: `pass` | `fail` | `n/a`.

Tagging `1.0.0` requires **no `fail`** in any platform file.
Simulator/emulator allowed; record device spot-checks in the "Sign-off" section.
```

- [ ] **Step 2: Create each platform file from this template**

For `docs/gates/macos.md` (repeat for windows/linux/android/ios/ohos; adjust `n/a` rows):

```markdown
# Gate: macOS

| # | Item | Status | Evidence |
|---|------|--------|----------|
| 1 | Official example starts + basic interaction | fail | |
| 2 | IME Latin + one CJK | fail | |
| 3 | a11y label + Default action | fail | |
| 4 | Clipboard read/write | fail | |
| 5 | File pick / share-open as applicable | fail | |
| 6 | Safe area insets | n/a | desktop |
| 7 | Focus + basic keyboard nav | fail | |
| 8 | CI/target compile | fail | |

## Sign-off

- Spot-check date:
- Environment (device/simulator):
- Reviewer:
```

For android/ios/ohos: item 6 starts as `fail` (not `n/a`).

- [ ] **Step 3: jj describe**

```bash
jj describe -m "docs: add tier-B platform gate skeletons"
```

---

### Task 5: Ecosystem compatibility doc

**Files:**
- Create: `docs/ecosystem.md`
- Modify: `README.md` — one short paragraph linking to ecosystem + stating flome-ui/editor are satellites

**Interfaces:**
- Produces: version matrix table

- [ ] **Step 1: Write `docs/ecosystem.md`**

```markdown
# Floem ecosystem

Official satellites (not part of floem 1.0 core promise):

| Satellite | Role | Depends on |
|-----------|------|------------|
| [flome-ui](https://github.com/) | Material / Cupertino / Fluent kit (`floem-elements`) | `floem ^1` |
| floem-editor | Code/text editor views + editor-core | `floem ^1` |

## Compatibility matrix

| floem | flome-ui | floem-editor | Notes |
|-------|----------|--------------|-------|
| 0.2.x | path/git | in-tree `editor` feature | pre-1.0 |
| 1.0.0 | (fill on release) | (fill on release) | |

Update this table when tagging floem or publishing satellites.
```

(Use real flome-ui remote URL if known; otherwise leave path `../flome-ui`.)

- [ ] **Step 2: Add README pointer**

Near Features in `README.md`, add:

```markdown
## Ecosystem

Design-system widgets live in the separate **flome-ui** project. The rich text/code editor will live in **floem-editor**. Neither is part of the floem 1.0 core guarantee — see [docs/ecosystem.md](docs/ecosystem.md).
```

- [ ] **Step 3: jj describe**

```bash
jj describe -m "docs: add ecosystem matrix for satellite crates"
```

---

### Task 6: Expose `AccessTree` to headless tests

**Files:**
- Modify: `src/window/handle.rs` — add `pub fn access_tree(&self) -> &AccessTree` (or clone API)
- Modify: `src/window/state.rs` if a getter is cleaner
- Test: `test/tests/access_tree_harness.rs` (new)

**Interfaces:**
- Consumes: `WindowState.accessibility: AccessTree` (today `pub(crate)`)
- Produces:
  - `WindowHandle::access_tree(&self) -> &crate::accessibility::AccessTree`
  - Later Task 7 builds queries on this

- [ ] **Step 1: Write failing integration test**

Create `test/tests/access_tree_harness.rs`:

```rust
use floem::accessibility::AccessRole;
use floem::headless::{HeadlessHarness, TestRoot};
use floem::views::{button, Decorators};
use floem::IntoView;

#[test]
fn harness_exposes_access_tree_with_button_label() {
    let root = TestRoot::new();
    let view = button("Save")
        .style(|s| s.size(80.0, 32.0))
        .into_view();
    // If Button does not yet set accessibility_label by default, set on id after creation
    // via a wrapper — for this test, use a labeled empty+button pattern once API exists.
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 100.0);
    harness.rebuild();
    let tree = harness.access_tree();
    assert!(
        tree.iter().any(|(_, n)| n.label.as_deref() == Some("Save") || n.role == AccessRole::Button),
        "expected a button node in access tree, got len={}",
        tree.len()
    );
}
```

If `AccessTree::iter` / `label` field names differ, match `src/accessibility/tree.rs` and `node.rs` exactly when implementing.

- [ ] **Step 2: Run test — expect fail**

```bash
cargo test -p floem-test --test access_tree_harness -- --nocapture
```

Expected: compile error `no method named access_tree` OR assertion fail if method stubbed.

- [ ] **Step 3: Implement `WindowHandle::access_tree` + `HeadlessHarness::access_tree`**

Minimal shape:

```rust
// on WindowHandle
pub fn access_tree(&self) -> &crate::accessibility::AccessTree {
    &self.window_state.borrow().accessibility
}

// on HeadlessHarness
pub fn access_tree(&self) -> &crate::accessibility::AccessTree {
    self.window_handle().access_tree()
}
```

Ensure the harness rebuild path calls the same accessibility tree update used by real windows (`update_accessibility_tree` or equivalent). If headless currently skips a11y rebuild, call that update from `rebuild` / `process_update_messages` before returning the tree.

- [ ] **Step 4: Ensure default button role/label**

If the test fails because Button has no a11y attrs, set defaults inside `src/views/button.rs` when the view is created:

```rust
id.accessibility_role(AccessRole::Button);
id.accessibility_label(label_string);
id.accessibility_actions([AccessAction::Default]);
```

(Use the actual Button construction site; do not add a compatibility wrapper.)

- [ ] **Step 5: Re-run test — expect pass**

```bash
cargo test -p floem-test --test access_tree_harness -- --nocapture
```

Expected: `PASS`

- [ ] **Step 6: jj describe**

```bash
jj describe -m "feat: expose AccessTree on HeadlessHarness for testing"
```

---

### Task 7: Headless Testing API — find by label + perform action

**Files:**
- Modify: `src/headless.rs`
- Modify: `src/accessibility/tree.rs` if helpers belong there
- Test: `test/tests/access_testing_api.rs`

**Interfaces:**
- Consumes: Task 6 `access_tree()`
- Produces:
  - `HeadlessHarness::find_by_accessible_label(&self, label: &str) -> Option<AccessNodeId>`
  - `HeadlessHarness::find_by_accessible_role(&self, role: AccessRole) -> Vec<AccessNodeId>`
  - `HeadlessHarness::perform_access_action(&mut self, id: AccessNodeId, action: AccessAction) -> bool`
  - Prefer implementing action via existing `WindowState::handle_accessibility_action`

- [ ] **Step 1: Write failing tests**

Create `test/tests/access_testing_api.rs`:

```rust
use floem::accessibility::{AccessAction, AccessRole};
use floem::headless::{HeadlessHarness, TestRoot};
use floem::reactive::RwSignal;
use floem::views::{button, label, stack, Decorators};
use floem::IntoView;

#[test]
fn find_by_accessible_label_clicks_button() {
    let root = TestRoot::new();
    let clicked = RwSignal::new(false);
    let view = button("Go").action(move || clicked.set(true));
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 80.0);
    harness.rebuild();
    let id = harness
        .find_by_accessible_label("Go")
        .expect("label Go");
    assert!(harness.perform_access_action(id, AccessAction::Default));
    harness.process_update_messages();
    assert!(clicked.get());
}

#[test]
fn find_by_role_returns_button() {
    let root = TestRoot::new();
    let view = button("X");
    let mut harness = HeadlessHarness::new_with_size(root, view, 200.0, 80.0);
    harness.rebuild();
    let ids = harness.find_by_accessible_role(AccessRole::Button);
    assert!(!ids.is_empty());
}
```

- [ ] **Step 2: Run — expect fail**

```bash
cargo test -p floem-test --test access_testing_api -- --nocapture
```

Expected: missing methods.

- [ ] **Step 3: Implement methods on `HeadlessHarness`**

```rust
pub fn find_by_accessible_label(&self, label: &str) -> Option<crate::accessibility::AccessNodeId> {
    self.access_tree().iter().find_map(|(id, node)| {
        (node.label.as_deref() == Some(label)).then_some(*id)
    })
}

pub fn find_by_accessible_role(
    &self,
    role: crate::accessibility::AccessRole,
) -> Vec<crate::accessibility::AccessNodeId> {
    self.access_tree()
        .iter()
        .filter_map(|(id, node)| (node.role == role).then_some(*id))
        .collect()
}

pub fn perform_access_action(
    &mut self,
    id: crate::accessibility::AccessNodeId,
    action: crate::accessibility::AccessAction,
) -> bool {
    use crate::accessibility::PendingAction;
    self.window_handle_mut()
        .window_state
        .borrow_mut()
        .handle_accessibility_action(PendingAction { node: id, action, payload: None })
}
```

Adapt `PendingAction` field names to `src/accessibility/backend.rs`. If `handle_accessibility_action` is `pub(crate)` on `WindowState`, add a `WindowHandle::perform_access_action` wrapper.

Match real `AccessTree::iter` signature — if it yields `(&AccessNodeId, &AccessNode)` adjust accordingly.

- [ ] **Step 4: Run — expect pass**

```bash
cargo test -p floem-test --test access_testing_api -- --nocapture
```

Expected: `PASS`

- [ ] **Step 5: jj describe**

```bash
jj describe -m "feat: headless find/perform accessibility testing API"
```

---

### Task 8: justfile gate helpers + inspector a11y readout

**Files:**
- Create: `justfile`
- Modify: `src/inspector/data.rs`, `src/inspector/view.rs` (or equivalent UI)
- Test: manual `cargo run -p widget-gallery` + inspector; unit test for data model if one exists

**Interfaces:**
- Consumes: Task 4 gate files; Task 6 tree
- Produces: `just gate-desktop` runs `cargo test -p floem-test --test access_testing_api` + `cargo check -p floem`; inspector shows a11y nodes for selected view

- [ ] **Step 1: Create `justfile`**

```just
gate-desktop:
    cargo check -p floem
    cargo test -p floem-test --test access_tree_harness
    cargo test -p floem-test --test access_testing_api

gate-docs:
    test -f docs/gates/windows.md
    test -f docs/gates/macos.md
    test -f docs/gates/linux.md
    test -f docs/gates/android.md
    test -f docs/gates/ios.md
    test -f docs/gates/ohos.md
```

- [ ] **Step 2: Run just recipes**

```bash
just gate-docs
just gate-desktop
```

Expected: both succeed.

- [ ] **Step 3: Inspector — add a11y fields to inspector data**

In `src/inspector/data.rs` (follow existing patterns for layout fields), add optional:

```rust
pub access_role: Option<String>,
pub access_label: Option<String>,
```

Populate when building inspector node from `ViewState.accessibility` / access tree.

In `src/inspector/view.rs`, render label/role under the selected node details.

- [ ] **Step 4: Smoke the inspector**

```bash
cargo run -p widget-gallery
```

Open inspector (existing shortcut/menu). Select a Button. Confirm role/label visible.

- [ ] **Step 5: jj describe**

```bash
jj describe -m "feat: just gate helpers and inspector a11y readout"
```

---

### Task 9: Create `floem-editor` repo skeleton

**Files (new repo `/Users/humphrey/Project/Rust/floem-editor`):**
- Create: `Cargo.toml` workspace
- Create: `README.md`
- Create: `.gitignore`

**Interfaces:**
- Produces: empty workspace depending on `floem = { path = "../floem" }`

- [ ] **Step 1: Init repo with jj/git**

```bash
mkdir -p /Users/humphrey/Project/Rust/floem-editor
cd /Users/humphrey/Project/Rust/floem-editor
git init
jj git init --colocate
```

- [ ] **Step 2: Root manifests**

`Cargo.toml`:

```toml
[workspace]
resolver = "3"
members = ["floem-editor-core", "floem-editor", "examples/editor", "examples/syntax-editor"]

[workspace.package]
edition = "2024"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
floem = { path = "../floem", default-features = false, features = ["default-image-formats", "vger", "serde"] }
```

`README.md` must state: official satellite; not part of floem 1.0 core promise; see floem `docs/ecosystem.md`.

- [ ] **Step 3: jj describe**

```bash
jj describe -m "chore: scaffold floem-editor workspace"
```

---

### Task 10: Move editor code into floem-editor

**Files:**
- Move: `floem/editor-core/**` → `floem-editor/floem-editor-core/**`
- Move: `floem/src/views/editor/**`, `floem/src/views/text_editor.rs` → `floem-editor/floem-editor/src/**`
- Move: `floem/examples/editor`, `floem/examples/syntax-editor` → `floem-editor/examples/**`
- Modify: floem `Cargo.toml`, `src/views/mod.rs`, `src/lib.rs`, workspace members
- Modify: floem-editor crates’ `Cargo.toml` to depend on public floem APIs only

**Interfaces:**
- Consumes: floem public `View`, style, events, IME
- Produces: `floem-editor` crate exporting editor views; floem no longer compiles editor modules

- [ ] **Step 1: Copy trees then fix imports**

```bash
cp -R editor-core /Users/humphrey/Project/Rust/floem-editor/floem-editor-core
mkdir -p /Users/humphrey/Project/Rust/floem-editor/floem-editor/src
# move editor view modules into floem-editor/src following existing module layout
```

Replace `floem::` internal `pub(crate)` uses with public APIs. If a hook is missing, add a **documented public** API on floem in a prior jj change — do not depend on `pub(crate)`.

- [ ] **Step 2: Remove editor from floem**

In floem `Cargo.toml`:
- Remove `editor-core` from `[workspace] members` / `default-members`
- Remove `floem-editor-core` dependency
- Remove `editor` feature and from `default`
- Remove editor-related benches if they only target moved code (or move benches)

In `src/views/mod.rs` / `src/lib.rs`: delete `editor` / `text_editor` modules.

- [ ] **Step 3: Verify builds**

```bash
cd /Users/humphrey/Project/Rust/floem && cargo check -p floem
cd /Users/humphrey/Project/Rust/floem-editor && cargo check -p floem-editor
```

Expected: both succeed.

- [ ] **Step 4: Update `docs/ecosystem.md` paths and floem README examples that imported editor**

- [ ] **Step 5: jj describe in both repos**

```bash
# floem
jj describe -m "refactor: remove editor feature; editor lives in floem-editor"

# floem-editor
jj describe -m "feat: import editor-core and editor views from floem"
```

---

### Task 11: Default features sanity after editor removal

**Files:**
- Modify: `Cargo.toml` default features
- Modify: examples that used `editor` feature

**Interfaces:**
- Produces: `cargo run -p counter` and `cargo run -p widget-gallery` work with new defaults

- [ ] **Step 1: Set defaults without editor**

```toml
default = ["default-image-formats", "vger", "serde"]
```

- [ ] **Step 2: Run smoke examples**

```bash
cargo run -p counter
cargo run -p widget-gallery
```

Expected: launch OK (manual quit).

- [ ] **Step 3: jj describe**

```bash
jj describe -m "build: drop editor from floem default features"
```

---

### Task 12: Point flome-ui at local floem

**Files:**
- Modify: `/Users/humphrey/Project/Rust/flome-ui/Cargo.toml` `[patch]`

**Interfaces:**
- Produces: flome-ui resolves floem from `../floem`

- [ ] **Step 1: Fix patch**

Replace:

```toml
[patch."https://github.com/lapce/floem"]
floem = { path = "/Users/ahs/Project/Rust/floem" }
```

with:

```toml
[patch."https://github.com/lapce/floem"]
floem = { path = "../floem" }
```

- [ ] **Step 2: Check demo crate**

```bash
cd /Users/humphrey/Project/Rust/flome-ui
cargo check -p floem-demo --features material
```

Expected: compiles, or only flome-ui-local errors unrelated to path resolution. Fix path-related errors only in this task.

- [ ] **Step 3: jj describe in flome-ui**

```bash
jj git init --colocate  # if needed
jj describe -m "build: patch floem to ../floem"
```

---

### Task 13: Android safe area → `EmbedderEvent::SafeArea`

**Files:**
- Modify: `src/app/handle.rs` or Android embedder path that already handles iOS safe area
- Modify: `src/platform/android_*.rs` or `src/embedder/android.rs`
- Modify: `docs/gates/android.md` item 6 evidence when verified
- Reference: iOS path that emits `EmbedderEvent::SafeArea` / `#110` `apply_safe_area`

**Interfaces:**
- Consumes: existing `EmbedderEvent::SafeArea` and layout consumer
- Produces: Android inset reader setting the same event

- [ ] **Step 1: Locate iOS emitter**

```bash
rg "SafeArea" src/app src/embedder src/platform
```

- [ ] **Step 2: Write a unit/doc test or headless-style test if insets can be injected**

If events can be injected via harness/embedder:

```rust
#[test]
fn safe_area_event_updates_layout_insets() {
    // dispatch EmbedderEvent::SafeArea { .. } through the same path Android will use
    // assert window/layout inset fields match
}
```

If only on-device: add a debug assert log path and mark gate evidence manually.

- [ ] **Step 3: Implement Android `WindowInsets` → SafeArea**

Mirror iOS: on configuration/insets change, send `EmbedderEvent::SafeArea` with top/right/bottom/left in logical px.

- [ ] **Step 4: Update `docs/gates/android.md` item 6 when spot-checked**

- [ ] **Step 5: jj describe**

```bash
jj describe -m "feat(android): emit SafeArea insets into embedder"
```

---

### Task 14: Fill gates toward 1.0 (process task)

**Files:**
- Modify: `docs/gates/*.md`
- Modify: `CHANGELOG.md` when tagging
- Modify: `Cargo.toml` version → `1.0.0` only when gates clear

**Interfaces:**
- Consumes: Tasks 1–13 deliverables + on-device/simulator runs
- Produces: all gate rows `pass` or justified `n/a`; then version bump

- [ ] **Step 1: Run `just gate-desktop` and mark desktop gate rows**

Update windows/macos/linux gate files with evidence links (CI logs, dates).

- [ ] **Step 2: Run mobile example checklists from `docs/cross_platform_a11y_ime_test.md`**

Fill android/ios/ohos gate files. OHOS item 8 may remain documented exception only if the template’s rules in the spec still allow a reproducible path — otherwise keep `fail` and **do not tag 1.0**.

- [ ] **Step 3: Upstream tracking issues**

Open/track:
- lapce/winit PR for iOS openURL (exit criterion for deleting `platform-compat/floem-winit`)
- optional tgui `!Send` / floem-winit-core support

- [ ] **Step 4: When no `fail` remains, bump version**

```toml
# workspace.package / package version
version = "1.0.0"
```

Update `CHANGELOG.md` and `docs/ecosystem.md` matrix.

- [ ] **Step 5: jj describe + bookmark**

```bash
jj describe -m "release: floem 1.0.0"
jj bookmark set floem-1.0.0 -r @
# jj git push when ready to publish
```

---

## Self-review (spec coverage)

| Spec section | Tasks |
|--------------|-------|
| §1 platform-compat in-tree | 1–3 |
| §2 commitment + gates | 4, 14 |
| §2b Testing API / inspector / just / jj / ecosystem | 5–8 |
| §3 upstream/exit criteria | 3, 14 (tracking); iOS upstream not blocking import |
| §4 floem-editor extract + flome-ui patch | 9–12 |
| §5 jj workflow | all describe steps |
| Android safe area gap | 13 |
| Non-goal: no DSL / no kit in 1.0 promise | enforced in constraints + Task 5 docs |

**Placeholder scan:** none intentional. Adapt exact `PendingAction` / `AccessTree::iter` field names to source when implementing.

**Type consistency:** `AccessNodeId`, `AccessRole`, `AccessAction`, `HeadlessHarness::access_tree` shared across Tasks 6–7.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-08-31-floem-industrial-1.0.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks  
2. **Inline Execution** — execute in this session with executing-plans checkpoints  

Which approach?
