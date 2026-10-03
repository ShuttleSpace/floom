# Floom workspace baseline (pre–2.0)

**Date:** 2026-09-04  
**Git:** `0715ac3` on branch `floom-2.0`  
**Host:** macOS 26.5.1 arm64, `rustc 1.98.0` (Homebrew)  
**Related decision:** [`../decisions/2026-09-04-no-legacy-view-adapter.md`](../decisions/2026-09-04-no-legacy-view-adapter.md) (Policy A: no Legacy View adapter)

This inventory freezes the repository shape before Floom 2.0 Phase 1 crate topology lands.

---

## 1. Workspace members

`Cargo.toml` workspace `resolver = "3"`, `exclude = ["platform-compat"]`.

### Default / library members

| Package | Path |
|---|---|
| `floom` | `.` |
| `floom_reactive` | `reactive/` |
| `floom_renderer` | `renderer/` |
| `floem_vger_renderer` | `vger/` |
| `floom_vello_renderer` | `vello/` |
| `floom_skia_renderer` | `skia/` |
| `floom_tiny_skia_renderer` | `tiny_skia/` |
| `ui-events-floem-winit` | `ui-events-winit/` |
| `floom-test` | `test/` |

### Examples (`examples/*`)

Includes desktop demos (`counter`, `widget-gallery`, …) and platform samples (`floom-android-example`, `floom-ios-example`, `ohos`).

### Not workspace members

- `platform-compat/` — vendor trees (`floem-winit`, `tgui-winit-ohos`, `harmony-app-rs`); pulled via `[patch]` only.

### Floom 2.0 crates (absent)

No `floom-types`, `floom-core`, `floom-runtime-core`, `floom-scene`, `floom-platform`, etc.

---

## 2. Feature flags (`floom` package)

| Feature | Notes |
|---|---|
| **default** | `default-image-formats`, `vger`, `serde` |
| `vger` / `vello` / `skia` | Optional GPU backends (`tiny-skia` always linked as CPU fallback) |
| `serde` | Optional serialization |
| `tokio` | Optional async |
| `crossbeam` | Optional + renderer feature |
| `localization` | Fluent / locale |
| image-* | Per-format image codecs |

`editor` feature: **removed** (Industrial 1.0); editor lives in sibling `floom-editor`.

---

## 3. Public entry points (compatibility seams)

### App / window

- `floom::launch`, `floom::Application`, `floom::AppConfig`, `floom::AppEvent`
- `floom::new_window`, `floom::close_window`, `floom::WindowState`, …
- Modules: `app`, `window`

### View tree (legacy — delete without shim under Policy A)

- `floom::View`, `floom::ViewId`, `floom::IntoView`, `floom::AnyView`, `floom::ParentView`
- Modules: `view`, `views`, `style`, `context` (internal)

### Reactive

- Crate: `floom_reactive`
- Re-exports: `floom::reactive`, crate-root `RwSignal`, `Scope`, `create_*`, `prelude`

### Layout / text / paint

- Layout: Taffy via `floom::taffy`, `layout` module, `ScreenLayout`
- Text: `text` module + Parley/Fontique via renderer
- Paint: `paint::renderer::Renderer` enum wrapping vger/vello/skia/tiny-skia

### Platform / embedder

- `platform` (clipboard, file, menu, mobile bridges)
- `embedder` (capabilities, surface, IME, android/ios/ohos)
- `accessibility`, `headless` (`HeadlessHarness`)

### IDs already present

- `ViewId` — primary view identity (`crates/floom/src/view/id.rs`)
- `floom::ElementId` — re-export from `understory_box_tree` (not the 2.0 generation-safe ElementId yet)

---

## 4. Renderer / platform assumptions

| Concern | Current |
|---|---|
| Windowing | `floem-winit` (git + `platform-compat` patch for iOS) |
| OHOS | `tgui-winit-ohos` patched onto `floem-winit-core` |
| Desktop a11y | accesskit (macOS/Windows/Linux) |
| Mobile | embedder backends + examples |
| Scene graph | **None** — paint goes through per-view `Renderer` |
| Platform ABI crate | **None** — OS details mixed into `platform` / `app` / `window` |

---

## 5. Test baseline (recorded)

**Command:** `cargo test --workspace --no-fail-fast`  
(`cargo nextest` not installed on this host.)

**Totals (sum of all `test result:` lines, including doctests):**  
~**1084** passed, **4** failed, **72** ignored.

### Known failures (pre-existing at baseline)

1. **`floom-test` / `embedder_protocol`**
   - `capabilities_winit_default_reports_desktop_features`
   - Panic: `assertion failed: !caps.accessibility`  
     (desktop winit default now reports accessibility; test expectation stale.)

2. **`floom` doctests** (`crates/floom/src/views/dropdown.rs`)
   - Three doctests fail compile: `unresolved import strum`

These are **baseline defects**, not regressions introduced by Floom 2.0 work. Fix opportunistically or in a dedicated hygiene task; do not treat as Phase 0 blockers unless they block new crates.

---

## 6. Benchmark baseline (commands)

Existing Criterion benches (root `Cargo.toml` `[[bench]]`, `harness = false`):

```bash
cargo bench -p floom --bench event_dispatch
cargo bench -p floom --bench style_computation
cargo bench -p floom --bench painting
cargo bench -p floom --bench layout
cargo bench -p floom --bench animation
cargo bench -p floom --bench text
cargo bench -p floom --bench view_creation
# or all:
cargo bench -p floom
```

**Gap vs Floom 2.0 Task 0.3:** no `runtime_baseline` / `scene_baseline` / `render_baseline` / memory report yet — those land in Task 0.3.

---

## 7. Likely Floom 2.0 seams

| Seam | Today | 2.0 target |
|---|---|---|
| Types / IDs | `ViewId` + understory `ElementId` | `floom-types` generation-safe IDs |
| Runtime | View storage + reactive effects | ElementArena + DirtyFlags + scheduler |
| Scene | Implicit in paint | `floom-scene` |
| Renderer | Concrete enum in `paint` | Renderer ABI backends |
| Platform | Mixed modules | Platform ABI crates |
| Legacy View | Public API | **Removed** (Policy A; no adapter) |

---

## 8. CI today

`.github/workflows/ci.yml`: fmt, clippy, `cargo build`, `cargo nextest run --workspace`, macOS doctests.  
No architecture-dependency boundary check yet (Task 0.2).
