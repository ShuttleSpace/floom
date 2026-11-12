# floom-demo — a runnable component gallery for `floom-ui`

**Date:** 2026-11-12
**Status:** approved (2026-11-12) — scope decisions below
**Scope:** `crates/ecosystem/floom-ui` (the satellite) + a new root example crate

---

## 1. Intent and the why

`floom-ui` ships **306 workspace members** — 10 consolidated crates plus 296
per-design-system kit crates (98 material + 99 cupertino + 99 fluent; the
extra one in each of the latter two is `icon-paths`, a data directory, not a
crate). Every component has a real, renderable `view()`, but **there is no way
to look at them**. The v1 `floom-demo` crate that used to render a gallery
window was removed along with the `floom-elements` facade, and nothing replaced
it. The three `justfile` targets that ran it (`material` / `cupertino` /
`fluent`) are broken today — they point at a crate that does not exist.

The result is a library whose entire value is visual, with no visual surface.
A developer cannot answer "what does `md-slider` actually look like", "how
does Cupertino's button differ from Material's", or "what props does
`MdTextField` take" without writing a throwaway program per question.

**This design builds that surface:** a runnable gallery application that mounts
the real kit widgets for **all 98 components**, shows each component's
props/events/methods, and **places the three design systems side by side** so
their differences are directly visible.

The design record for this decision lives outside the repo at
`floom-wip/2026-11-11-04-floom-demo-gallery/MANIFEST.md`; this document is the
specification.

---

## 2. Goals, non-goals, anti-goals

### Goals

- **G1 — A runnable gallery.** `cargo run -p floom-demo` opens a window listing
  every component in the library, grouped by category, with live samples.
- **G2 — Real widgets, not pictures.** Every sample mounts the actual kit
  widget (`MdButton`, `CupertinoButton`, `FluentButton`, …) so what you see is
  what the library renders. No screenshots, no re-implementations.
- **G3 — All 98 components.** Every component that exists in all three design
  systems gets a page. Not a representative subset.
- **G4 — Side-by-side design systems.** Each sample shows the component three
  times — Material, Cupertino, Fluent — **each rendered with its own design
  system's real tokens**, in one window, in one binary. This is the gallery's
  most distinctive capability and the reason it exists.
- **G5 — Documented API.** Each component page carries a Props / Events /
  Methods table, so the gallery doubles as the reference a docs site will
  later be generated from.
- **G6 — Interactive.** Samples respond to input; exposed methods
  (e.g. `Dialog::open()`) are callable from the page, as the v1 design called
  for.
- **G7 — Headless verification.** The gallery is covered by the root
  `cargo test --workspace` gate: every page is mounted and asserted to paint.
- **G8 — Mechanical extension.** Adding a component to the gallery is one
  module + one registry entry, not a change to the shell.

### Non-goals

- **N1 — Not a runtime *whole-app* design-system switcher.** The library has
  one active design system per process (§3, C12). The gallery's compare view
  works around that with pinned themes rather than changing the rule (§5.2).
- **N2 — Not pixel goldens.** Golden-image comparison is `floom-accuracy` /
  `floom-snapshot` territory. This crate asserts *behaviour* (mounts, paints,
  responds to a tap), not pixels.
- **N3 — Not a docs site, theme builder, or Figma importer.** Those are P3 in
  `crates/ecosystem/floom-ui/docs/PLAN.md`, out of scope here.
- **N4 — Not inside the satellite workspace.** The satellite stays headless
  (see §4, approach C).

### Anti-goals

- **A1 — Do not pull the whole `floom-ui` workspace into the build.** Path-dep
  only the 294 kit crates the gallery shows. The satellite's other crates
  (`floom-ui-composites`, `floom-harmony`, the 10 consolidated crates) are not
  gallery content.
- **A2 — Do not re-implement components in the demo.** A gallery that renders
  its own lookalike button is worthless as a preview.
- **A3 — Do not change the satellite's behaviour.** The gallery is a *consumer*.
  The one exception is the additive `MultiTheme::pinned` in §5.2, which adds
  capability without changing any existing behaviour, and which needs explicit
  approval. Nothing else in the satellite may be modified.
- **A4 — Do not hand-maintain a second copy of the props metadata.** The
  props tables are derived from the source of truth (§5.6), not retyped.

---

## 3. Constraints

Properties of the tree as it stands today, each verified before writing this
document.

| # | Constraint | Evidence |
|---|---|---|
| C1 | The satellite is a separate workspace with 306 members; root `cargo test --workspace` does not reach it. | `crates/ecosystem/floom-ui/Cargo.toml` opens with `[workspace]`; root `Cargo.toml` `exclude`s `crates/ecosystem` |
| C2 | `examples/floom-*` is a workspace member glob, so a new example joins the root workspace with no manifest edit. | root `Cargo.toml` `members = [… "examples/floom-*"]` |
| C3 | Examples are in `members` but **not** `default-members`, so `cargo test --workspace` (the gate's `test` check) builds and tests them. | root `Cargo.toml`; `scripts/gate.sh:401` |
| C4 | The root clippy check covers `FLOOM2_PACKAGES` only, which contains **no examples**. | `scripts/gate.sh:392` |
| C5 | CI runs `cargo check --workspace --all-features` (MSRV job). Any feature design must survive all features on at once. | `.github/workflows/ci-core.yml:346` |
| C6 | The 3 DS kit crates have **identical builder method names and signatures**; only the type names differ. | diff of `md-button` vs `cupertino-button`: 132 diff lines, all internal |
| C7 | 94 of 99 components have an **identical entry-point API across all three DS** (same method name, same return type). Two real outliers: `card` (`md-card` has no `view()`, but all three have `surface() -> Surface`) and `switch` (all three use `switch() -> Switch`, not `view()`). | per-crate survey of `pub fn (…) -> EngineType` across `material/`, `cupertino/`, `fluent/` |
| C8 | **Material kits read the global theme signal** (`theme_signal()`), so they are runtime-re-tintable. **Cupertino/Fluent kits bake `MultiTheme::new_for(DS)` at construction.** | `md-button/src/lib.rs:50` vs `cupertino-button/src/lib.rs:50` |
| C9 | Dark mode *is* runtime-switchable for all three DS: `Theme::colors()` reads the global `is_dark()` live. | `floom-core-theme/src/theme.rs:199,310,421` |
| C10 | Headless UI testing exists: `floom_testkit::TestApp` mounts a widget tree without a window and can find/tap nodes. | `crates/devtools/floom-testkit/src/lib.rs:132,348,512` |
| C11 | The satellite's `justfile` runs `cargo … -p floom-demo` **from the satellite directory**, where that package does not and will not exist. | `crates/ecosystem/floom-ui/justfile` |
| **C12** | **The library has exactly one active design system per process, by design and by test.** `MultiTheme::new_for(ds)` **ignores its argument** — `activate()` is a documented no-op and `ds()` returns the process-global `current_design_system()`. A test asserts this explicitly. | `floom-core-theme/src/theme.rs:499-521`; `floom-core-theme/tests/design_system_themes.rs:37-62` (`multi_theme_reads_follow_the_process_global_not_its_own_argument`) |
| **C13** | **294 of 296 kit crates expose `with_theme(MultiTheme)`** — the hook a compare view needs. (The two without it are `cupertino-icon-paths` / `fluent-icon-paths`, which are data directories, not crates.) | survey of `pub fn with_theme` across all kit crates |
| **C14** | **Engine widgets bake their resolved style at construction.** `floom_widgets::Button` holds concrete `base`, `text_color`, `radius`, `hover`, `press` — not a theme signal. So a widget built under a pinned theme keeps that theme's values. | `floom-widgets/src/controls.rs:329-360`; `md-button/src/lib.rs:144-158` |
| C15 | Exactly **98 components exist in all three design systems** (the intersection). Kit directory names are inconsistent — some hyphenated (`md-text-field`, `md-radio-group`), some underscored (`md-progress_linear`, `md-icon_button`). | set intersection of the three `crates/*/` directories; `docs/COMPONENT_PARITY.md` contract table |
| C16 | `floom-core-theme` exposes the props structs as plain `pub` fields with a `Default` impl giving each default — machine-readable without a parser framework. No `syn`/`quote` in the workspace. | `floom-core-theme/src/props/*.rs` |

---

## 4. Approaches considered

### Approach A — Grow `examples/floom-showcase`'s `/kits` route

The root showcase already mounts `md-text`, `md-button` and `md-checkbox`, and
already has a shell, a router and devtools wiring.

- **For:** no new crate; reuses the shell and the F12 inspector.
- **Against:** `floom-showcase` is explicitly *"one application over the
  engine, signals, the ported kits, and the editor"* — a tour of the stack.
  Growing it into a 98-component browser turns the tour into the product. With
  all three design systems now compiled in, it would also multiply the
  showcase's own build cost. (Superseded: the showcase predates the 3-DS
  compare requirement and cannot host it without becoming the gallery.)

### Approach B — A new dedicated crate at `examples/floom-demo` ✅ **chosen**

- **For:** focused scope; proven precedent (`floom-showcase` already path-deps
  into the satellite from the root workspace); automatic root membership via
  the `examples/floom-*` glob (C2); automatic headless coverage via
  `cargo test --workspace` (C3); the satellite stays headless and its CI cost
  unchanged; the 294 kit deps are scoped to one crate instead of polluting the
  showcase.
- **Against:** one more crate in the root workspace; the shell chrome (sidebar,
  props table) is new code rather than reused.
- **Why this one:** the gallery is a *product surface*, not a demo of the
  stack. It deserves its own crate whose dependency list can stay narrow
  (anti-goal A1), and the root workspace already knows how to host such a
  crate.

### Approach C — Put the gallery inside the satellite (`crates/ecosystem/floom-ui/examples/floom-demo`)

- **For:** lives next to the library it documents; `just material|cupertino|fluent`
  in the satellite's `justfile` would work unchanged.
- **Against:** the satellite is deliberately **all-headless** — 306 members
  tested with `cargo test --workspace` in CI (`ecosystem-check`). A gallery
  needs the root `floom` desktop stack (winit + GPU), which would pull a
  windowing and GPU toolchain into a workspace whose entire value proposition
  is *"it builds anywhere, headlessly, fast"*. This directly violates N4.

### Decision

**Approach B.** Recorded per the brainstorming skill; the trade-off summary is:
A is cheaper but conflates two products, C is better placed but poisons the
satellite's headless guarantee, B costs one crate and buys focus.

---

## 5. The design

### 5.1 Shape

```
examples/floom-demo/
├── Cargo.toml          # package "floom-demo", publish = false, 294 path deps
├── README.md           # how to run, what the compare view shows
└── src/
    ├── main.rs         # entry point: window mode | --headless
    ├── ds.rs           # per-design-system type aliases (all three, always)
    ├── registry.rs     # the catalogue: 6 categories → 98 component pages
    ├── shell.rs        # two-pane shell: nav rail + scrolling detail pane
    ├── widgets.rs      # gallery chrome: section, compare row, props table
    ├── api.rs          # Props / Events / Methods metadata types
    ├── props_src.rs    # derives the props tables from floom-core-theme's source
    └── pages/
        ├── mod.rs      # page modules + the category list
        ├── actions.rs  # 18 components
        ├── forms.rs    # 16
        ├── data.rs     # 21
        ├── feedback.rs # 9
        ├── layout.rs   # 19
        └── motion.rs   # 15
```

### 5.2 Three design systems in one binary

**The problem (C12).** The library has one active design system per process.
`MultiTheme::new_for(DesignSystem::Cupertino)` does *not* produce a
Cupertino-themed theme — it produces a theme that reads whatever the process
global says. The Cupertino kits call exactly that constructor, so a Cupertino
button renders Material's palette whenever the global is Material. This is
asserted by a test whose name says it outright:
`multi_theme_reads_follow_the_process_global_not_its_own_argument`.

So the naïve "compile three design systems and show them side by side" does
not work: all three columns would render the *same* palette, and the compare
view would show three identical pictures with different code paths.

**The hook (C13, C14).** Every kit exposes `with_theme(MultiTheme)`, and engine
widgets bake their resolved style at construction. So a component can be
*given* a specific design system's theme and keep it.

**The satellite change (additive; needs approval).** `floom-core-theme` gains
a pinned-design-system variant of `MultiTheme`:

```rust
pub struct MultiTheme {
    /// When set, this theme reads this design system's tokens regardless of
    /// the process-global one. `None` follows the global, as before.
    pinned: Option<DesignSystem>,
    material: MaterialTheme,
    cupertino: CupertinoTheme,
    fluent: FluentTheme,
}

impl MultiTheme {
    /// Build a theme that follows the process-global design system.
    pub fn new_for(ds: DesignSystem) -> Self { /* unchanged: ds ignored */ }

    /// Build a theme pinned to one design system, ignoring the global.
    pub fn pinned(ds: DesignSystem) -> Self {
        Self { pinned: Some(ds), material: MaterialTheme::new(),
               cupertino: CupertinoTheme::new(), fluent: FluentTheme::new() }
    }

    fn ds(&self) -> DesignSystem {
        self.pinned.unwrap_or_else(current_design_system)
    }
}
```

Roughly ten lines, in one file, **purely additive**:

- `new_for` keeps its current (global-following) behaviour, so the 484 kit
  crates that call it are unaffected and the existing test still passes.
- `activate()` stays the no-op it is documented to be.
- `pinned` is new API that nothing in the satellite calls.
- Dark mode is untouched: `is_dark()` is consulted live by the concrete
  themes, so a pinned theme still honours the global dark flag (C9).

**Why this and not the alternatives.**

- *Change `new_for` to honour its argument* — that inverts the library's
  documented, tested rule and would break the "switch the whole app's design
  system at runtime" behaviour for the 484 kits that bake their own theme. It
  is a redesign of the library's theme model, not a gallery change.
- *Temporarily set the process global while resolving each variant* — the
  engine widgets are rebuilt inside `Child` closures, so the global would have
  to be re-pinned on every rebuild. Writing a signal during a build risks a
  rebuild loop, and it is invisible to a reader. Rejected as fragile.
- *Compare by token tables only* — possible without touching the satellite
  (`MaterialTheme`/`CupertinoTheme`/`FluentTheme` are concrete and per-DS),
  but it shows numbers, not components. Offered in §6 as the fallback if the
  satellite change is refused.

**No Cargo features for the design system.** All three design systems are
compiled into the one binary, always. This is what makes the compare view
possible, and it is also why C5 (`--all-features`) becomes moot for this
crate: there are no features to collide. The root gate's added compile cost
(294 kit crates) has been reviewed and accepted.

### 5.3 The `ds` module — one gallery source, three design systems

Because the builder APIs are identical across the three design systems (C6)
and 94/99 components have the same entry point in each (C7), the gallery code
is written **once** against three per-design-system namespaces:

```rust
// ds.rs — all three are always compiled
pub mod material {
    pub use floom_widget_md_button::MdButton as Button;
    pub use floom_widget_md_text_field::MdTextField as TextField;
    // … one alias per component
}
pub mod cupertino { /* … */ }
pub mod fluent { /* … */ }
```

A sample declares the component **once** and the gallery renders it three
times, each under its own pinned theme:

```rust
// The sample says "a Button"; the gallery makes it three.
ds::material::Button::new().label("Click me")
ds::cupertino::Button::new().label("Click me")
ds::fluent::Button::new().label("Click me")
```

The per-design-system *type* differs, so a sample cannot literally be one
expression reused three times. It is instead **one builder closure per design
system**, which is why the gallery uses a small macro:

```rust
/// Build the same component for all three design systems, each pinned to its
/// own theme, and return the three engine widgets side by side.
macro_rules! compare_row {
    ($material:expr, $cupertino:expr, $fluent:expr) => { … };
}
```

Each design system's arm differs only in the type path and the pinned theme,
so the *body* of the sample — the props, the variants, the interaction — is
written once per component and repeated per design system. **This is the
mechanism that makes G3 and G8 hold at 98 components.**

The entry terminal **differs per component but is uniform across design
systems** — most components expose `view()`, `card` exposes `surface()`, and
`switch` exposes `switch()` (all three return the matching engine widget:
`Button`, `TextField`, `Stack`, `Slider`, `Select`, `Surface`, `Switch`, …).
The sample calls whichever its component uses; this is the only place the
per-component terminal leaks in.

### 5.4 The gallery model

```rust
pub struct ComponentPage {
    pub name: &'static str,          // "Button"
    pub category: Category,           // Actions / Forms / Data display / …
    pub intro: &'static str,          // one line, shown under the title
    pub api: ApiDoc,                 // Props / Events / Methods tables
    pub samples: Vec<Sample>,         // live, interactive examples
}

pub struct Sample {
    pub title: &'static str,         // "Variants"
    pub note: Option<&'static str>,   // inline caveat, e.g. DS differences
    pub build: fn() -> Child,         // the live widget tree (3 DS columns)
}

pub struct ApiDoc { pub props: Vec<ApiRow>, pub events: Vec<ApiRow>,
                    pub methods: Vec<ApiRow> }

pub struct ApiRow { pub name: &'static str, pub type_: &'static str,
                    pub default: &'static str, pub description: &'static str }
```

### 5.5 The shell

Two panes, as in the v1 design:

- **Left rail** — the catalogue, grouped by category. Each entry is a
  `floom_widgets::Button`; the selected one is highlighted. Selection lives in
  a `RwSignal<usize>` held by the shell; the detail pane re-declares its
  children when the signal changes (the `SignalText` pattern from
  `floom-showcase` already in the tree).
- **Right pane** — a `floom_widgets::Scroll` containing, for the selected
  component: the title, the intro, each sample in its own titled section, and
  the Props / Events / Methods tables.
- **Header** — the three design-system column labels, a dark-mode toggle, and
  the component count.

**The compare row.** Each sample renders as a three-column row, one column per
design system, each column labelled. The columns are the same component built
under `MultiTheme::pinned(ds)`, so each shows **its own design system's real
palette and metrics**.

Chrome is drawn with engine primitives (`Stack`, `Button`, `Text`, `Divider`,
`Surface`, `Scroll`) — theme-neutral, so it looks the same in all three design
systems and never competes with the component being previewed.

Dark mode is a global `set_dark(true)` on `floom-core-theme` (C9); it works
across all three columns because the concrete themes read the global flag live.

### 5.6 The props tables are derived, not retyped

Anti-goal A4. The props structs in `floom-core-theme/src/props/*.rs` are the
source of truth: plain `pub` fields plus a `Default` impl giving each default
(C16). The demo reads those sources with `include_str!` and parses them at
runtime with a small hand-written scanner (no `syn`, no build script, no
generated file to drift):

```rust
const BUTTON_PROPS: &str = include_str!(
    "../../crates/ecosystem/floom-ui/crates/floom-core-theme/src/props/button.rs"
);
```

The parser extracts, per `pub struct XProps { … }`: field name, field type,
and the matching `Default` initialiser. That yields the **name / type /
default** columns of the Props table automatically, for all 88 props modules.

The **description** column is prose and is hand-written per component. It is
filled in for every component, but the descriptions for the less common
components are one line each; where a prop has no interesting behaviour, the
cell is `—`. The Props table is therefore always *complete* (no missing
props), with descriptions at a quality that improves as they are written.

Events and Methods tables are hand-written: the engine's event surface is not
machine-readable, and the kits expose methods through their builders rather
than through a uniform trait.

### 5.7 Entry points and run modes

```rust
fn main() {
    // --headless: mount every page in turn, print painted counts (manual dev path)
    // default:    open a window via floom::run_desktop_with_devtools
}
```

**Verification (G7) lives in `#[test]` functions, not in `main --headless`.**
The gate's `test` check is `cargo test --workspace`, which runs `#[test]`s —
it does **not** run `main` with a flag. The `--headless` branch is a manual
diagnostic that prints a painted-count table.

Two headless `#[test]` files mirror `floom-showcase/src/tests.rs`:

- `tests/pages.rs` — uses `floom_testkit::TestApp` to mount each of the 98
  registry pages in turn and assert it paints (`host.frame().painted > 0`).
  `painted > 0` proves the page tree was declared and the host laid it out;
  this is the same bar the showcase holds, and it is honest about what it
  proves (the page shell, not a pixel of the component).
- `tests/interaction.rs` — for the interactive samples (button tap, checkbox
  toggle, switch flip, slider drag), asserts the exposed signal moves. Samples
  that wrap an overlay must be **revealed** in their sample builder (e.g.
  `Dialog::open(true)`) so the headless harness can observe them — a closed
  dialog paints no content of its own, and an unasserted "it compiled" would be
  a lie about coverage.

The desktop path uses `floom::run_desktop_with_devtools` — the same entry the
showcase uses — so F12 opens the inspector on a gallery page and a developer
can inspect the real widget tree of a real component.

### 5.8 Dependencies

Mirroring `examples/floom-showcase/Cargo.toml`:

```toml
[dependencies]
floom       = { path = "../../crates/app/floom", features = ["desktop", "renderer-gpu", "devtools"] }
floom-widgets = { path = "../../crates/widgets/floom-widgets" }
floom-signal  = { path = "../../crates/core/floom-signal" }
floom-core-theme = { path = "../../crates/ecosystem/floom-ui/crates/floom-core-theme" }
# + 294 kit crates (98 × 3), one path dep each

[dev-dependencies]
floom-testkit = { path = "../../crates/devtools/floom-testkit" }
```

The 294 path deps are **generated** from the satellite's directory listing by
a checked-in script (`tools/gen_deps.sh`), so they stay in step with the
satellite instead of being hand-maintained. The generated block is committed.

`floom-core-theme` is a direct dependency because the gallery needs the shared
prop types (`ButtonVariant`, `ButtonSize`, `AlertVariant`, …), the global
theme controls (`set_dark`), and `MultiTheme::pinned`.

**No new third-party dependencies.** The gallery is built entirely from
crates already in this repository.

### 5.9 The catalogue — 98 components in 6 categories

| Category | Count | Components |
|---|---:|---|
| Actions | 18 | Button, ButtonGroup, CheckboxBtn, ChipGroup, ConfirmEdit, Counter, ExpansionPanels, FloatingActionButton, IconButton, ItemGroup, ListGroup, ListSubheader, Radio, RadioGroup, Rating, SpeedDial, Stepper, Switch |
| Forms | 16 | Autocomplete, Checkbox, ColorPicker, Combobox, DatePicker, FileInput, Form, NumberInput, OTPInput, RangeSlider, Select, Slider, TextField, Textarea, TimePicker, ValidationMessages |
| Data display | 21 | Avatar, Badge, Calendar, Card, Carousel, Chip, Code, DataIterator, DataTable, Icon, Image, Kbd, List, ListItem, Pagination, Sparkline, Table, Text, Timeline, Treeview, VirtualScroller |
| Feedback | 9 | Alert, Banner, Dialog, EmptyState, Menu, Overlay, Sheet, Snackbar, Tooltip |
| Navigation & layout | 19 | App, AppBar, BottomNavigation, Breadcrumbs, Container, Divider, Footer, Grid, Hover, Main, NavigationDrawer, Parallax, Responsive, SlideGroup, Spacer, SystemBar, Tabs, Toolbar, Window |
| Motion, progress & providers | 15 | DateAdapterProvider, DefaultsProvider, DisplayProvider, ExpandTransition, FadeTransition, IconsProvider, InfiniteScroller, LocaleProvider, ProgressCircular, ProgressLinear, PullToRefresh, ScaleTransition, SkeletonLoader, SlideTransition, ThemeProvider |

**Components that do not render on their own.** The six providers are pure
service layers (they expose a signal and wrap a child; they have no visual
output of their own). Their pages show the API table and a note explaining
what they provide and how to read their signal, rather than an empty
thumbnail. The four transitions require a `show` flag, which their samples
set so the transition is observable. Both cases are stated on the page, not
hidden.

### 5.10 Verification

| Check | Command | Bar |
|---|---|---|
| Tests | `cargo test -p floom-demo` | all 98 pages mount and paint; a button tap moves its signal |
| Lint | `cargo clippy -p floom-demo --all-targets -- --deny warnings` | zero warnings |
| Format | `cargo fmt --all --check` | clean |
| CI shape | `cargo check --workspace --all-features` | compiles (C5) |
| Whole gate | `just gate` | no regression |
| Satellite | `just gate ecosystem` | no regression from the `MultiTheme` change |

The demo is *not* added to `FLOOM2_PACKAGES` (C4) — that list is clippy'd with
`--all-features`; with no features of its own this would be harmless, but the
list is about the engine track, not examples. It is covered by
`cargo test --workspace` (C3), which is the check that matters for an
example.

> **G7 wording.** "Paints" means the page tree is declared and the host laid it
> out (`host.frame().painted > 0`), exactly as `floom-showcase/tests.rs`
> asserts. It does **not** claim a pixel of the component was rendered; that
> bar is reserved for a later golden-image pass (N2). Overlays (Dialog) are
> revealed in their sample builders so the assert is not vacuously true.

### 5.11 The satellite `justfile`

The satellite's `justfile` runs `cargo … -p floom-demo` from the satellite
directory, where that package cannot exist (C1, C11). It is updated to point
at the root workspace:

```make
material:
    cargo watch --manifest-path ../../../../Cargo.toml -c -d 1 -x 'run -p floom-demo'
```

The broken-target note added during the P0 doc sweep is replaced by this
working target. Because all three design systems are now in one binary, the
`cupertino` and `fluent` targets are no longer separate builds; they become
the gallery's compare mode (or are dropped in favour of one target).

---

## 6. What's left to the builder

**Fixed by this document**

- Location, crate name, and the fact that it lives in the root workspace.
- All three design systems compiled into one binary; no Cargo features for DS.
- The compare mechanism (`MultiTheme::pinned` + `with_theme`) and the
  additive satellite change it requires (§5.2).
- The `ds` module's three namespaces and the compare-row macro.
- The gallery data model (`ComponentPage`, `Sample`, `ApiDoc`, `ApiRow`).
- The two-pane shell, the compare row layout, the dark toggle.
- The full 98-component catalogue and its 6 categories (§5.9).
- The props tables are derived from `floom-core-theme`'s sources (§5.6).
- The dependency list and the generated-depends script.
- The verification matrix, and the "not in `FLOOM2_PACKAGES`" decision.
- The satellite `justfile` repair.

**Free for the builder**

- Exact widget composition and spacing inside the shell; the chrome's colours
  and metrics; column widths in the compare row.
- Whether the props table is rendered as a grid of `Text` leaves or as
  purpose-built rows.
- How many samples per component, and which variants each shows.
- The wording of intros, notes and API descriptions.
- Whether the detail pane keeps its scroll position when the selection changes.
- Whether the compare row is 3 columns wide or 3 stacked rows on narrow
  windows.

**Out of the builder's hands (raised for the human partner)**

- **Approval of the additive `MultiTheme::pinned` change to
  `floom-core-theme`** (§5.2). It is the only change to the satellite, it
  touches the crate compiled into all 294 kits, and the side-by-side view
  depends on it. If it is refused, the fallback is a token-table comparison
  (`MaterialTheme` / `CupertinoTheme` / `FluentTheme` are concrete and
  per-DS) instead of three live columns.
- Whether the remaining work should land as one push (98 components, a
  ~300-line manifest, and roughly 98 page modules) or in category-sized
  pushes behind the same registry.

---

## 7. Verification of this document's claims

Every factual claim above was checked against the tree before writing:

- `cargo metadata --no-deps` in the satellite: 306 members.
- Set intersection of the three kit directories: **98 components in all
  three**; `icon-paths` is the only non-crate directory (C15).
- Per-crate survey of `pub fn (…) -> EngineType`: 94/99 uniform entry points;
  `card` and `switch` the outliers (C7).
- `diff` of `md-button` vs `cupertino-button` with the type names normalised:
  identical builder surface (C6).
- `md-button/src/lib.rs:50` vs `cupertino-button/src/lib.rs:50`: global signal
  vs. baked theme (C8).
- `floom-core-theme/src/theme.rs:499-521`: `activate()` is a no-op, `ds()`
  reads the global (C12).
- `floom-core-theme/tests/design_system_themes.rs:37-62`: the one-DS-per-
  process rule is explicit and tested (C12).
- Survey of `pub fn with_theme`: 294/296 kits (C13).
- `floom-widgets/src/controls.rs:329-360`: engine widgets hold concrete
  colours (C14).
- `floom-core-theme/src/theme.rs:199,310,421`: `is_dark()` consulted live
  (C9).
- `scripts/gate.sh:392,401` and `.github/workflows/ci-core.yml:346`: the
  clippy / test / MSRV commands quoted in §3 and §5.10.
- `crates/ecosystem/floom-ui/justfile` and root `Cargo.toml` `members` /
  `default-members`: C2, C3, C11.
