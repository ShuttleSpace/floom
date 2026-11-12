# floom-demo Gallery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `examples/floom-demo`, a runnable gallery that mounts all 98
`floom-ui` components in all three design systems side by side, with derived
props tables and a headless test that mounts every page.

**Architecture:** A new root-workspace example crate. All 294 kit crates
(98 components × Material/Cupertino/Fluent) are ordinary path dependencies, so
one binary contains all three design systems. A small `ds` module aliases each
component's per-design-system type, and a `compare_children!` macro
instantiates one written sample body three times — once per design system, each
with a theme pinned to it — producing the three columns of a compare row. The
gallery's props tables are parsed at runtime out of
`floom-core-theme`'s own props sources, so they cannot drift from the library.

**Tech Stack:** Rust 2024 (MSRV 1.91), the `floom` umbrella crate
(`desktop` + `renderer-gpu` + `devtools` features), `floom-widgets`,
`floom-testkit` for headless tests, `floom-core-theme` from the satellite.

**Spec:** `docs/superpowers/specs/2026-11-11-floom-demo-gallery-design.md`
(read it before Task 1 — it carries the constraints and the evidence for
every decision below).

## Global Constraints

- The crate is `examples/floom-demo`, package name `floom-demo`,
  `publish = false`, at the **root** workspace — never inside
  `crates/ecosystem/floom-ui` (that workspace stays headless).
- **No new third-party dependencies.** Everything comes from crates already in
  this repository.
- All three design systems are compiled into the one binary. The crate has
  **no Cargo features** of its own, so `cargo check --workspace --all-features`
  (the CI MSRV job) has nothing to collide.
- The **only** change to the satellite is Task 1's additive
  `MultiTheme::pinned`. No other file under `crates/ecosystem/` may change,
  except the `justfile` repair in Task 14.
- Rust edition 2024, `rust-version = "1.91"`, toolchain pinned to 1.99.1 by
  `rust-toolchain.toml`.
- Clippy must run on the pinned toolchain: the bare `cargo`/`rustc` on this
  machine is 1.98.1 but `clippy-driver` is 1.99.1. Prefix clippy commands with
  `PATH="/Users/humphrey/.rustup/toolchains/1.99.1-aarch64-apple-darwin/bin:$PATH"`.
- Commit messages follow Conventional Commits with a scope, as the recent
  history does (`feat(cli): …`, `docs: …`).
- Every task ends with a commit. Do not push without explicit instruction.

## Review Focus

Five conditions a reasonable person would expect to work, that no task's
happy-path test exercises. Each line's test is added to the task named.

1. **A component whose builder methods differ between the three design
   systems.** The `compare_children!` body is written once and instantiated
   three times; if one design system's kit spells a builder differently, the
   sample silently compiles for two arms and not the third. Task 5's test must
   assert all three arms compile *and* produce three distinct widget instances.
2. **A component whose entry terminal takes arguments** (`item_group`'s
   `view(self, Option<impl Fn(usize) …>, Option<impl Fn(Vec<usize>) …>)`).
   Task 5 must use `item_group` as the argument-taking case so the macro's
   contract is proven, not assumed.
3. **A kit crate with more than one `pub` struct** (most have a `…Style`
   companion; `toolbar` also has `ToolbarTokensHeightNote`). The generator
   picks the type by brace-matched `impl` scan, and Task 2's test must fail
   if that pick is ever ambiguous.
4. **A component that renders nothing on its own** (the six providers,
   `item_group`). Its compare row would be three empty columns. Task 11 must
   assert such a page states that in its note rather than showing a blank.
5. **A component present in one design system but not another.**
   `cupertino-icon-paths` / `fluent-icon-paths` exist only in the latter two
   and are data directories, not crates. Task 2's generator must emit exactly
   the 98-component intersection and no alias for those.

---

## File Structure

```
crates/ecosystem/floom-ui/crates/floom-core-theme/src/theme.rs   # MODIFIED (Task 1)
crates/ecosystem/floom-ui/crates/floom-core-theme/tests/design_system_themes.rs  # MODIFIED (Task 1)
crates/ecosystem/floom-ui/justfile                               # MODIFIED (Task 14)

examples/floom-demo/
├── Cargo.toml                 # GENERATED (Task 2) — 294 path deps
├── README.md                  # Task 14
├── tools/gen.py               # Task 2 — the generator
└── src/
    ├── main.rs                # Task 4 — entry point
    ├── api.rs                 # Task 3 — ApiRow / ApiDoc
    ├── macros.rs              # Task 5 — compare_children!
    ├── ds.rs                  # GENERATED (Task 2) — 3 namespaces of aliases
    ├── props_src.rs           # GENERATED (Task 2) — include_str! + parser
    ├── registry.rs            # Task 3 — the 98-page catalogue
    ├── shell.rs               # Task 4 — nav rail + scrolling detail pane
    ├── widgets.rs             # Task 4 — chrome: labels, tables, compare row
    └── pages/
        ├── mod.rs             # Task 3 — Category, ComponentPage, Sample
        ├── actions.rs         # Task 6 — 18 components
        ├── forms.rs           # Task 7 — 16
        ├── data.rs            # Task 8 — 21
        ├── feedback.rs        # Task 9 — 9
        ├── layout.rs          # Task 10 — 19
        └── motion.rs          # Task 11 — 15
```

---

### Task 1: `MultiTheme::pinned` in the satellite

**Files:**
- Modify: `crates/ecosystem/floom-ui/crates/floom-core-theme/src/theme.rs`
  (the `MultiTheme` struct at ~line 493 and its `impl` at ~line 499)
- Test: `crates/ecosystem/floom-ui/crates/floom-core-theme/tests/design_system_themes.rs`

**Interfaces:**
- Consumes: `floom_core_theme::global_theme::{current_design_system, DesignSystem}`
  (already imported in `theme.rs`).
- Produces:
  ```rust
  impl MultiTheme {
      /// Build a theme pinned to one design system, ignoring the process-global one.
      pub fn pinned(ds: DesignSystem) -> Self;
  }
  ```
  and a private field `pinned: Option<DesignSystem>` on `MultiTheme`.

**Notes for the implementer.** `MultiTheme` today holds `material`, `cupertino`,
`fluent` and its `ds()` returns `current_design_system()` — the process global.
`activate()` is a documented no-op. The change is to add a fourth field
`pinned: Option<DesignSystem>`, set it to `None` in `new_for`, add `pinned(ds)`
which sets it to `Some(ds)`, and change `ds()` to return the pinned value when
present. **`new_for` must keep its current behaviour** (the argument stays
ignored) so the existing test
`multi_theme_reads_follow_the_process_global_not_its_own_argument` still
passes. `activate()` stays a no-op. `MultiTheme` derives `Clone`; the new
field must not break that.

- [ ] **Step 1: Write the failing test**

  Add to `crates/ecosystem/floom-ui/crates/floom-core-theme/tests/design_system_themes.rs`:

  ```rust
  /// A pinned theme reads its own design system regardless of the global.
  #[test]
  fn a_pinned_theme_ignores_the_process_global() {
      design_system_signal().set(DesignSystem::Material);
      let pinned_to_cupertino = MultiTheme::pinned(DesignSystem::Cupertino);

      design_system_signal().set(DesignSystem::Material);
      let while_global_is_material = pinned_to_cupertino.colors().surface_variant;

      design_system_signal().set(DesignSystem::Fluent);
      let while_global_is_fluent = pinned_to_cupertino.colors().surface_variant;

      assert_eq!(
          while_global_is_material,
          Color::rgb(242, 242, 247),
          "a Cupertino-pinned theme must stay Cupertino while the global says Material"
      );
      assert_eq!(
          while_global_is_fluent,
          Color::rgb(242, 242, 247),
          "and must stay Cupertino while the global says Fluent"
      );
  }
  ```

- [ ] **Step 2: Run the test to verify it fails**

  Run:
  `cd crates/ecosystem/floom-ui && cargo test -p floom-core-theme --test design_system_themes`

  Expected: FAIL — `MultiTheme::pinned` does not exist (E0599).

- [ ] **Step 3: Implement `MultiTheme::pinned`**

  In `crates/ecosystem/floom-ui/crates/floom-core-theme/src/theme.rs`:
  - add the field `pinned: Option<DesignSystem>` to `pub struct MultiTheme`;
  - initialise it to `None` in `new_for`;
  - add `pub fn pinned(ds: DesignSystem) -> Self` building the three concrete
    themes and setting `pinned: Some(ds)`;
  - change `fn ds(&self) -> DesignSystem` to return the pinned value when it is
    `Some`, otherwise `current_design_system()`.

  Do not change `new_for`, `activate`, or any of the `Theme` trait methods.

- [ ] **Step 4: Run the test to verify it passes**

  Run:
  `cd crates/ecosystem/floom-ui && cargo test -p floom-core-theme --test design_system_themes`

  Expected: PASS — 4 tests, including the pre-existing
  `multi_theme_reads_follow_the_process_global_not_its_own_argument`.

- [ ] **Step 5: Run the whole satellite gate**

  Run: `just gate ecosystem`

  Expected: PASS — all 306 satellite members still build and test green. The
  change is additive, so nothing may regress; if anything does, the pinned
  field must not be consulted when it is `None`.

- [ ] **Step 6: Commit**

  ```bash
  git add crates/ecosystem/floom-ui/crates/floom-core-theme/src/theme.rs \
          crates/ecosystem/floom-ui/crates/floom-core-theme/tests/design_system_themes.rs
  git commit -m "feat(theme): add MultiTheme::pinned for per-instance design systems"
  ```

---

### Task 2: The generator and the three generated files

**Files:**
- Create: `examples/floom-demo/tools/gen.py`
- Create (generated): `examples/floom-demo/Cargo.toml`
- Create (generated): `examples/floom-demo/src/ds.rs`
- Create (generated): `examples/floom-demo/src/props_src.rs`
- Create: `examples/floom-demo/src/main.rs` (a stub so the crate compiles)

**Interfaces:**
- Consumes: the satellite's directory listing at
  `crates/ecosystem/floom-ui/crates/{material,cupertino,fluent}/`.
- Produces:
  - `examples/floom-demo/Cargo.toml` — package `floom-demo`, `publish = false`,
    `version.workspace = true`, `edition.workspace = true`,
    `license.workspace = true`; dependencies `floom` (features `desktop`,
    `renderer-gpu`, `devtools`), `floom-widgets`, `floom-signal`,
    `floom-core-theme`, and **294** path dependencies to the kit crates;
    dev-dependency `floom-testkit`.
  - `examples/floom-demo/src/ds.rs` — three public modules `material`,
    `cupertino`, `fluent`, each re-exporting one alias per component:
    `pub use floom_widget_md_button::MdButton as Button;` etc. The alias name
    is the component name in CamelCase (`text-field` → `TextField`,
    `progress_linear` → `ProgressLinear`, `radio-group` → `RadioGroup`).
  - `examples/floom-demo/src/props_src.rs` — one `include_str!` constant per
    props module in `floom-core-theme/src/props/`, plus
    `pub fn props_for(module: &str) -> Vec<ApiRow>`.

**Notes for the implementer.** The generator is a Python 3 script (the repo
already uses `python3` in `scripts/check-architecture-deps.sh`); it is run as
`python3 examples/floom-demo/tools/gen.py` from the repo root and is
idempotent. Two rules it must implement:

- **The component set is the intersection.** A directory counts as a kit crate
  when it contains `src/lib.rs`. Emit an alias only for components present in
  all three design systems. `cupertino-icon-paths` and `fluent-icon-paths` have
  no `src/lib.rs` and are excluded; the result is exactly **98** components.
- **The kit type is found by a brace-matched `impl` scan.** For each kit crate's
  `src/lib.rs`, find every `impl <Name> {` block, brace-match its body, and
  select the one whose body contains `pub fn with_theme`. Exactly one must
  match per crate (verified: 294/294). If zero or more than one match, the
  script must exit non-zero with the offending crate named — this is Review
  Focus #3.

The `Cargo.toml` path deps use the kit crate's **directory path** under the
satellite, e.g.
`floom-widget-md-button = { path = "../../crates/ecosystem/floom-ui/crates/material/md-button" }`.
Directory names are inconsistent (`md-text-field` hyphenated, `md-progress_linear`
underscored) — use the directory name verbatim.

`props_src.rs` must include every `floom-core-theme/src/props/*.rs` file except
`mod.rs`, keyed by its module name (`button`, `text_field`, `radio_group`, …).

- [ ] **Step 1: Write the generator and run it**

  Write `examples/floom-demo/tools/gen.py`, then run:

  `python3 examples/floom-demo/tools/gen.py`

  Expected: it writes `Cargo.toml`, `src/ds.rs` and `src/props_src.rs`, and
  prints a summary line: `98 components, 294 kit deps, 88 props modules`.

- [ ] **Step 2: Write the failing test for the generator's output**

  Create `examples/floom-demo/tests/generated.rs`:

  ```rust
  use floom_demo::ds;

  #[test]
  fn the_ds_module_aliases_every_component_in_all_three_design_systems() {
      // One alias per component, per design system: 98 each.
      assert_eq!(ds::material::COMPONENTS.len(), 98);
      assert_eq!(ds::cupertino::COMPONENTS.len(), 98);
      assert_eq!(ds::fluent::COMPONENTS.len(), 98);
  }

  #[test]
  fn the_component_set_is_the_intersection_of_the_three_design_systems() {
      // The two data directories that exist only in Cupertino/Fluent must not
      // appear: they are not crates.
      for names in [
          ds::material::COMPONENTS,
          ds::cupertino::COMPONENTS,
          ds::fluent::COMPONENTS,
      ] {
          assert!(!names.contains(&"IconPaths"));
          assert!(names.contains(&"Button"));
          assert!(names.contains(&"TextField"));
          assert!(names.contains(&"ProgressLinear"));
      }
      // All three design systems expose the same component set.
      let mut material: Vec<&str> = ds::material::COMPONENTS.to_vec();
      material.sort_unstable();
      let mut cupertino: Vec<&str> = ds::cupertino::COMPONENTS.to_vec();
      cupertino.sort_unstable();
      let mut fluent: Vec<&str> = ds::fluent::COMPONENTS.to_vec();
      fluent.sort_unstable();
      assert_eq!(material, cupertino);
      assert_eq!(material, fluent);
  }
  ```

  The `COMPONENTS` constant is a `&'static [&'static str]` the generator also
  emits into `ds.rs`, listing each component's alias name.

- [ ] **Step 3: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test generated`

  Expected: FAIL — the crate has no `lib` target yet and no `ds` module.

- [ ] **Step 4: Add the library target and make it compile**

  - Create `examples/floom-demo/src/lib.rs` exposing `pub mod ds;`,
    `pub mod props_src;` and the `api` module Task 3 will add.
  - Create `examples/floom-demo/src/main.rs` as a stub whose `main` prints
    `floom-demo` and returns.
  - In `Cargo.toml`, set `[lib] name = "floom_demo" path = "src/lib.rs"` and
    `[[bin]] name = "floom-demo" path = "src/main.rs"`.

  Run: `cargo check -p floom-demo`

  Expected: PASS — 294 kit crates compile from the root workspace. This is the
  moment the path-dependency scheme is proven.

- [ ] **Step 5: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test generated`

  Expected: PASS.

- [ ] **Step 6: Verify the all-features CI shape**

  Run: `cargo check --workspace --all-features`

  Expected: PASS — the crate has no features, so the MSRV job's shape is
  unaffected.

- [ ] **Step 7: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): generate the floom-demo crate from the floom-ui satellite"
  ```

---

### Task 3: The gallery model, the categories and the registry

**Files:**
- Create: `examples/floom-demo/src/api.rs`
- Create: `examples/floom-demo/src/pages/mod.rs`
- Create: `examples/floom-demo/src/registry.rs`
- Create (stubs): `examples/floom-demo/src/pages/{actions,forms,data,feedback,layout,motion}.rs`
- Test: `examples/floom-demo/tests/registry.rs`

**Interfaces:**
- Consumes: `crate::api::ApiRow`, `crate::api::ApiDoc` (this task), and
  `floom_core::Child` for `Sample::build`.
- Produces:
  ```rust
  // api.rs
  pub struct ApiRow { pub name: String, pub type_: String, pub default: String, pub description: String }
  impl ApiRow { pub fn new(name: &str, type_: &str, default: &str, description: &str) -> Self }

  pub struct ApiDoc { pub props: Vec<ApiRow>, pub events: Vec<ApiRow>, pub methods: Vec<ApiRow> }
  impl ApiDoc { pub fn empty() -> Self }

  // pages/mod.rs
  #[derive(Clone, Copy, PartialEq, Eq, Debug)]
  pub enum Category { Actions, Forms, DataDisplay, Feedback, Layout, Motion }
  impl Category {
      pub const ALL: [Category; 6];
      pub fn label(self) -> &'static str;   // "Actions", "Forms", "Data display", …
      pub fn slug(self) -> &'static str;    // "actions", "forms", "data-display", …
  }

  pub struct Sample { pub title: &'static str, pub note: Option<&'static str>, pub build: fn() -> Child }
  pub struct ComponentPage {
      pub name: &'static str,
      pub category: Category,
      pub intro: &'static str,
      pub api: ApiDoc,
      pub samples: Vec<Sample>,
  }

  // registry.rs
  pub fn pages() -> Vec<ComponentPage>;   // all pages, grouped by category, in order
  pub fn page_count() -> usize;
  ```

  Each category module exposes `pub fn pages() -> Vec<ComponentPage>`.

**Notes for the implementer.** `Category::ALL` order fixes the order pages
appear in the left rail. The six modules are split by the categories in the
spec §5.9. In this task each module holds only the component **names** and
categories with an empty `samples` vec and `ApiDoc::empty()`; Tasks 6–11 fill
them in. The registry test in this task checks the *shape*; the completeness
check (one page per component, no duplicates) is Task 13's, once the pages
have content.

- [ ] **Step 1: Write the failing test**

  Create `examples/floom-demo/tests/registry.rs`:

  ```rust
  use floom_demo::pages::Category;
  use floom_demo::registry;

  #[test]
  fn the_registry_holds_one_page_per_component() {
      assert_eq!(registry::page_count(), 98);
  }

  #[test]
  fn every_page_has_a_unique_name() {
      let mut names: Vec<&str> = registry::pages().iter().map(|p| p.name).collect();
      names.sort_unstable();
      let total = names.len();
      names.dedup();
      assert_eq!(names.len(), total, "two pages share a name");
  }

  #[test]
  fn pages_are_grouped_by_category_in_category_order() {
      let pages = registry::pages();
      let mut seen: Vec<Category> = Vec::new();
      for page in &pages {
          if seen.last() != Some(&page.category) {
              seen.push(page.category);
          }
      }
      assert_eq!(seen, Category::ALL.to_vec());
  }
  ```

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test registry`

  Expected: FAIL — no `registry` module.

- [ ] **Step 3: Implement the model**

  Write `api.rs`, `pages/mod.rs`, `registry.rs` and the six category modules
  with the signatures above. Each category module lists its components with
  the exact names from spec §5.9, e.g. `actions.rs` lists Button,
  ButtonGroup, CheckboxBtn, ChipGroup, ConfirmEdit, Counter, ExpansionPanels,
  FloatingActionButton, IconButton, ItemGroup, ListGroup, ListSubheader, Radio,
  RadioGroup, Rating, SpeedDial, Stepper, Switch — and each category module
  returns one `ComponentPage` per component with `samples: Vec::new()`.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test registry`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the gallery model, six categories and the 98-page registry"
  ```

---

### Task 4: The shell, the chrome and the entry point

**Files:**
- Create: `examples/floom-demo/src/widgets.rs`
- Create: `examples/floom-demo/src/shell.rs`
- Modify: `examples/floom-demo/src/main.rs`
- Test: `examples/floom-demo/tests/shell.rs`

**Interfaces:**
- Consumes: `crate::registry::pages`, `crate::pages::{Category, ComponentPage}`,
  `floom_core::{Child, ChildKey, LayoutRequest, Widget, WidgetAny}`,
  `floom_widgets::{Button, Divider, Stack, Surface, Text}`.
- Produces:
  ```rust
  // widgets.rs
  pub fn label(key: &'static str, content: impl Into<String>, size: f32, weight: FontWeight, color: Color) -> Child;
  pub fn paragraph(key: &'static str, content: impl Into<String>, size: f32, color: Color) -> Child;
  pub fn section_label(key: &'static str, content: &'static str) -> Child;
  pub fn spacer(key: &'static str, height: f64) -> Child;
  pub fn panel(key: &'static str, body: Stack) -> Child;
  pub fn nav_item(key: &'static str, text: &'static str, active: bool, on_click: impl Fn() + Send + Sync + 'static) -> Child;
  pub fn table(key: &'static str, title: &'static str, rows: &[ApiRow]) -> Stack;

  // shell.rs
  pub struct GalleryShell { /* selection state */ }
  impl GalleryShell {
      pub fn new() -> Self;                                  // all pages, first selected
      pub fn showing(pages: Vec<ComponentPage>, selected: usize) -> Self;
  }
  impl Widget for GalleryShell { /* children(): rail + detail */ }
  impl WidgetAny for GalleryShell { /* as_any, as_any_mut, semantics_hint */ }

  // pages/mod.rs — added in this task
  impl ComponentPage {
      /// A shell showing the catalogue with this page selected.
      pub fn build_shell(&self) -> GalleryShell;
  }
  ```

  `build_shell` is `GalleryShell::showing(crate::registry::pages(), index_of_name(self.name))`
  — the page finds itself in the catalogue by name, so a test can show one
  page without the test knowing its index.

**Notes for the implementer.** Copy the composition idioms from
`examples/floom-showcase/src/widgets.rs` — that file is the proven pattern in
this repo for `Child::new(ChildKey::from(key), LayoutRequest::new(), move ||
Box::new(widget))`, for `Stack::new(Axis::Vertical)`, and for the
`SignalText` pattern that re-declares children when a signal changes. The
shell holds the selected page index in an `Arc<RwSignal<usize>>` (or an
`Arc<Mutex<usize>>`); the detail pane re-declares its children when the
selection changes. The left rail lists every page, grouped by category, with a
section label per category. The detail pane is a `Scroll` holding the selected
page's title, intro, samples and API tables.

The chrome is drawn with engine primitives only, so it looks identical in all
three design systems and never competes with the component being previewed.
`floom-showcase`'s colour constants (`BORDER`, `SIDEBAR`, `ACCENT`, `FAINT`,
`INK`, `MUTED`) are the palette to copy.

- [ ] **Step 1: Write the failing test**

  Create `examples/floom-demo/tests/shell.rs`:

  ```rust
  use floom::floom_types::Size;
  use floom_testkit::TestApp;
  use floom_demo::shell::GalleryShell;

  const WINDOW: Size = Size::new(1280.0, 860.0);

  #[test]
  fn the_shell_mounts_and_paints() {
      let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
      assert!(app.frame().painted > 0);
  }

  #[test]
  fn the_rail_lists_every_page_by_name() {
      let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
      for name in ["Button", "TextField", "Dialog", "ThemeProvider", "OTPInput"] {
          assert!(
              app.find_by_label(name).is_some(),
              "the rail has no entry for {name}"
          );
      }
  }
  ```

  If `floom_testkit::TestApp` exposes no `find_by_label`, use its existing
  selector API (`Selector::label("Button")` plus `app.node(...)`) as
  `examples/floom-showcase/src/tests.rs` does, and assert `app.node(...)` does
  not panic.

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test shell`

  Expected: FAIL — no `shell` module.

- [ ] **Step 3: Implement the chrome, the shell and the entry point**

  Write `widgets.rs`, `shell.rs`, and make `main.rs`:
  - `fn main()` opens a window via
    `floom::run_desktop_with_devtools("Floom 2.0 — Component Gallery", WINDOW, …)`,
    mounting `GalleryShell::new()` with `host.mount_root(LayoutStyle::default(), …)`.
  - `floom-demo --headless` mounts the shell without a window, pumps three
    frames and prints the painted count — a manual diagnostic, not the test
    path.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test shell`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the two-pane gallery shell and its chrome"
  ```

---

### Task 5: The compare row and the first real page

**Files:**
- Create: `examples/floom-demo/src/macros.rs`
- Modify: `examples/floom-demo/src/pages/actions.rs` (the Button page)
- Modify: `examples/floom-demo/src/widgets.rs`
- Test: `examples/floom-demo/tests/compare.rs`

**Interfaces:**
- Consumes: `crate::ds::{material, cupertino, fluent}`,
  `floom_core_theme::theme::MultiTheme`,
  `floom_core_theme::global_theme::DesignSystem`,
  `floom_core::Child`, `floom_widgets::Stack`.
- Produces:
  ```rust
  // macros.rs
  macro_rules! compare_children {
      ($m:ty, $c:ty, $f:ty, |$b:ident, $theme:ident| $body:expr)
          => {{ /* (Child, Child, Child) */ }};
  }
  pub(crate) use compare_children;

  // widgets.rs
  pub fn compare_row(children: (Child, Child, Child)) -> Stack;  // a horizontal Stack of the three
  ```

**Notes for the implementer.** `compare_children!` must instantiate the body
**three times**, each with `$b` bound to that design system's type and
`$theme` bound to `MultiTheme::pinned(<that design system>)`. Each
instantiation returns its own `Child::new(ChildKey::index(n), LayoutRequest::new(),
move || Box::new(<body>))`, so the widget is built *inside* the closure — the
engine widgets are not `Send + Sync` (their `on_click` is a bare
`Box<dyn Fn()>`), so they can never be captured across the closure boundary.
Each closure must therefore capture nothing but `Send + Sync` values, and
`MultiTheme::pinned(...)` is constructed inside the closure for that reason.

The macro's body is written once by the page author and must compile for all
three types, which holds because the three design systems spell their builder
methods identically.

The Button page is the proof:

```rust
fn button_variants() -> Child {
    let (m, c, f) = compare_children!(
        ds::material::Button, ds::cupertino::Button, ds::fluent::Button,
        |b, theme| b.with_theme(theme).label("Click me").view()
    );
    compare_row((m, c, f))
}
```

Note `view(&self) -> Button` for buttons; `resolve(&self)` for `icon` and
`expansion_panels`; `surface(&self)` for `card`; `divider(&self)` for
`divider`; `switch(&self)` for `switch`; and
`view(self, Option<impl Fn(usize) + Send + Sync + 'static>, Option<impl Fn(Vec<usize>) + Send + Sync + 'static>)`
for `item_group`.

- [ ] **Step 1: Write the failing test**

  Create `examples/floom-demo/tests/compare.rs`:

  ```rust
  use floom::floom_types::Size;
  use floom_testkit::TestApp;
  use floom_demo::ds;
  use floom_demo::macros::compare_children;
  use floom_demo::registry;
  use floom_demo::widgets::compare_row;

  const WINDOW: Size = Size::new(1280.0, 860.0);

  #[test]
  fn the_button_page_shows_three_columns() {
      let page = registry::page_named("Button").expect("Button has a page");
      let mut app = TestApp::unpinned(WINDOW, page.build_shell());
      assert!(app.frame().painted > 0);
      for label in ["Material", "Cupertino", "Fluent"] {
          assert!(
              app.find_by_label(label).is_some(),
              "the compare row has no {label} column label"
          );
      }
  }

  #[test]
  fn the_compare_macro_handles_an_argument_taking_terminal() {
      // Review Focus #2: `item_group`'s terminal takes two callbacks.
      let (m, c, f) = compare_children!(
          ds::material::ItemGroup,
          ds::cupertino::ItemGroup,
          ds::fluent::ItemGroup,
          |b, theme| b.with_theme(theme).view(None, None)
      );
      let row = compare_row((m, c, f));
      let mut app = TestApp::unpinned(WINDOW, row);
      assert!(app.frame().painted > 0);
  }
  ```

  `registry::page_named(&str) -> Option<ComponentPage>` is added in this task
  alongside `registry::pages()`.

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test compare`

  Expected: FAIL — no `compare_children!`, no Button samples.

- [ ] **Step 3: Implement the macro, the compare row and the Button page**

  Write `macros.rs`, add `compare_row` to `widgets.rs`, and give the Button
  page one sample (`button_variants`) plus an `ApiDoc` with an empty props
  list (Task 12 fills props in) and one hand-written method row.

  `compare_row` is a horizontal `Stack` of the three children with a gap and
  a per-column caption. The captions are the strings `"Material"`,
  `"Cupertino"`, `"Fluent"` — the same three names the registry test uses.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test compare`

  Expected: PASS.

- [ ] **Step 5: Run the whole demo test suite**

  Run: `cargo test -p floom-demo`

  Expected: PASS — every test so far.

- [ ] **Step 6: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the three-design-system compare row, with the Button page"
  ```

---

### Task 6: The Actions category (18 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/actions.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:**
- Consumes: `compare_children!`, `compare_row`, `crate::api::ApiDoc`,
  `crate::pages::Sample`.
- Produces: every Actions page gains real samples and API tables.

**Components:** Button, ButtonGroup, CheckboxBtn, ChipGroup, ConfirmEdit,
Counter, ExpansionPanels, FloatingActionButton, IconButton, ItemGroup, ListGroup,
ListSubheader, Radio, RadioGroup, Rating, SpeedDial, Stepper, Switch.

**Notes for the implementer.** Each component gets **at least two samples**:
one showing its default or common configuration, and one showing a variation
(disabled state, a second variant, an empty state). Each sample is one
`compare_children!` call plus `compare_row`. Interactive samples that need
state (a selected index, a toggle) hold the state in an
`Arc<RwSignal<…>>` created inside the sample's builder closure and pass
closures that update it.

`ItemGroup` is Review Focus #2 — its terminal takes two `Option<impl Fn…>`
arguments; pass `None, None`.

`ExpansionPanels` and `Icon` use `resolve()` instead of `view()`.

Components with no visual of their own state that in the sample's `note`
rather than showing an empty column.

- [ ] **Step 1: Write the failing test**

  In `examples/floom-demo/tests/categories.rs`:

  ```rust
  use floom_demo::registry;

  #[test]
  fn every_actions_component_has_at_least_two_samples() {
      for name in ACTIONS {
          let page = registry::page_named(name).expect("page exists");
          assert!(
              page.samples.len() >= 2,
              "{name} has {} samples",
              page.samples.len()
          );
      }
  }
  ```

  where `ACTIONS` is the list of the 18 names.

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL — the pages have no samples.

- [ ] **Step 3: Write the samples and the API tables**

  For each of the 18 components, write its samples (each a `compare_children!`
  call) and an `ApiDoc` with hand-written **events** and **methods** rows.
  Props rows are filled by Task 12 — leave `props` empty here.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Actions category — 18 components"
  ```

---

### Task 7: The Forms category (16 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/forms.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:** as Task 6.

**Components:** Autocomplete, Checkbox, ColorPicker, Combobox, DatePicker,
FileInput, Form, NumberInput, OTPInput, RangeSlider, Select, Slider, TextField,
Textarea, TimePicker, ValidationMessages.

**Notes for the implementer.** Several of these take children or a value:
`Form` and `ValidationMessages` wrap content; `Combobox`/`Autocomplete`/`Select`
take an items list; `FileInput` reports a path through `path_signal()` rather
than opening an OS dialog (say so in the note). `OTPInput` shows the masked
variant as its second sample. `RangeSlider` shows a two-thumb range.

- [ ] **Step 1: Write the failing test**

  Add to `examples/floom-demo/tests/categories.rs` a test
  `every_forms_component_has_at_least_two_samples` over the 16 names.

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL.

- [ ] **Step 3: Write the samples and the API tables**

  As Task 6 Step 3.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Forms category — 16 components"
  ```

---

### Task 8: The Data display category (21 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/data.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:** as Task 6.

**Components:** Avatar, Badge, Calendar, Card, Carousel, Chip, Code, DataIterator,
DataTable, Icon, Image, Kbd, List, ListItem, Pagination, Sparkline, Table, Text,
Timeline, Treeview, VirtualScroller.

**Notes for the implementer.** `Card` uses `surface()` (not `view()`), and
`md-card` has no `.child()` builder — attach content through the `Surface`
itself. `Icon` uses `resolve()`. `List` returns a `Stack`, so its items are
children of that stack rather than a builder argument — build the item list
inside the sample's closure.

- [ ] **Step 1: Write the failing test**

  Add to `examples/floom-demo/tests/categories.rs` a test
  `every_data_display_component_has_at_least_two_samples` over the 21 names.

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL.

- [ ] **Step 3: Write the samples and the API tables**

  As Task 6 Step 3.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Data display category — 21 components"
  ```

---

### Task 9: The Feedback category (9 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/feedback.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:** as Task 6.

**Components:** Alert, Banner, Dialog, EmptyState, Menu, Overlay, Sheet, Snackbar,
Tooltip.

**Notes for the implementer.** These are the overlay components, and an
unrevealed overlay paints nothing — which would make a headless "it painted"
assert vacuous. Each overlay sample must therefore be **revealed** in its
sample builder (`Dialog::open(true)`, the snackbar's open flag, the menu's open
flag, the tooltip's visible flag, the sheet's open flag). The overlay's
`open_flag()`-style accessor is the way in; where the kit exposes only a
`RwSignal`, set it directly. `Overlay` itself is the base layer — show it with
a child so it has something to draw.

- [ ] **Step 1: Write the failing test**

  Add to `examples/floom-demo/tests/categories.rs` a test
  `every_feedback_component_has_at_least_two_samples` over the 9 names, and a
  second test asserting each overlay page's note is not `None` (an overlay that
  says nothing about its reveal state is the vacuous-assert case).

- [ ] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL.

- [ ] **Step 3: Write the samples and the API tables**

  As Task 6 Step 3, plus the reveal rule above.

- [ ] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Feedback category — 9 components"
  ```

---

### Task 10: The Navigation & layout category (19 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/layout.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:** as Task 6.

**Components:** App, AppBar, BottomNavigation, Breadcrumbs, Container, Divider,
Footer, Grid, Hover, Main, NavigationDrawer, Parallax, Responsive, SlideGroup,
Spacer, SystemBar, Tabs, Toolbar, Window.

**Notes for the implementer.** `Divider` uses `divider()` as its terminal.
`App`, `Main` and `Window` are application shells — their samples show them
wrapping a small amount of content, and their notes say they are shells rather
than leaf controls. `Responsive` and `Hover` depend on pointer/scroll state;
their samples show the resting state and say in the note what triggers the
other.

- [x] **Step 1: Write the failing test**

  Add to `examples/floom-demo/tests/categories.rs` a test
  `every_layout_component_has_at_least_two_samples` over the 19 names.

- [x] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL.

- [x] **Step 3: Write the samples and the API tables**

  As Task 6 Step 3.

- [x] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [x] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Navigation and layout category — 19 components"
  ```

---

### Task 11: The Motion, progress & providers category (15 components)

**Files:**
- Modify: `examples/floom-demo/src/pages/motion.rs`
- Test: `examples/floom-demo/tests/categories.rs`

**Interfaces:** as Task 6.

**Components:** DateAdapterProvider, DefaultsProvider, DisplayProvider,
ExpandTransition, FadeTransition, IconsProvider, InfiniteScroller,
LocaleProvider, ProgressCircular, ProgressLinear, PullToRefresh,
ScaleTransition, SkeletonLoader, SlideTransition, ThemeProvider.

**Notes for the implementer.** Review Focus #4 applies here: the six
providers (`ThemeProvider`, `LocaleProvider`, `DisplayProvider`,
`DefaultsProvider`, `IconsProvider`, `DateAdapterProvider`) are pure service
layers with **no visual output of their own**. Their pages must state that in
the sample's `note` — "provides X to its descendants; no visual of its own" —
and show their API table plus a read-out of their signal's current value, so
the page is not an empty column.

The four transitions need their `show` flag set so the transition is
observable; their samples wrap a child and set the flag true. `InfiniteScroller`
and `PullToRefresh` depend on scroll and pointer state; show the resting
state and say what triggers the other.

- [x] **Step 1: Write the failing test**

  Add to `examples/floom-demo/tests/categories.rs` a test
  `every_motion_component_has_at_least_two_samples` over the 15 names, and a
  test asserting each provider page's samples all carry a `note` (Review
  Focus #4).

- [x] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test categories`

  Expected: FAIL.

- [x] **Step 3: Write the samples and the API tables**

  As Task 6 Step 3, plus the provider and transition rules above.

- [x] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test categories`

  Expected: PASS.

- [x] **Step 5: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): the Motion, progress and providers category — 15 components"
  ```

---

### Task 12: The props tables, derived from the library

**Files:**
- Modify: `examples/floom-demo/src/props_src.rs` (the parser half; the
  `include_str!` half is generated)
- Modify: `examples/floom-demo/src/pages/*.rs` (each page's `api.props`)
- Test: `examples/floom-demo/tests/props.rs`

**Interfaces:**
- Consumes: `crate::api::ApiRow`, the generated `PROPS_SOURCES` table.
- Produces:
  ```rust
  // props_src.rs
  pub fn props_for(module: &str) -> Vec<ApiRow>;
  pub fn field_count(module: &str) -> usize;
  pub const COMPONENT_PROPS: &[(&str, &str)];   // (component name, props module)
  ```
  where `module` is a props-module name such as `"button"`, `"text_field"`,
  `"radio_group"`. Returns one row per `pub` field of the module's `…Props`
  struct, with the field's type as the `type_` column and the matching
  `Default` initialiser as the `default` column. The `description` column is
  empty for now — Task 12's second half fills it.

  `field_count` re-counts the `pub` fields by a path independent of the
  row-building path, so a parser that silently drops a field is caught.
  `COMPONENT_PROPS` pairs each component with its props module and is
  generated with the rest of `props_src.rs`.

**Notes for the implementer.** The parser reads the props source text, finds
`pub struct <Name>Props {` and its `pub` fields, then finds the `impl Default
for <Name>Props` block and reads each field's initialiser. Types are rendered
as written in the source, minus the `pub ` prefix. A field with no matching
default (there are none today) gets `—`.

Then, for every page, set `api.props` to `props_for("<module>")` where
`<module>` is that component's props module (`button` → `"button"`,
`TextField` → `"text_field"`, `RadioGroup` → `"radio_group"`, `FloatingActionButton`
→ `"floating_action_button"`). The mapping is the component name in snake
case; where it differs, the props module list is authoritative — every
component has a props module with the same name as its props file.

Finally, fill in the `description` column for each row. The description for a
row comes from a hand-written side table in the same page module; a prop with
no side-table entry keeps an empty description, and the table renders an empty
cell as `—`.

- [x] **Step 1: Write the failing test**

  Create `examples/floom-demo/tests/props.rs`:

  ```rust
  use floom_demo::props_src;

  #[test]
  fn the_button_props_table_matches_the_librarys_button_props() {
      let rows = props_src::props_for("button");
      let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
      assert_eq!(
          names,
          vec!["label", "size", "variant", "disabled", "loading", "on_click"]
      );
      let size = rows.iter().find(|r| r.name == "size").unwrap();
      assert_eq!(size.type_, "ButtonSize");
      assert_eq!(size.default, "ButtonSize::Medium");
  }

  #[test]
  fn an_unknown_props_module_yields_no_rows() {
      assert!(props_src::props_for("not_a_component").is_empty());
  }
  ```

- [x] **Step 2: Run the test to verify it fails**

  Run: `cargo test -p floom-demo --test props`

  Expected: FAIL — `props_src::props_for` does not exist.

- [x] **Step 3: Implement the parser and wire it into the pages**

  Write the parser in `props_src.rs` and set every page's `api.props`.

- [x] **Step 4: Run the test to verify it passes**

  Run: `cargo test -p floom-demo --test props`

  Expected: PASS.

- [x] **Step 5: Verify the tables are complete**

  Add to `examples/floom-demo/tests/props.rs`:

  ```rust
  #[test]
  fn every_component_has_a_props_table_covering_its_props_struct() {
      for (component, module) in floom_demo::props_src::COMPONENT_PROPS {
          let rows = floom_demo::props_src::props_for(module);
          let expected = floom_demo::props_src::field_count(module);
          assert_eq!(
              rows.len(),
              expected,
              "{component}'s table has {} rows, its props struct has {expected}",
              rows.len()
          );
      }
  }
  ```

  `COMPONENT_PROPS` is the generated `(&str, &str)` list pairing each
  component with its props module, and `field_count` re-counts the `pub`
  fields independently of the row-building path, so a parser that silently
  drops a field fails the test. A component whose table is missing a prop is
  a drift bug.

- [x] **Step 6: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "feat(demo): derive the props tables from floom-core-theme's own sources"
  ```

---

### Task 13: The test suite

**Files:**
- Create: `examples/floom-demo/tests/pages.rs`
- Create: `examples/floom-demo/tests/interaction.rs`

**Interfaces:**
- Consumes: `crate::registry::{pages, page_named}`, `crate::shell::GalleryShell`,
  `floom_testkit::TestApp`.
- Produces: the headless gate for the gallery.

**Notes for the implementer.** `tests/pages.rs` mounts **every one of the 98
pages** in turn and asserts it paints:

```rust
for page in floom_demo::registry::pages() {
    let mut app = TestApp::unpinned(WINDOW, page.build_shell());
    assert!(app.frame().painted > 0, "{} painted nothing", page.name);
}
```

`ComponentPage::build_shell() -> GalleryShell` was added in Task 4; it
builds a shell showing that one page, so the test does not have to drive the
selection. Its `Samples` must all be rendered, so a page whose samples are all
overlays shows them revealed.

`tests/interaction.rs` proves the interactive samples move: for the Button
page, tap the button and assert a counter increments; for the Checkbox page,
tap and assert the checked flag changed. Use `floom_testkit::TestApp`'s
`app.tap(app.node(Selector::label("…")))` as
`examples/floom-showcase/src/tests.rs` does.

- [x] **Step 1: Write the failing test**

  Create `examples/floom-demo/tests/pages.rs` as above and
  `examples/floom-demo/tests/interaction.rs` with the two interaction tests.

- [x] **Step 2: Run the tests to verify they fail**

  Run: `cargo test -p floom-demo --test pages --test interaction`

  Expected: FAIL — the pages currently have no samples to render, so the
  per-page loop paints nothing.

- [x] **Step 3: Make the interaction tests pass**

  Adjust the interactive samples so a tap is observable: the Button sample
  increments a counter held in an `Arc<AtomicUsize>` the test can read, and
  the Checkbox sample toggles a flag the test can read.

- [x] **Step 4: Run the tests to verify they pass**

  Run: `cargo test -p floom-demo`

  Expected: PASS — every test, including the 98-page loop.

- [x] **Step 5: Run the three design systems' builds**

  Run:
  `cargo test -p floom-demo --no-default-features` and
  `cargo check -p floom-demo --all-targets`

  Expected: PASS (the crate has no features, so this is the same build).

- [x] **Step 6: Commit**

  ```bash
  git add examples/floom-demo
  git commit -m "test(demo): mount every gallery page headlessly and prove the interactions"
  ```

---

### Task 14: The satellite justfile, the README and the docs

**Files:**
- Modify: `crates/ecosystem/floom-ui/justfile`
- Create: `examples/floom-demo/README.md`
- Modify: `crates/ecosystem/floom-ui/docs/PLAN.md`
- Modify: `crates/ecosystem/floom-ui/docs/COMPONENT_PARITY.md`
- Modify: `crates/ecosystem/floom-ui/docs/demo-gallery-plan.md`

**Interfaces:**
- Consumes: the finished crate.
- Produces: the runnable entry points and the documentation that matches
  reality.

**Notes for the implementer.** The satellite `justfile`'s three targets point
at `cargo … -p floom-demo` **from the satellite directory**, where that
package cannot exist. Replace them with one target that runs the demo from the
root workspace:

```make
demo:
    cargo run --manifest-path ../../../../Cargo.toml -p floom-demo
```

and remove the stale broken-target note added during the P0 doc sweep. The
`material` / `cupertino` / `fluent` targets go away: all three design systems
are now in one binary, shown side by side.

`PLAN.md` — mark the P1 gallery task done and point at the new crate.
`COMPONENT_PARITY.md` — replace the note saying no runnable gallery exists
with the new one. `demo-gallery-plan.md` — replace the banner saying the plan
is unimplemented with what was actually built and where it diverged from the
plan (one binary with all three design systems side by side, rather than one
build per design system).

- [ ] **Step 1: Repair the satellite justfile**

  Edit `crates/ecosystem/floom-ui/justfile` as described.

  Run: `cargo run --manifest-path ../../../../Cargo.toml -p floom-demo -- --headless`
  from `crates/ecosystem/floom-ui`.

  Expected: prints the painted count and exits.

- [ ] **Step 2: Write the README**

  `examples/floom-demo/README.md`: what the gallery is, how to run it, what
  the three columns mean, and how to add a component (the three-step recipe:
  regenerate the deps, write the page, register it).

- [ ] **Step 3: Update the three docs**

  As described in the notes.

- [ ] **Step 4: Run the whole gate**

  Run: `just gate`

  Expected: PASS — fmt, clippy, tests, ecosystem, docs, architecture, probe,
  plugins, goldens. The demo is covered by the `test` check.

- [ ] **Step 5: Commit**

  ```bash
  git add examples/floom-demo crates/ecosystem/floom-ui
  git commit -m "docs(demo): the gallery README, the repaired satellite justfile and the doc updates"
  ```

---

## Self-Review

**Spec coverage.** §5.1 file structure → Tasks 2–5. §5.2 three design systems
and the pinned theme → Tasks 1 and 5. §5.3 the `ds` module and the compare
macro → Tasks 2 and 5. §5.4 the gallery model → Task 3. §5.5 the shell → Task 4.
§5.6 derived props → Task 12. §5.7 entry points and the headless tests →
Tasks 4 and 13. §5.8 dependencies → Task 2. §5.9 the 98-component catalogue →
Tasks 3 and 6–11. §5.10 verification → Tasks 5, 12, 13, 14. §5.11 the
satellite justfile → Task 14. Goals G1–G8 each have a task. Non-goals N1–N4
and anti-goals A1–A4 are enforced by the Global Constraints.

**Step scan.** Every step names a file, a signature or a test name, and the
command that proves it. The category tasks (6–11) are deliberately parallel:
the shape of a page is fixed by Task 5 and Task 3, so those tasks state the
component list and the test rather than repeating the page template.

**Type consistency.** `ApiRow`/`ApiDoc` (Task 3) are used by `props_src`
(Task 12) and by `widgets::table` (Task 4). `ComponentPage`/`Sample`/
`Category` (Task 3) are produced by the category modules (Tasks 6–11) and
consumed by `registry` (Task 3) and `ComponentPage::build_shell` (Task 13).
`compare_children!`/`compare_row` (Task 5) are used by every category task.
`registry::page_named` is introduced in Task 5 and used in Tasks 6–11 and 13.

**Review Focus.** #1 and #2 → Task 5's `compare.rs` (all three arms compile;
`item_group` proves the argument-taking terminal). #3 → Task 2's generator
exits non-zero on ambiguity, and `generated.rs` pins the component set. #4 →
Task 11's provider-note assertion. #5 → Task 2's `generated.rs` excludes
`IconPaths`.

**Proportion.** The plan states interfaces and assertions, not implementations.
The only code blocks are the macro's call shape, the parser's contract, and
the tests that pin them — each of which a reader could otherwise get wrong in
a way that only shows up at compile time.
