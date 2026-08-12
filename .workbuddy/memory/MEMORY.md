# floem-ui — long-term project notes

Vuetify-parity Rust UI library on Floem (git main, v0.2-era). Target: Material / Cupertino /
Fluent across **98 components × 3 design systems = 294 per-component crates** + a neutral
`floem-elements` facade (feature-gated re-exports) + `floem-core-theme` (shared props/tokens/
theme) + `floem-widget-runtime` (Material-only facade).

## Conventions
- One crate per component per design system. Material crates live at `crates/material/md-*`,
  cupertino at `crates/cupertino/cupertino-*`, fluent at `crates/fluent/fluent-*`.
- Scaffolding / swap-to-cupertino-fluent / wire-into-floem-elements are done with Python
  scripts (zsh doesn't word-split unquoted `$var`). `DIR_PREFIX = {material:md, cupertino:
  cupertino, fluent:fluent}`.
- `floem-elements` exposes neutral names (`App`, `Main`, `Button`, …) via per-feature
  `pub use` in `app.rs` / `main_view.rs` / etc., re-exported from the prelude.
- Never name a dispatch file `main.rs` inside a lib crate — Cargo treats it as a binary
  entry point. Use `main_view.rs` (that's where `Main` dispatch lives).
- **`floem-elements` wiring gotcha (feature-array must match `[dependencies]`):** the 9 per-DS
  widget crates are `optional = true` deps in `floem-elements/Cargo.toml`. Adding them under
  `[dependencies]` is NOT enough — they MUST also be listed in **all three** `material` /
  `cupertino` / `fluent` feature arrays, or the facade's `#[cfg(feature="…")] pub use
  floem_widget_{ds}_{comp}::…` import is unresolved → `E0432 unresolved import`. The scaffold
  script `scaffold_batchN.py` adds the deps, but the feature-array insertion used an anchor that
  broke after Batch 19 inserted before `validation_messages`; if a feature check fails with
  `E0432` for a just-added component, grep the feature arrays for the missing
  `floem-widget-{ds}-{comp}` entry and add it (anchor: the `validation_messages` line is the
  last entry of each array).

## Floem style API gotchas
- NO per-side padding methods (`padding_top/bottom/left/right` do not exist) — only
  `padding_horiz` / `padding_vert` and `_pct` variants. Use reactive `Empty` spacers for
  insets instead.
- Absolute positioning: `s.absolute().inset_top(0.0).inset_left(0.0).width_full()` etc.
  `.width_full()` / `.width_pct(100.0)` / `.height_pct(100.0)` exist; percentage sizes need
  a definite-height/width ancestor or they collapse.
- `AnyView = Box<dyn View>`; stored children go into `Container::new(child)` directly
  (`impl View for Box<dyn View>`). `into_any()` converts a `View` to `AnyView`.
- **Facade components are builders, NOT `IntoView`.** Every `floem-elements` neutral type
  (`Container`, `Text`, `Button`, `Slider`, … i.e. `Md*`/`Cupertino*`/`Fluent*`) has a `view()`
  method that returns `impl View`; it does NOT implement `IntoView`/`Decorators` itself. So you
  CANNOT write `Container::new(...).style(...)` or pass `Text::new(x).into_any()` directly —
  that fails with E0599 ("doesn't satisfy `IntoView`"). Instead: `Container::new().child(x)
  .view().style(...)` and `Text::new(x).view().style(...)` (or `.view().into_any()`). The
  `Container` facade `new()` also takes **0 args** — child goes through `.child(impl IntoView)`,
  never `Container::new(child)` (that's only the floem built-in `floem::views::Container`).

## Foundation / token-factory gotchas
- Adding a new component's tokens requires editing `MaterialComponentTokens` in
  `crates/floem-core-theme/src/component_tokens/material.rs` in **THREE places**: the struct
  field, the `new()` init, AND both `cupertino()` and `fluent()` inits — else E0063 "missing
  fields". Also add `ComponentId` + `ComponentTokens` enum variants and the `get()` arm.
- `floem_core_theme::tokens::color::Color` is a **custom** struct (r,g,b,a) with `.to_peniko()`.
  It is NOT peniko's `AlphaColor`. Because `floem::prelude::*` glob-imports peniko `Color`, you
  MUST add an explicit `use floem_core_theme::tokens::color::Color;` in any widget that names a
  `Color` field — otherwise the field type resolves to peniko and `theme.colors().<x>` (a
  floem_core_theme `Color`) fails to assign (E0308).

## "open_signal()" pattern (component-owned reactive state)
- Components that expose external reactive control use a getter that RETURNS the component's
  **internal** `RwSignal` (Dialog/Menu `open_signal()`, Form `valid_signal()`/`errors_signal()`,
  ValidationMessages `messages_signal()`). The getter takes NO argument and does NOT set state —
  the consumer obtains the signal and drives it (e.g. `form.valid_signal().set(ok)` inside a
  `create_effect`). Passing an argument to these getters is a compile error (E0061).

## "selected(usize) / on_change(impl Fn(usize))" pattern (internal RwSignal<usize>)
- Components with an active-item index (BottomNavigation, Stepper) hold it in an internal
  `RwSignal<usize>`, set it on click, and fire `on_change(i)`. A `dyn_view` renders the items
  so selection updates re-render reactively. `Breadcrumbs` follows the same `on_click(impl
  Fn(usize))` click-reporting shape (last item = current, non-clickable). Stepper adds
  `editable(bool)` (click completed steps) + `non_linear(bool)` (click any step).

## Providers service-layer pattern (added 2026-08-11, Batch 13)
- Providers (ThemeProvider / LocaleProvider / DisplayProvider / DefaultsProvider / IconsProvider
  / DateAdapterProvider) are a PURE service layer with NO visual tokens → they do NOT add
  `ComponentId`/`ComponentTokens`/`material.rs` factory wiring. Foundation = only `props/*.rs`
  + `props/mod.rs` registration. Material crates need only `floem` + `floem-core-theme` (NOT
  `floem-widget-runtime`).
- Each provider stores an `RwSignal<T>` seeded from props and exposes a typed getter
  (`dark_signal`/`locale_signal`/`density_signal`/`defaults_signal`/`icons_signal`/`adapter_signal`)
  — same no-arg getter contract as `open_signal()`. `child(impl IntoView)` wraps the subtree.
- Documented limitation: Rust/Floem static dispatch means subtree-scoped re-theming is NOT
  enforced by the type system; descendants read the getter and react. (No scoped-context.)
- `new()` must `.clone()` `String`/`HashMap` props before moving the struct field (E0382
  partial move). Default child uses `Empty::new().into_any()` (deprecated `empty()` → warning).
- Cupertino/Fluent provider crates are byte-identical to Material except the struct name
  (`Md{Provider}`→`{Ds}{Provider}`); the swap still runs the leak assertions.

## Transition / `dyn_view` enter-leave animation pattern (Batch 18)
- A real enter/leave animation uses `dyn_view(move || -> AnyView { if show.get() {
  Container::new(child).animation(...) } else { Empty::new().into_any() } })`. Floem plays the
  animation's keyframe 0→100 on mount (`run_on_create(true)`) and REVERSES 100→0 on unmount
  (`run_on_remove(true)`) — swapping `Container`↔`Empty` gives both enter and leave for free.
- The child MUST be a factory closure `impl Fn() -> impl IntoView` stored as
  `Option<Box<dyn Fn() -> AnyView>>`, because the child is re-mounted/re-unmounted on every
  transition and `AnyView` is not `Clone` (can't be shared between the two `dyn_view` branches).
- CRITICAL `'static` closure rule: any `.animation(|a| …)` / `keyframe(.., |f| …)` closure that
  captures component state must capture by `move` using a pre-extracted **Copy** value, never by
  borrowing `style` from the outer `dyn_view` closure. A bare `|a| { … style.x … }` fails with
  E0495/E0621 "lifetime may not live long enough" because the stored animation closure needs
  `'static` but borrows the outer closure's captured `style`. Fix: `let x = style.x;` (f32/enum/
  `Pct` are Copy) then `.animation(move |a| … x …)`. This hit Fade/Scale/Expand/Slide.
- `Container` animation style methods available: `opacity(f32)`, `scale(Pct)` (`s.scale(100.pct())`),
  `translate_x`/`translate_y(impl Into<Length>)`, `height(f32)`. `ExpandTransition` clips with
  `.style(|s| s.overflow_x(Overflow::Clip).overflow_y(Overflow::Clip))`.
- **`child` API INCONSISTENCY (demo gotcha, 2026-08-12):** Transition `child` requires a
  **closure factory** `Fn() -> impl IntoView` (because the child is rebuilt on every
  enter/leave mount and `AnyView` isn't `Clone`). `Overlay::child` by contrast accepts an
  `impl IntoView` **value**. Demo code that copies Overlay's `.child(value.into_any())` shape
  into a Transition fails with `E0277: expected a Fn() closure, found dyn View`
  (e.g. `SlideTransition::child`). Fix: pass `|| { view }` and drop the inner `.into_any()`.

## App / Main shell + layout provider (added 2026-08-11)
- `floem-core-theme::layout`: `layout_signal() -> RwSignal<LayoutState>` (thread-local
  `OnceCell`), `register_layout(slot, f32)`, `LayoutState` with `top()/bottom()/left()/right()`.
  DS-agnostic so shell primitives need no `floem-widget-runtime` dep (use fully-qualified
  `floem_core_theme::layout::*` paths).
- `App::child(content)` = full-viewport frame that initializes the signal. `Main::child(content)`
  = content region that reserves reactive top/bottom/left/right spacers from the signal so it
  never hides under a bar or open drawer.
- Composition contract: bars (AppBar, Footer, SystemBar, BottomNavigation) and
  NavigationDrawer are ABSOLUTE overlays over a full-viewport `Main` — never flex siblings of
  `Main` (would double the top/bottom inset). NavigationDrawer is already absolute; wrap
  AppBar/Footer in an absolute `Container` when composing the shell. Registration still fires
  through the wrapper, so `Main` insets correctly.

## Verification discipline
- Always `cargo check -p floem-demo` for material (default), `--no-default-features
  --features cupertino`, `--no-default-features --features fluent`; plus `cargo check --workspace`.
  NOTE: `floem-demo --features fluent` alone keeps the material default → both features enabled
  → the `compile_error!` DS-guard in `floem-elements` fires. Always pass `--no-default-features`.
  Likewise `cargo check -p floem-elements` needs an explicit `--features material` (its
  `[features]` has no `default`).
- Prior sessions also required host + android-target dual 0-error compiles for the floem
  Android embedder work (separate from this UI library).
- **Background `cargo check` timing pitfall (2026-08-12):** when a `cargo check` runs in the
  background, do NOT read its log until the task has TRULY finished. A premature `grep '^error'`
  while the build is still compiling yields a false "0 errors" (later crates' errors like
  `floem-demo`'s E0277 may not be flushed yet). Always confirm a `Finished` line exists in the
  log, or `TaskOutput(block=true)` on the task, BEFORE declaring a build green. The workspace
  check printing no `Finished` while `errors=0` is the tell-tale of a half-read log.

## Demo gallery (`examples/demo`) — two-pane master/detail (2026-08-12)
- `examples/demo/src/main.rs` is a two-pane: LEFT = clickable list of all 98 component names
  (`selected: RwSignal<usize>`; active row reactive-highlighted); RIGHT = `Scroll`+`dyn_view`
  rendering `gallery::build().pages[selected].1()`. Window 1040×900.
- `examples/demo/src/gallery.rs` owns the content: `build() -> Gallery { pages: Vec<(String,
  Page)>, overlays: Vec<AnyView> }`, `Page = Box<dyn Fn() -> AnyView>`. 98 pages (one per
  component, canonical order), each wraps its body in `page(title, content)` (a `v_stack` with a
  heading). Edit a component's demo by editing its `pages.push(...)` factory here.
- **Floating surfaces MUST render at the WINDOW ROOT, never inside a scroll list.** Dialog /
  Menu / Snackbar / NavigationDrawer / Sheet / Overlay are `Overlay`-based; their scrim uses
  `width_pct(100%).height_pct(100%)` which collapses to 0 inside a flex list item → invisible /
  unclickable. `build()` returns these 6 views in `overlays`; `main.rs` wraps each in an
  `absolute().inset_top(0).inset_left(0).width_full().height_full()` container at the root. Their
  page factories only show the trigger button + a description. (This was the fix for "dialog
  click does nothing".)
- `create_effect`s that must persist (Form validity, PullToRefresh completion) are created ONCE
  in `build()` (inside the window view's reactive scope) and the views are hosted in a
  `SharedView` (`Rc<RefCell<Box<dyn View>>>`, manual `View` delegation + `Clone`) so the page
  factory can re-render them on selection without rebuilding or leaking effects.
