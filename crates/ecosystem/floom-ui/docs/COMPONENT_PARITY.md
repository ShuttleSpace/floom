# Component Parity Roadmap

Status: working roadmap — reconciled to the v2 tree 2026-11-11  
Last updated (batch history): 2026-08-12 (Batch 20: inputs & interaction — NumberInput + PullToRefresh + ConfirmEdit — 98 components × 3 design systems = 294 crates; Batch 19: tier-2 components — IconButton + ExpansionPanels + SpeedDial + ChipGroup + ListGroup + Counter + Responsive + CheckboxBtn + ListSubheader — 95 components × 3 design systems = 285 crates; Batch 18: Transitions — Fade + Scale + Expand + Slide — 86 components × 3 design systems = 258 crates; Batch 17: Media and motion — Parallax + InfiniteScroller + Hover — 82 components × 3 design systems = 246 crates; Batch 16: Radio + Overlay = 237 crates; Batch 15 Data display = 231 crates; Batch 14 Actions/controls = 213 crates; Batch 13 Providers service layer = 198 crates)

This project targets a Floom component library with a Vuetify-like out-of-the-box developer experience across Material, Cupertino, and Fluent design systems.

> **Current state (reconciled 2026-11-11; design-system merge 2026-11-15).** The tree now
> has **9 workspace members** (verified via `cargo metadata`): `floom-widget-runtime`,
> `floom-core-theme`, `floom-theme`, `floom-design`, `floom-ui-composites`, `floom-material`,
> `floom-cupertino`, `floom-fluent`, `floom-testing`. On 2026-11-15 the 296 per-component kit
> crates (98 material + 99 cupertino + 99 fluent) were merged into the three `floom-<ds>`
> kit crates, one `#[cfg(feature)]` per component (web-sys style), and **`floom-harmony` was
> deleted** (it was consolidated-only). See
> `docs/superpowers/plans/2026-11-15-floom-ui-feature-kits.md` for the merge. The kits'
> `default = []` keeps component code out of a default build; `cargo test --workspace
> --all-features` turns every component test on. The **`floom-elements` re-export facade no
> longer exists** (removed in "Stage D"); each kit crate re-exports its own neutral builder
> directly over `floom-ui-composites` + the engine `floom-widgets` primitives. A
> **`floom-demo` gallery crate does exist**, at `examples/floom-demo` in the **root**
> workspace — it mounts the components of Material, Cupertino and Fluent side by side and its
> headless tests cover every page and every sample.
>
> The batch narratives below are **history**: they describe the v1 pipeline
> ("→ floom-elements facade → demo → verify") and were true when each batch shipped. Read
> them as the build record, not as the current wiring — where they say `floom-elements` /
> `facade` read *per-kit neutral re-export*, and where they say `demo` read *planned gallery
> (PLAN.md P1)*. The current-state sections (`view()` status, implementation order, acceptance
> criteria) are corrected to today's tree.

Primary references:

- Vuetify All Components: https://vuetifyjs.com/en/components/all/
- Vuetify repository: https://github.com/vuetifyjs/vuetify
- Vuetify 0 repository: https://github.com/vuetifyjs/vuetify0

The Vuetify documentation page is the source of truth for component inventory. If this file and the official page diverge, update this file before implementing more components.

## Current implemented contract

Phase 1 has a cross-design-system implementation for these 98 components. All three design
systems ship a real, renderable, interactive `view()` (composes the engine `floom-widgets`
primitives + `floom-ui-composites` + token-first theme styling). Those crates are exercised
headlessly by `cargo test --workspace` in the satellite (CI `ecosystem-check` / `just gate
ecosystem`); a single runnable gallery binary now exists as well — `examples/floom-demo`
in the root workspace, showing the three design systems side by side (see the current-state
note above and `PLAN.md` P1). The batch narratives below record the v1 pipeline that built
them.

- The first 10 (Button, Text, TextField, Checkbox, Switch, Dialog, Menu, Card, Icon, Divider)
  were the Phase-1 parity milestone.
- The next 5 (Slider, RadioGroup, Tooltip, Tabs, List) are the "extended controls" batch added
  2026-08-10.
- Batch 3 (Select, Textarea, Snackbar, Chip, Badge) added 2026-08-10 — every one implemented
  in all three design systems with the same neutral builder API.
- Batch 4 (AppBar, NavigationDrawer, BottomNavigation, ProgressLinear, ProgressCircular) added
  2026-08-10 — the start of the layout/navigation shell + feedback components; every one
  implemented in all three design systems with the same neutral builder API.
- Batch 5 (Avatar, Alert, Rating, Pagination, Timeline) added 2026-08-10 — display &
  small-interactive components; every one implemented in all three design systems with the
  same neutral builder API. The library now ships **30 components × 3 design systems = 90
  crates**, plus the `floom-elements` neutral re-export facade.

- Batch 6 (Image, Banner, SkeletonLoader, Toolbar, Sheet) added 2026-08-10 — display,
  feedback, and surface components; every one implemented in all three design systems with
  the same neutral builder API. The library now ships **35 components × 3 design systems =
  105 crates**, plus the `floom-elements` neutral re-export facade.

- Batch 7 (Container, Grid, Spacer, Footer, SystemBar) added 2026-08-10 — the remaining
  "Application and layout" primitives (the layout/navigation shell is now complete except
  for `App`/`Main`/`Layout provider`). Every one implemented in all three design systems
  with the same neutral builder API. `Container` centers and caps content width; `Grid`
  chunks children into N equal-width rows (`h_stack_from_iter` + `list`, no CSS grid in this
  Floom build); `Spacer` is a flex-growing empty view; `Footer` is a bottom bar with optional
  border + elevation; `SystemBar` is a slim top status strip with a dark variant. The library
  now ships **40 components × 3 design systems = 120 crates**, plus the `floom-elements`
  neutral re-export facade.

- Batch 8 (Combobox, Autocomplete, DatePicker, TimePicker, EmptyState) added 2026-08-10 —
  the inputs/pickers group (the remaining "Text entry and selection" + "Pickers" +
  "Feedback and overlays" Vuetify slots). Every one implemented in all three design systems
  with the same neutral builder API. `Combobox` is a searchable single-select dropdown
  (`TextInput` + chevron, case-insensitive filter); `Autocomplete` is the same shape but
  commits the typed free text on `FocusLost`; `DatePicker` is a 6×7 reactive month-grid
  popover; `TimePicker` is an hour/minute stepper popover; `EmptyState` is a centered
  icon/title/description/child placeholder. The library now ships **45 components × 3 design
  systems = 135 crates**, plus the `floom-elements` neutral re-export facade.

- Batch 9 (DataTable, Treeview, Calendar, Carousel, VirtualScroller) added 2026-08-10 —
  the data-heavy + media collection/display group (the remaining "Data tables" + "Trees" +
  "Pickers" + "Carousels" Vuetify slots). Every one implemented in all three design systems
  with the same neutral builder API. `DataTable` is a header + striped/dense body with
  per-column alignment and row-click (owned `Vec<Vec<String>>` props, no `AnyView`);
  `Treeview` is a recursive collapsible node list (`Vec<TreeviewNode>` props, `Rc<dyn Fn>`
  toggle, rebuilds on expand via `dyn_view`); `Calendar` is a fixed 6×7 reactive month grid
  with optional ISO week-number column (reuses the DatePicker date math); `Carousel` renders
  all slides as absolutely-positioned `AnyView`s with arrow/dot navigation; `VirtualScroller`
  is a windowed spacer list driven by `PointerWheel` with lazy `Box<dyn Fn() -> Box<dyn View>>`
  item factories and an `on_reach_end` callback. The library now ships **50 components × 3
  design systems = 150 crates**, plus the `floom-elements` neutral re-export facade.

- App + Main + layout provider added 2026-08-11 — closes the "Application and layout"
  section. `App` is the root frame that initializes the process-wide layout signal
  (`layout_signal`) and paints the app background; `Main` is the content region that reads
  the signal and auto-insets its content via reactive spacers so it never hides under a
  fixed bar or an open navigation drawer. The 5 shell primitives (AppBar, NavigationDrawer,
  BottomNavigation, Footer, SystemBar) auto-register their sizes into the signal, so `Main`
  reacts with zero wiring. The library now ships **52 components × 3 design systems = 156
  crates**, plus the `floom-elements` neutral re-export facade and the `floom-core-theme`
  layout provider.

- Batch 10 (Form + ValidationMessages) added 2026-08-11 — the "Form" grouping surface with
  a validity-gated submit button and a standalone `ValidationMessages` ("校验提示") primitive
  that renders a reactive error list in the theme `error` colour. Every one implemented in all
  three design systems with the same neutral builder API. `Form` owns its internal
  `valid_signal()` / `errors_signal()` (consumer-driven, mirroring `open_signal()`);
  `ValidationMessages` owns its `messages_signal()` for the bound `Vec<String>` of errors. The
  library now ships **54 components × 3 design systems = 162 crates**.

- Batch 11 (Stepper + Breadcrumbs) added 2026-08-11 — the "Navigation" group. `Stepper`
  renders an ordered, horizontal progression of circular step markers joined by connectors,
  holding the current step in an internal `RwSignal<usize>` (mirrors `BottomNavigation`'s
  `selected`/`on_change`); `editable` lets you click back to a completed step, `non_linear`
  lets you jump to any step, and `on_change(impl Fn(usize))` reports the activated index.
  `Breadcrumbs` renders a `>`-separated clickable trail (last item is the current,
  non-clickable location) and fires `on_click(impl Fn(usize))` for earlier items. Every one
  implemented in all three design systems with the same neutral builder API. The library now
  ships **56 components × 3 design systems = 168 crates**.

- Batch 12 (Window + SlideGroup + ItemGroup + ColorPicker) added 2026-08-11 — the remaining
  "Navigation" + "Pickers" Vuetify slots. `Window` is a carousel-style slide stack with
  arrow/dot navigation and `selected`/`on_change(impl Fn(usize))`; `SlideGroup` is a
  horizontally scrollable item strip with active-indicator and `show_arrows`/`mandatory`;
  `ItemGroup` is a single/multi selectable group owning `selected_signal()` /
  `selected_multi_signal()` (mirrors `open_signal()`); `ColorPicker` is an HSV SV-square + hue
  strip + hex input with drag interaction and `color_signal()` / `on_change(Color)`. Every one
  implemented in all three design systems with the same neutral builder API, exercising the
  full foundation → scaffold → Material source → Python swap → facade → demo → verify pipeline.
  The library now ships **60 components × 3 design systems = 180 crates**.

- Batch 13 (Providers service layer: ThemeProvider / LocaleProvider / DisplayProvider /
  DefaultsProvider / IconsProvider / DateAdapterProvider) added 2026-08-11 — the Vuetify
  "Providers and services" group, implemented as a pure service layer (no visual tokens, so
  **no** `ComponentId`/`ComponentTokens`/`material.rs` factory wiring). Each provider stores a
  `RwSignal<T>` seeded from its props and exposes a typed getter
  (`dark_signal` / `locale_signal` / `density_signal` / `defaults_signal` / `icons_signal` /
  `adapter_signal`); the builder's `child(impl IntoView)` wraps the subtree. Because
  Rust/Floom uses static dispatch, subtree-scoped re-theming is NOT enforced by the type system
  — descendants read the signal getter and react (documented limitation). All 18 provider
  crates (material/cupertino/fluent) compile; the Cupertino/Fluent copies are byte-identical to
  Material except for the struct name. The library now ships **66 components × 3 design systems
  = 198 crates**.

- Batch 14 (Actions and controls: ButtonGroup / FloatingActionButton / RangeSlider / FileInput /
  OTPInput) added 2026-08-11 — the Vuetify "Actions and controls" group. Every one implemented in
  all three design systems with the same neutral builder API, following the standard foundation →
  scaffold (15 crates) → Material source → swap-to-Cupertino/Fluent → floom-elements facade → demo
  → verify pipeline. `ButtonGroup` is a `joined`/`vertical` segmented layout of child views;
  `FloatingActionButton` is the circular/elevated (or extended) primary action; `RangeSlider` is a
  two-thumb track reusing the `Slider` pointer-drag pattern (guarantees low ≤ high); `FileInput`
  is a trigger + path readout whose `on_change(Option<String>)` fires on click so the host opens a
  real picker and writes back via `path_signal()` (no OS dialog in this UI library); `OTPInput` is
  `length` single-char `TextInput` boxes bound to a master `code` string with optional `masked`
  (•) display. The library now ships **71 components × 3 design systems = 213 crates**.

- Batch 15 (Data display: ListItem / Table / DataIterator / Sparkline / Kbd / Code) added
  2026-08-11 — the remaining Vuetify "Data display" group (distinct from the already-shipped
  `DataTable`). Every one implemented in all three design systems with the same neutral builder
  API, following the standard foundation → scaffold (18 crates) → Material source → swap-to-
  Cupertino/Fluent → floom-elements facade → demo → verify pipeline. `ListItem` is a clickable
  row with optional `leading`/`trailing` `AnyView` slots + `title`/`subtitle`/`nav`/`disabled`/
  `on_click`; `Table` is a simple header + bordered/striped body (owned `Vec<Vec<String>>`, no
  `AnyView`) — lighter than `DataTable`; `DataIterator` wraps `items(Vec<AnyView>)` with an
  optional `header`/`footer` and 1px `divided` separators; `Sparkline` renders the `values` as
  theme-coloured vertical bars (no SVG — guaranteed token-coloured fill via `with_alpha(primary,
  90)`) with `width`/`height`/`fill`/`line_width`; `Kbd`/`Code` are monospace, `surface_variant`-
  background inline marks for keyboard shortcuts and code snippets, both accepting `content(String)`
  or `child(impl IntoView)` and a `mono(bool)` toggle. The library now ships **77 components × 3
  design systems = 231 crates**.

- Batch 16 (Radio + Overlay) added 2026-08-11 — fills the remaining Vuetify "Actions and
  controls" slot (`Radio`, distinct from the already-shipped `RadioGroup`) and the "Feedback and
  overlays" slot (`Overlay`). Every one implemented in all three design systems with the same
  neutral builder API, following the standard foundation → scaffold (6 crates) → Material source
  → swap-to-Cupertino/Fluent → floom-elements facade → demo → verify pipeline. `Radio` is an
  **independent** single radio button (binds `checked: bool` + internal `RwSignal<bool>`; click
  toggles and fires `on_change(bool)`; `value(String)` is an informational group label and
  mutual exclusion is handled by the host — this is deliberately different from `RadioGroup`'s
  `Option<usize>` + `Vec<String>` list model). `Overlay` is a scrim-based base overlay layer
  (`Overlay::new(scrim)` like Dialog/Sheet): `show(bool)` + component-owned `show_signal()`
  (mirrors `open_signal()`), `child(impl IntoView)` content layer, `on_click(impl Fn())` fired on
  scrim tap, and `opacity(f32)` (0.0–1.0) mapped to the scrim alpha; tapping the scrim also
  `show.set(false)` for auto-close, while the content layer stops click propagation
  (`EventPropagation::Stop`) so interior taps don't close the overlay. The library now ships **79
  components × 3 design systems = 237 crates**.

- Batch 17 (Parallax + InfiniteScroller + Hover) added 2026-08-11 — the remaining Vuetify
  "Media and motion" slots (3 of the 10 in that group; the 4 `*Transition` components are
  deferred to Batch 18 pending a Floom animation-API assessment). Every one implemented in all
  three design systems with the same neutral builder API, following the standard foundation →
  scaffold (9 crates) → Material source → swap-to-Cupertino/Fluent → floom-elements facade →
  demo → verify pipeline. `Parallax` is a host-scroll-driven translate (the host owns the
  `scroll_signal()`, `view()` offsets the child by `-scroll * factor`); `InfiniteScroller` is a
  wheel-driven windowed viewport that mirrors `md-virtual_scroller`'s `PointerWheel` +
  `ScrollDelta` + `RwSignal<f32>` offset + `Overflow::Clip` pattern and fires `on_load_more`
  when scrolled within `threshold` of the end (component owns `loading_signal()`); `Hover` uses
  `listener::PointerEnter`/`PointerLeave` to track a `hovered_signal()` and tints its background
  with `with_alpha(on_surface, highlight_alpha)` (default `highlight_alpha` 8 / `transition_ms`
  120). The library now ships **82 components × 3 design systems = 246 crates**.

- Batch 18 (Transitions: Fade / Scale / Expand / Slide) added 2026-08-11 — closes the
  remaining Vuetify "Media and motion" slots (the 4 `*Transition` components deferred from
  Batch 17 pending a Floom animation-API assessment). Every one implemented in all three design
  systems with the same neutral builder API, following the standard foundation → scaffold (12
  crates) → Material source → swap-to-Cupertino/Fluent → floom-elements facade → demo → verify
  pipeline, using Floom 0.2.0's real animation system for **both** an enter (mount → keyframe
  0→100) and a leave (unmount → reverse 100→0) animation. `FadeTransition` interpolates
  `opacity` 0→1; `ScaleTransition` interpolates `scale` from `min_scale`→1.0; `ExpandTransition`
  animates `height` 0→natural with the box `Overflow::Clip`-clipped; `SlideTransition` translates
  `offset` px on the X (default) or Y axis → 0. Each owns a `show_signal() -> RwSignal<bool>`
  (mirrors `open_signal()`) that an external trigger flips to play enter/leave, and a
  `child(impl Fn() -> impl IntoView)` factory closure because the child is re-mounted/re-unmounted
  on every transition.   The library now ships **86 components × 3 design systems = 258 crates**.

- Batch 19 (tier-2 components: IconButton / ExpansionPanels / SpeedDial / ChipGroup / ListGroup /
  Counter / Responsive / CheckboxBtn / ListSubheader) added 2026-08-12 — the remaining real Vuetify
  tier-2 slots (icon-button under "Buttons", "Expansion panels", "Speed dials", "Chip groups", the
  "Lists" group sub-header/list-group, "Inputs and controls" counter, "Layout" responsive utility,
  and the button-style checkbox). Every one implemented in all three design systems with the same
  neutral builder API, following the standard foundation → scaffold (27 crates) → Material source →
  swap-to-Cupertino/Fluent → floom-elements facade → demo → verify pipeline. `IconButton` renders
  via `svg()` (pass a real SVG string, not an icon name) with Text/Contained/Outlined
  `IconButtonVariant` and `on_click(impl Fn())`; `ExpansionPanels` is a collapsible
  `panel(title, impl Fn() -> AnyView)` list with `multiple(bool)` + `on_change(impl Fn(usize, bool))`;
  `SpeedDial` is a FAB + action cluster (`action(icon, Option<String>)` + `direction(SpeedDialDirection)`
  + `fab_icon(String)` + `open(bool)` + `on_action(impl Fn(usize))`); `ChipGroup` is a single/multi
  selectable chip row (`chip(String)` + `multiple(bool)` + `mandatory(bool)` + `on_change(Vec<usize>)`);
  `ListGroup` is a collapsible item list with a header (`item(String)` + `title(String)` +
  `expanded(bool)` + `inset(bool)` + `on_toggle(impl Fn(bool))`); `Counter` is a numeric stepper
  (`value(i64)` + `min(i64)` + `max(i64)` + `step(i64)` + `on_change(impl Fn(i64))`); `Responsive` is a
  fixed aspect-ratio container (`new(f32)` + `child(impl Fn() -> AnyView)` closure factory);
  `CheckboxBtn` is a button-style checkbox (`checked(bool)` + `label(String)` + `disabled(bool)` +
  `on_change(impl Fn(bool))`, keyboard Enter/Space toggle); `ListSubheader` is a section divider label
  (`new(String)` + `inset(bool)`). The library now ships **95 components × 3 design systems = 285
  crates**.

- Batch 20 (NumberInput / PullToRefresh / ConfirmEdit) added 2026-08-12 — the three remaining
  distinct Vuetify components (`VNumberInput`, `VPullToRefresh`, `VConfirmEdit`) that were not
  covered by the earlier 95. Every one implemented in all three design systems with the same
  neutral builder API, following the standard foundation → scaffold (9 crates) → Material source
  → swap-to-Cupertino/Fluent → floom-elements facade → demo → verify pipeline. `NumberInput` is a
  numeric field with `−` / `+` stepper buttons and a clamped `TextInput` (raw `i64` values via
  `value/min/max/step`, typed text is parsed back to a clamped `i64` and fires `on_change` — same
  shape as `Counter`, but editable-typed); `PullToRefresh` is a downward-drag wrapper that detects a
  pull past `threshold` and fires `on_refresh` (component-owned `refreshing` flag, host clears via
  `refreshing_signal()` — there is no built-in timer in this Floom build); `ConfirmEdit` is a
  click-to-edit inline wrapper that shows `child` (or the `value` text) in display mode and swaps in
  a `TextInput` + ✓/✗ on edit (`on_save` fires with the edited string). The library now ships **98
  components × 3 design systems = 294 crates**.

| Contract component | Material | Cupertino | Fluent | Current capability |
| --- | --- | --- | --- | --- |
| Button | `md-button` | `cupertino-button` | `fluent-button` | ✅ `view()` + interactive (clicks, disabled) |
| Text | `md-text` | `cupertino-text` | `fluent-text` | ✅ `view()` |
| TextField | `md-text-field` | `cupertino-text-field` | `fluent-text-field` | ✅ `view()` + live `on_change` |
| Checkbox | `md-checkbox` | `cupertino-checkbox` | `fluent-checkbox` | ✅ `view()` + toggle |
| Switch | `md-switch` | `cupertino-switch` | `fluent-switch` | ✅ `view()` + toggle |
| Dialog | `md-dialog` | `cupertino-dialog` | `fluent-dialog` | ✅ `view()` + scrim + `open_signal()` |
| Menu | `md-menu` | `cupertino-menu` | `fluent-menu` | ✅ `view()` + items + `open_signal()` |
| Card | `md-card` | `cupertino-card` | `fluent-card` | ✅ `view()` + `child()` content |
| Icon | `md-icon` | `cupertino-icon` | `fluent-icon` | ✅ `view()` |
| Divider | `md-divider` | `cupertino-divider` | `fluent-divider` | ✅ `view()` |
| Slider | `md-slider` | `cupertino-slider` | `fluent-slider` | ✅ `view()` + `value/min/max/step/on_change` |
| RadioGroup | `md-radio-group` | `cupertino-radio-group` | `fluent-radio-group` | ✅ `view()` + `options/selected/on_change` |
| Tooltip | `md-tooltip` | `cupertino-tooltip` | `fluent-tooltip` | ✅ `view()` + `text().child()` hover |
| Tabs | `md-tabs` | `cupertino-tabs` | `fluent-tabs` | ✅ `view()` + `tabs/selected/on_change` |
| List | `md-list` | `cupertino-list` | `fluent-list` | ✅ `view()` + `item()` rows |
| Select | `md-select` | `cupertino-select` | `fluent-select` | ✅ `view()` + `options/selected/on_change` dropdown |
| Textarea | `md-textarea` | `cupertino-textarea` | `fluent-textarea` | ✅ `view()` + `value/rows/on_change` |
| Snackbar | `md-snackbar` | `cupertino-snackbar` | `fluent-snackbar` | ✅ `view()` + `message/action/open_signal` auto-dismiss |
| Chip | `md-chip` | `cupertino-chip` | `fluent-chip` | ✅ `view()` + `label/selected/on_click` |
| Badge | `md-badge` | `cupertino-badge` | `fluent-badge` | ✅ `view()` + `child()/content/dot` |
| AppBar | `md-app_bar` | `cupertino-app_bar` | `fluent-app_bar` | ✅ `view()` + `title/leading/actions` |
| NavigationDrawer | `md-navigation_drawer` | `cupertino-navigation_drawer` | `fluent-navigation_drawer` | ✅ `view()` + `header/child/open_signal` scrim |
| BottomNavigation | `md-bottom_navigation` | `cupertino-bottom_navigation` | `fluent-bottom_navigation` | ✅ `view()` + `labels/selected/on_change` |
| ProgressLinear | `md-progress_linear` | `cupertino-progress_linear` | `fluent-progress_linear` | ✅ `view()` + `value/indeterminate` |
| ProgressCircular | `md-progress_circular` | `cupertino-progress_circular` | `fluent-progress_circular` | ✅ `view()` + `value/indeterminate` (SVG) |
| Avatar | `md-avatar` | `cupertino-avatar` | `fluent-avatar` | ✅ `view()` + `content/size` (circular) |
| Alert | `md-alert` | `cupertino-alert` | `fluent-alert` | ✅ `view()` + `variant/title/message/dismissible/on_dismiss` |
| Rating | `md-rating` | `cupertino-rating` | `fluent-rating` | ✅ `view()` + `value/max/readonly/on_change` interactive |
| Pagination | `md-pagination` | `cupertino-pagination` | `fluent-pagination` | ✅ `view()` + `total/current/on_change` interactive |
| Timeline | `md-timeline` | `cupertino-timeline` | `fluent-timeline` | ✅ `view()` + `item()/items()` vertical connector |
| Image | `md-image` | `cupertino-image` | `fluent-image` | ✅ `view()` + `src/alt/width/height/rounded` (local file bytes; placeholder fallback) |
| Banner | `md-banner` | `cupertino-banner` | `fluent-banner` | ✅ `view()` + `variant/message/icon/action/on_action` tinted bar |
| SkeletonLoader | `md-skeleton_loader` | `cupertino-skeleton_loader` | `fluent-skeleton_loader` | ✅ `view()` + `skeleton_type/lines/width/height` placeholders |
| Toolbar | `md-toolbar` | `cupertino-toolbar` | `fluent-toolbar` | ✅ `view()` + `title/leading/actions/elevation` |
| Sheet | `md-sheet` | `cupertino-sheet` | `fluent-sheet` | ✅ `view()` + `side/open/elevated/child/open_signal` overlay |
| Container | `md-container` | `cupertino-container` | `fluent-container` | ✅ `view()` + `max_width/padding/fluid/child` centered capped-width box |
| Grid | `md-grid` | `cupertino-grid` | `fluent-grid` | ✅ `view()` + `cols/gap/row_gap/child` (repeatable) equal-width rows |
| Spacer | `md-spacer` | `cupertino-spacer` | `fluent-spacer` | ✅ `view()` + `grow(f32)` flex-growing empty view |
| Footer | `md-footer` | `cupertino-footer` | `fluent-footer` | ✅ `view()` + `height/elevation/bordered/pad/child` bottom bar |
| SystemBar | `md-system_bar` | `cupertino-system_bar` | `fluent-system_bar` | ✅ `view()` + `height/dark/pad/child` slim top status strip |
| Combobox | `md-combobox` | `cupertino-combobox` | `fluent-combobox` | ✅ `view()` + `options/selected/placeholder/disabled/on_change` searchable dropdown |
| Autocomplete | `md-autocomplete` | `cupertino-autocomplete` | `fluent-autocomplete` | ✅ `view()` + `options/selected/placeholder/disabled/on_change` free-text commit on blur |
| DatePicker | `md-date_picker` | `cupertino-date_picker` | `fluent-date_picker` | ✅ `view()` + `value/placeholder/disabled/on_change` reactive month-grid popover |
| TimePicker | `md-time_picker` | `cupertino-time_picker` | `fluent-time_picker` | ✅ `view()` + `value/placeholder/disabled/on_change` hour/minute stepper popover |
| EmptyState | `md-empty_state` | `cupertino-empty_state` | `fluent-empty_state` | ✅ `view()` + `icon/title/description/child` centered placeholder |
| DataTable | `md-datatable` | `cupertino-datatable` | `fluent-datatable` | ✅ `view()` + `columns/rows/striped/dense/on_row_click` aligned header + body |
| Treeview | `md-treeview` | `cupertino-treeview` | `fluent-treeview` | ✅ `view()` + `nodes(Vec<TreeviewNode>)/on_node_click` recursive collapsible |
| Calendar | `md-calendar` | `cupertino-calendar` | `fluent-calendar` | ✅ `view()` + `value/show_week_numbers/on_change` 6×7 month grid |
| Carousel | `md-carousel` | `cupertino-carousel` | `fluent-carousel` | ✅ `view()` + `slides(Vec<AnyView>)/auto_play/show_arrows/show_dots/on_change` |
| VirtualScroller | `md-virtual_scroller` | `cupertino-virtual_scroller` | `fluent-virtual_scroller` | ✅ `view()` + `items(Vec<Box<dyn Fn()->Box<dyn View>>>)/height/on_reach_end` windowed |
| App | `md-app` | `cupertino-app` | `fluent-app` | ✅ `view()` + `child(impl IntoView)` full-viewport frame; initializes the layout signal |
| Main | `md-main` | `cupertino-main` | `fluent-main` | ✅ `view()` + `child(impl IntoView)` content region with reactive auto-insets from the layout signal |
| Form | `md-form` | `cupertino-form` | `fluent-form` | ✅ `view()` + `child(impl IntoView)` + `submit_label/show_submit/on_submit` + `valid_signal()`/`errors_signal()` validity-gated submit |
| ValidationMessages | `md-validation_messages` | `cupertino-validation_messages` | `fluent-validation_messages` | ✅ `view()` + `messages_signal(RwSignal<Vec<String>>)` reactive error list in theme `error` colour (the 校验提示 slot) |
| Stepper | `md-stepper` | `cupertino-stepper` | `fluent-stepper` | ✅ `view()` + `items(Vec<String>)/selected(usize)/editable(bool)/non_linear(bool)/on_change(impl Fn(usize))` internal `RwSignal<usize>`; completed steps show ✓, connectors fill when passed |
| Breadcrumbs | `md-breadcrumbs` | `cupertino-breadcrumbs` | `fluent-breadcrumbs` | ✅ `view()` + `items(Vec<String>)/on_click(impl Fn(usize))` `>`-separated trail; last item current/non-clickable |
| Window | `md-window` | `cupertino-window` | `fluent-window` | ✅ `view()` + `items(Vec<AnyView>)/height/selected/show_arrows/show_dots/continuous/on_change(impl Fn(usize))` carousel slide stack |
| SlideGroup | `md-slide_group` | `cupertino-slide_group` | `fluent-slide_group` | ✅ `view()` + `items(Vec<AnyView>)/selected/show_arrows/mandatory/on_change(impl Fn(usize))` scrollable strip with active indicator |
| ItemGroup | `md-item_group` | `cupertino-item_group` | `fluent-item_group` | ✅ `view()` + `items(Vec<AnyView>)/selected/multiple/mandatory/on_change` + `selected_signal()`/`selected_multi_signal()` |
| ColorPicker | `md-color_picker` | `cupertino-color_picker` | `fluent-color_picker` | ✅ `view()` + `value(Color)/show_alpha/on_change(Color)` + `color_signal()` HSV SV-square + hue strip + hex input |
| ThemeProvider | `md-theme_provider` | `cupertino-theme_provider` | `fluent-theme_provider` | ✅ service layer — `child(impl IntoView)` + `dark(bool)` + `dark_signal() -> RwSignal<bool>` (no visual tokens) |
| LocaleProvider | `md-locale_provider` | `cupertino-locale_provider` | `fluent-locale_provider` | ✅ service layer — `child(impl IntoView)` + `locale(String)` + `locale_signal() -> RwSignal<String>` |
| DisplayProvider | `md-display_provider` | `cupertino-display_provider` | `fluent-display_provider` | ✅ service layer — `child(impl IntoView)` + `dense(bool)` + `density_signal() -> RwSignal<bool>` |
| DefaultsProvider | `md-defaults_provider` | `cupertino-defaults_provider` | `fluent-defaults_provider` | ✅ service layer — `child(impl IntoView)` + `defaults(HashMap<String,String>)` + `defaults_signal()` |
| IconsProvider | `md-icons_provider` | `cupertino-icons_provider` | `fluent-icons_provider` | ✅ service layer — `child(impl IntoView)` + `icons(HashMap<String,String>)` + `icons_signal()` |
| DateAdapterProvider | `md-date_adapter_provider` | `cupertino-date_adapter_provider` | `fluent-date_adapter_provider` | ✅ service layer — `child(impl IntoView)` + `adapter(String)` + `adapter_signal() -> RwSignal<String>` |
| ButtonGroup | `md-button_group` | `cupertino-button_group` | `fluent-button_group` | ✅ `view()` + `joined`/`vertical` segmented layout of child views |
| FloatingActionButton | `md-floating_action_button` | `cupertino-floating_action_button` | `fluent-floating_action_button` | ✅ `view()` + circular/elevated (or extended) primary action |
| RangeSlider | `md-range_slider` | `cupertino-range_slider` | `fluent-range_slider` | ✅ `view()` + `values`/`on_change((f32, f32))` two-thumb track (guarantees low ≤ high) |
| FileInput | `md-file_input` | `cupertino-file_input` | `fluent-file_input` | ✅ `view()` + trigger + path readout; `on_change(Option<String>)` on click, writes back via `path_signal()` (no OS dialog) |
| OTPInput | `md-otp_input` | `cupertino-otp_input` | `fluent-otp_input` | ✅ `view()` + `length` single-char `TextInput` boxes bound to a master `code` string; optional `masked` (•) |
| ListItem | `md-list_item` | `cupertino-list_item` | `fluent-list_item` | ✅ `view()` + `title(String)/subtitle(Option<String>)/leading(AnyView)/trailing(AnyView)/nav(bool)/disabled(bool)/on_click(impl Fn())` clickable row; `Container.action()` handler guarded by `disabled` |
| Table | `md-table` | `cupertino-table` | `fluent-table` | ✅ `view()` + `headers(Vec<String>)/rows(Vec<Vec<String>>)/dense(bool)/striped(bool)` simple bordered body (no `AnyView`, `Clone`-free); odd rows tinted `surface_variant` when `striped` |
| DataIterator | `md-data_iterator` | `cupertino-data_iterator` | `fluent-data_iterator` | ✅ `view()` + `item(AnyView)/header(AnyView)/footer(AnyView)/divided(bool)` stack with 1px `divided` separators between items |
| Sparkline | `md-sparkline` | `cupertino-sparkline` | `fluent-sparkline` | ✅ `view()` + `values(Vec<f32>)/width(f32)/height(f32)/fill(bool)/line_width(f32)` theme-coloured vertical bars (no SVG); `fill` uses `with_alpha(primary, 90)` |
| Kbd | `md-kbd` | `cupertino-kbd` | `fluent-kbd` | ✅ `view()` + `content(String)/child(AnyView)/mono(bool)` monospace inline keyboard-shortcut mark on `surface_variant` bg; rounded |
| Code | `md-code` | `cupertino-code` | `fluent-code` | ✅ `view()` + `content(String)/child(AnyView)/mono(bool)` monospace inline code snippet mark on `surface_variant` bg |
| Radio | `md-radio` | `cupertino-radio` | `fluent-radio` | ✅ `view()` + `label(String)/value(String)/checked(bool)/disabled(bool)/on_change(impl Fn(bool))` independent toggle radio (internal `RwSignal<bool>`; host handles mutual exclusion); keyboard Enter/Space toggles |
| Overlay | `md-overlay` | `cupertino-overlay` | `fluent-overlay` | ✅ `view()` + `show(bool)/child(impl IntoView)/on_click(impl Fn())/opacity(f32)` scrim overlay via `Overlay::new(scrim)`; `show_signal() -> RwSignal<bool>` (mirrors `open_signal()`); scrim tap auto-closes + fires `on_click`; content layer stops propagation |
| Parallax | `md-parallax` | `cupertino-parallax` | `fluent-parallax` | ✅ `view()` + `child(impl IntoView)/factor(f32)/scroll_signal(RwSignal<f32>)` host-scroll-driven translate; `view()` offsets child by `-scroll.get() * factor` (host owns the offset signal) |
| InfiniteScroller | `md-infinite_scroller` | `cupertino-infinite_scroller` | `fluent-infinite_scroller` | ✅ `view()` + `child(impl IntoView)/content_height_signal(RwSignal<f32>)/viewport(f32)/threshold(f32)/on_load_more(impl Fn())/loading_signal() -> RwSignal<bool>` wheel-driven windowed viewport (PointerWheel + ScrollDelta + Overflow::Clip); fires `on_load_more` within `threshold` of end; owns `loading_signal()` |
| Hover | `md-hover` | `cupertino-hover` | `fluent-hover` | ✅ `view()` + `child(impl IntoView)/on_hover_change(impl Fn(bool))/hovered_signal() -> RwSignal<bool>` PointerEnter/Leave tracked `RwSignal<bool>`; tints bg with `with_alpha(on_surface, highlight_alpha)` when hovered |
| FadeTransition | `md-fade_transition` | `cupertino-fade_transition` | `fluent-fade_transition` | ✅ `view()` + `child(impl Fn()->impl IntoView)/show_signal() -> RwSignal<bool>` opacity 0↔1 enter/leave animation (Floom keyframe 0→100 on mount, reverse on unmount) |
| ScaleTransition | `md-scale_transition` | `cupertino-scale_transition` | `fluent-scale_transition` | ✅ `view()` + `child()/show_signal()` scale `min_scale`↔1.0 enter/leave animation |
| ExpandTransition | `md-expand_transition` | `cupertino-expand_transition` | `fluent-expand_transition` | ✅ `view()` + `child()/show_signal()` height 0↔natural (clipped) enter/leave animation |
| SlideTransition | `md-slide_transition` | `cupertino-slide_transition` | `fluent-slide_transition` | ✅ `view()` + `child()/direction(SlideDirection)/show_signal()` translate `offset`→0 on X/Y enter/leave animation |
| IconButton | `md-icon_button` | `cupertino-icon_button` | `fluent-icon_button` | ✅ `view()` + `new(icon: impl Into<String>)/size(f32)/variant(IconButtonVariant)/disabled(bool)/on_click(impl Fn())` renders via `svg()` (SVG string) |
| ExpansionPanels | `md-expansion_panels` | `cupertino-expansion_panels` | `fluent-expansion_panels` | ✅ `view()` + `panel(title, impl Fn()->AnyView)/multiple(bool)/on_change(impl Fn(usize,bool))` collapsible panel list (▸/▾ glyph) |
| SpeedDial | `md-speed_dial` | `cupertino-speed_dial` | `fluent-speed_dial` | ✅ `view()` + `action(icon, Option<String>)/direction(SpeedDialDirection)/fab_icon(String)/open(bool)/disabled(bool)/on_action(impl Fn(usize))` FAB + action cluster |
| ChipGroup | `md-chip_group` | `cupertino-chip_group` | `fluent-chip_group` | ✅ `view()` + `chip(String)/multiple(bool)/mandatory(bool)/on_change(impl Fn(Vec<usize>))` selectable chip row |
| ListGroup | `md-list_group` | `cupertino-list_group` | `fluent-list_group` | ✅ `view()` + `item(String)/title(String)/expanded(bool)/inset(bool)/on_toggle(impl Fn(bool))` collapsible item list with header |
| Counter | `md-counter` | `cupertino-counter` | `fluent-counter` | ✅ `view()` + `value(i64)/min(i64)/max(i64)/step(i64)/on_change(impl Fn(i64))` numeric stepper (− / value / +) |
| Responsive | `md-responsive` | `cupertino-responsive` | `fluent-responsive` | ✅ `view()` + `new(aspect_ratio: f32)/child(impl Fn()->AnyView)` fixed aspect-ratio (width_full) container |
| CheckboxBtn | `md-checkbox_btn` | `cupertino-checkbox_btn` | `fluent-checkbox_btn` | ✅ `view()` + `checked(bool)/disabled(bool)/label(String)/on_change(impl Fn(bool))` button-style checkbox, keyboard Enter/Space toggle |
| ListSubheader | `md-list_subheader` | `cupertino-list_subheader` | `fluent-list_subheader` | ✅ `view()` + `new(text: impl Into<String>)/inset(bool)` section divider label |
| NumberInput | `md-number_input` | `cupertino-number_input` | `fluent-number_input` | ✅ `view()` + `value(i64)/min(i64)/max(i64)/step(i64)/label(impl Into<String>)/disabled(bool)/on_change(impl Fn(i64))` numeric field with − / + stepper buttons (clamped) |
| PullToRefresh | `md-pull_to_refresh` | `cupertino-pull_to_refresh` | `fluent-pull_to_refresh` | ✅ `view()` + `child(impl Fn()->AnyView)/loading(bool)/threshold(f32)/on_refresh(impl Fn())/refreshing_signal()->RwSignal<bool>` downward-drag wrapper; fires on_refresh past threshold |
| ConfirmEdit | `md-confirm_edit` | `cupertino-confirm_edit` | `fluent-confirm_edit` | ✅ `view()` + `child(impl Fn()->AnyView)/value(impl Into<String>)/on_save(impl Fn(String))` click-to-edit inline text with ✓/✗ |

### Cross-design-system API contract (must hold for all three)

The gallery uses a design-system-neutral API, and it is now the thing that
enforces the contract: a sample is written once and instantiated three times,
once per design system, so a builder one of them spells differently fails to
compile instead of quietly leaving a column empty. Every implementation must
satisfy it:

- `Card::child(impl IntoView)` — sets the card's content (`Option<AnyView>`).
- `Dialog::open_signal() -> RwSignal<bool>` / `Menu::open_signal() -> RwSignal<bool>`
  — called on `&self` before `view(self)`, so an external trigger (button) can show/hide
  the overlay. The overlay must read the signal **reactively** (`open.get()` inside a
  `move` style closure), not capture a plain `bool`.
- Overlays render above the page via `Overlay::new(...)`.
- `AppBar::title(String)` + `AppBar::leading(impl IntoView)` + `AppBar::actions(Vec<AnyView>)`
  — builds a top bar; `actions` takes pre-converted `AnyView`s (call `.into_any()` on each).
- `NavigationDrawer::open_signal() -> RwSignal<bool>` — same external-trigger contract as
  Dialog/Menu; `header(impl IntoView)` + `child(impl IntoView)` set the panel content; the
  scrim closes it reactively.
- `BottomNavigation::labels(Vec<String>)` + `selected(usize)` + `on_change(impl Fn(usize))`
  (+ optional `icons(Vec<AnyView>)`) — selected index is held in an internal `RwSignal<usize>`
  so taps update it and fire the callback.
- `ProgressLinear`/`ProgressCircular` take `value(f32)` (0..100) + `indeterminate(bool)`.
  Circular is drawn with Floom SVG (`stroke-dasharray`/`stroke-dashoffset`); determinate
  indeterminate fallback is a static partial segment (this Floom build has no
  `request_anim_frame`).

### Batch 5 API contract (Avatar / Alert / Rating / Pagination / Timeline)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Avatar::content(impl Into<String>)` + `Avatar::size(f32)` — circular avatar; Cupertino
  uses a neutral `surface_variant` fill, Material/Fluent use the `primary_container` role.
- `Alert::variant(AlertVariant)` (`Info`/`Success`/`Warning`/`Error`) +
  `Alert::title(String)` + `Alert::message(String)` + `Alert::dismissible(bool)` +
  `Alert::on_dismiss(impl Fn())` — tinted banner. Semantic accents: Material/Cupertino/Fluent
  reuse the theme `error` role for `Error` and derive `Info`/`Success`/`Warning` from their
  respective brand system palettes (Material has no built-in semantic tokens).
- `Rating::value(f32)` + `Rating::max(usize)` + `Rating::readonly(bool)` +
  `Rating::on_change(impl Fn(f32))` — interactive stars held in an internal `RwSignal<f32>`
  so taps update the fill and fire the callback. Cupertino fills with system yellow.
- `Pagination::total(usize)` + `Pagination::current(usize)` +
  `Pagination::on_change(impl Fn(usize))` — prev / numbered / next controls; disabled prev on
  page 1 and next on last page (guarded inside the click handler, not via `apply_if` on the
  view).
- `Timeline::item(title, description)` (builder, repeatable) or `Timeline::items(Vec<TimelineItem>)`
  — vertical list with dot markers joined by a connector line (last item omits the line).

### Batch 6 API contract (Image / Banner / SkeletonLoader / Toolbar / Sheet)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Image::src(String)` + `Image::alt(String)` (+ optional `width(f32)` / `height(f32)` /
  `rounded(bool)`) — renders raster bytes via `img(move || bytes.clone())`. Local file paths
  are read with `std::fs::read`; `http(s)://` URLs and missing files fall back to a
  `text(alt)` placeholder (this build has no network loader). `rounded` applies a corner
  radius; `width`/`height` are applied only when `Some`.
- `Banner::variant(BannerVariant)` (`Info`/`Warning`/`Error`/`Success`) +
  `Banner::message(String)` + `Banner::icon(Option<String>)` +
  `Banner::action_label(Option<String>)` + `Banner::on_action(impl Fn())` +
  `Banner::single_line(bool)` — tinted bar. Semantic accent: `Error` reuses the theme `error`
  role; `Info`/`Success`/`Warning` derive from each brand's palette (Material
  rgb(33,150,243)/rgb(76,175,80)/rgb(255,152,0), Cupertino rgb(0,122,255)/rgb(52,199,89)/
  rgb(255,149,0), Fluent rgb(0,120,212)/rgb(16,124,16)/rgb(247,206,0)). The action button is
  wired through `Rc<Option<Box<dyn Fn()>>>` so the per-design-system `view()` can clone it.
- `SkeletonLoader::skeleton_type(SkeletonType)` (`Text`/`Circle`/`Rect`/`Avatar`/`Card`) +
  `SkeletonLoader::lines(usize)` + `SkeletonLoader::width(Option<f32>)` +
  `SkeletonLoader::height(Option<f32>)` + `SkeletonLoader::animated(bool)` — static loading
  placeholders via `Container::new(empty())` + `list(Vec<AnyView>)` for `Text`/`Avatar` rows;
  `Circle`/`Avatar` use `border_radius(9999.0)`. `animated` is accepted for parity but this
  Floom build has no `request_anim_frame`, so it is a no-op (documented in the crate).
- `Toolbar::title(String)` + `Toolbar::leading(Option<String>)` + `Toolbar::actions(impl
  IntoView)` + `Toolbar::elevation(u8)` + `Toolbar::height(f32)` — a top bar that mirrors
  `AppBar` but is designated for in-context tooling; leading icon + title + actions slot +
  elevation shadow (`FontWeight::Medium.to_floom()` for the title).
- `Sheet::side(SheetSide)` (`Bottom`/`Top`/`Start`/`End`) + `Sheet::open(bool)` +
  `Sheet::elevated(bool)` + `Sheet::child(impl IntoView)` +
  `Sheet::open_signal() -> RwSignal<bool>` — a docking surface rendered through
  `Overlay::new(scrim)` using a flex stack (`flex_col`/`flex_row`) anchored to the requested
  edge via `justify_end`/`justify_start` (no absolute positioning — `left/right/top/bottom`
  px are unavailable in this Floom build). Corner radius is applied on the docking edge
  only. `sheet.apply_if(!open.get(), |s| s.hide())` hides it reactively when closed; the
  stored `RwSignal<bool>` (set in `new()`/`open()`) is what `open_signal()` returns, so an
  external button can toggle it like `Dialog`/`Menu`/`NavigationDrawer`.

### Batch 7 API contract (Container / Grid / Spacer / Footer / SystemBar)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Container::max_width(f32)` + `Container::padding(f32)` + `Container::fluid(bool)` +
  `Container::child(impl IntoView)` — a centered content box. `max_width` caps the inner
  width (default 1200.0 dp); `fluid(true)` disables the cap so it fills width; `padding`
  (default 16.0 dp) pads the inner box. Implemented as an outer `Container` with
  `width_pct(100.0)` + `justify_center` wrapping an inner `Container` that applies the
  surface background, padding, and (unless `fluid`) `max_width`.
- `Grid::cols(usize)` + `Grid::gap(f32)` + `Grid::row_gap(f32)` + `Grid::child(impl IntoView)`
  (repeatable) — lays children into `cols` equal-width cells per row. `view()` chunks the
  `Vec<AnyView>` into rows of `cols`, pads a short last row with `empty()`, wraps each row in
  `h_stack_from_iter` with each cell `Container::new(c).style(flex_grow(1.0))`, then stacks the
  rows via `list(rows).style(gap(row_gap))`. No CSS grid exists in this Floom build, so rows
  are built explicitly.
- `Spacer::grow(f32)` (default 1.0) — a flex-growing empty view: `Container::new(empty())`
  with `flex_grow(grow)`. Use it inside an `h_stack`/`v_stack` to push siblings apart.
- `Footer::height(f32)` (default 56.0) + `Footer::elevation(u8)` (default 0) +
  `Footer::bordered(bool)` (default true) + `Footer::pad(f32)` (default 16.0) +
  `Footer::child(impl IntoView)` — a bottom content bar. Applies the surface background; when
  `bordered`, a 1px `border_top` using `outline_variant`; when `elevation > 0`, a top
  `box_shadow` for lift.
- `SystemBar::height(f32)` (default 24.0) + `SystemBar::dark(bool)` (default false) +
  `SystemBar::pad(f32)` (default 8.0) + `SystemBar::child(impl IntoView)` — a slim top status
  strip. Light variant uses `surface_variant` bg + `on_surface_variant` content; dark variant
  uses `scrim` bg + `on_surface` content.

### Batch 8 API contract (Combobox / Autocomplete / DatePicker / TimePicker / EmptyState)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Combobox::options(Vec<String>)` + `Combobox::selected(Option<usize>)` +
  `Combobox::placeholder(String)` + `Combobox::disabled(bool)` +
  `Combobox::on_change(impl Fn(Option<usize>))` — a searchable single-select dropdown. The
  trigger is a `TextInput` (query) + chevron; typing filters rows by case-insensitive
  `contains`; clicking a row sets `selected`, fills the query, closes the panel, and fires
  `on_change`. The dropdown is an absolute `Container` below the trigger
  (`inset_top_pct(100.0)` + `inset_left_pct(0.0)`), shown/hidden reactively via
  `apply_if(!open.get(), |s| s.hide())`. Rows reuse `opt`/`opt_style`/`opt_event` clones so
  `text(opt)` + the style matcher + the click handler each own a copy (a single `opt` cannot
  be moved into three consumers).
- `Autocomplete::options(Vec<String>)` + `Autocomplete::selected(Option<usize>)` +
  `Autocomplete::placeholder(String)` + `Autocomplete::disabled(bool)` +
  `Autocomplete::on_change(impl Fn(Option<String>))` — same shell as `Combobox`, but on
  `FocusLost` it commits the typed free text: empty query → `cb(None)`; no exact
  case-insensitive match → `cb(Some(query.clone()))`; otherwise the current selection is kept.
  The `FocusLost` handler must be an **inline block closure** (`move |_, _| { … }` passed
  directly to `.on_event`), not a named `let` binding — storing it in a variable triggers
  "implementation of `FnMut` is not general enough" because the inferred closure type is not
  HRTB-general. Row clicks call `cb(Some(opt_event.clone()))` (the dedicated clone, not the
  `opt` consumed by `text(opt)`).
- `DatePicker::value(DateValue)` + `DatePicker::placeholder(String)` +
  `DatePicker::disabled(bool)` + `DatePicker::on_change(impl Fn(DateValue))` —
  `DateValue { year: i32, month: u32, day: u32 }` (`Default` = 2026-08-10). The panel is a
  6×7 grid of **42 fixed `label` cells** (not a signal-of-views): each cell reads
  `view_month: RwSignal<(i32, u32)>` reactively via pure helpers `is_leap` / `days_in_month` /
  `weekday_of_first` / `day_at`, so prev/next navigation updates labels, highlight, and the
  click target without rebuilding the tree. Leading/trailing blank cells are hidden with
  `apply_if(!vis, |s| s.hide())` on the cell's own style. **Key Floom constraint:
  `RwSignal<AnyView>` is not `IntoView` (AnyView is not `Clone`), so `h_stack_from_iter` must
  consume an owned `Vec<AnyView>` built per row — do NOT call `.chunks(7).map(|c|
  c.to_vec())`, since `to_vec` requires `AnyView: Clone`.**
- `TimePicker::value(TimeValue)` + `TimePicker::placeholder(String)` +
  `TimePicker::disabled(bool)` + `TimePicker::on_change(impl Fn(TimeValue))` —
  `TimeValue { hour: u8, minute: u8 }`. The panel has hour and minute steppers (`−` / `+`
  buttons that wrap at 24 / 60) plus a Done button; the trigger shows `HH:MM`. The stepper
  rows use `.style(move |s| …)` so the non-`Copy` `cell_height` is captured by move rather
  than borrowed (a bare `.style(|s| … cell_height)` would fail with "closure may outlive
  borrowed value").
- `EmptyState::icon(String)` + `EmptyState::title(String)` + `EmptyState::description(String)`
  + `EmptyState::child(impl IntoView)` — a centered `list` of `[icon, title, description,
  child]` inside a `Container`. `title`/`description` styling uses `FontWeight::Medium` (import
  `FontWeight` from `floom_core_theme::tokens::typography` — it is **not** in the floom
  prelude).

### Known parity fix applied (2026-08-10)

- Cupertino `Card`/`Dialog`/`Menu` were missing `child()` / `open_signal()` — added to
  match Material (Task #4 demo-build blockers).
- Fluent `Card`/`Dialog`/`Menu` had the same gaps **plus** a reactive bug: `fluent-menu`
  captured `open` as a plain `bool`, so `open_signal().set(true)` had no effect. Fixed to
  a stored `RwSignal<bool>` with reactive `apply_if(!open.get(), …)` and close-on-select,
  matching Material/Cupertino.

## `view()` status (reconciled 2026-11-11)

All 98 components implement a real, renderable, interactive `view()` in **all three**
design systems. They are exercised headlessly by the satellite test suite, and by the
gallery, which mounts every one of them:

| Design system | How it is verified today | Status |
| --- | --- | --- |
| Material 3 | `cargo test --workspace` in `crates/ecosystem/floom-ui` | ✅ headless tests |
| Cupertino | same | ✅ headless tests |
| Fluent | same | ✅ headless tests |
| Harmony | consolidated `floom-harmony` gallery tests | ✅ headless tests (no kit crates) |

> A runnable gallery window **is** present — `examples/floom-demo`, in the root workspace
> (`cargo run -p floom-demo`; `just demo` from the satellite). It shows all 98 components
> in Material, Cupertino and Fluent side by side, and `cargo test -p floom-demo` mounts
> every page and every sample headlessly. The old "✅ compiles + runs" rows referred to the
> v1 demo crate, which has since been removed.

Components with working `view()` in all systems:
`button`, `text`, `text-field`, `checkbox`, `switch`, `dialog`, `menu`, `card`, `icon`,
`divider`, `slider`, `radio-group`, `tooltip`, `tabs`, `list`, `select`, `textarea`,
`snackbar`, `chip`, `badge`, `app-bar`, `navigation-drawer`, `bottom-navigation`,
`progress-linear`, `progress-circular`, `avatar`, `alert`, `rating`, `pagination`,
`timeline`, `image`, `banner`, `skeleton-loader`, `toolbar`, `sheet`, `container`,
`grid`, `spacer`, `footer`, `system-bar`, `combobox`, `autocomplete`, `date-picker`,
`time-picker`, `empty-state`, `app`, `main`, `form`, `validation-messages`, `stepper`, `breadcrumbs`,
`parallax`, `infinite-scroller`, `hover`, `fade-transition`, `scale-transition`, `expand-transition`, `slide-transition`,
`icon-button`, `expansion-panels`, `speed-dial`, `chip-group`, `list-group`, `counter`,
`responsive`, `checkbox-btn`, `list-subheader`, `number-input`, `pull-to-refresh`,
`confirm-edit`.

The `app`/`main` shell uses a DS-agnostic layout provider (`floom-core-theme::layout`):
`App` initializes the process-wide `layout_signal` and `Main` auto-insets via reactive
spacers driven by the 5 shell primitives auto-registering their sizes.

Because `CupertinoTheme`/`FluentTheme` reuse the same `MaterialComponentTokens` +
`ComponentTokens` enum, the per-design-system `view()` follows the identical
token-extraction pattern as Material — only the visual styling differs.

## Vuetify-aligned target inventory

The long-term target should track the Vuetify component surface area, grouped by capability so implementation can be staged without losing parity.

### Application and layout

- App ✅
- Main ✅
- Container ✅
- Grid / Row / Col ✅ (Grid only; Row/Col not separate)
- Spacer ✅
- Divider ✅
- Sheet ✅
- Footer ✅
- SystemBar ✅
- Toolbar ✅
- AppBar ✅
- BottomNavigation ✅
- NavigationDrawer ✅
- Layout item registration / layout provider ✅
- Responsive ✅

### Actions and controls

- Button ✅
- ButtonGroup ✅
- FloatingActionButton ✅
- Checkbox ✅
- Radio ✅
- RadioGroup ✅
- Switch ✅
- Slider ✅
- RangeSlider ✅
- Rating ✅
- FileInput ✅
- OTPInput ✅
- IconButton ✅
- CheckboxBtn ✅
- ChipGroup ✅
- Counter ✅
- SpeedDial ✅

### Text entry and selection

- TextField ✅
- Textarea ✅
- Select ✅
- Combobox ✅
- Autocomplete ✅
- Form ✅
- Input validation messages ✅
- NumberInput ✅
- ConfirmEdit ✅

### Data display

- Text ✅
- Icon ✅
- Avatar ✅
- Badge ✅
- Chip ✅
- List ✅
- ListItem ✅
- Card ✅
- Table ✅
- DataTable ✅
- DataIterator ✅
- Treeview ✅
- Timeline ✅
- Calendar ✅
- Sparkline ✅
- Kbd ✅
- Code ✅
- ListGroup ✅
- ListSubheader ✅

### Navigation

- Breadcrumbs ✅
- Tabs ✅
- Window ✅
- Stepper ✅
- Pagination ✅
- SlideGroup ✅
- ItemGroup ✅

### Feedback and overlays

- Alert ✅
- Banner ✅
- Dialog ✅
- Menu ✅
- Tooltip ✅
- Snackbar ✅
- Overlay ✅
- ProgressCircular ✅
- ProgressLinear ✅
- SkeletonLoader ✅
- EmptyState ✅
- ExpansionPanels ✅
- PullToRefresh ✅

### Media and motion

- Image ✅
- Carousel ✅
- Parallax ✅
- InfiniteScroller ✅
- VirtualScroller ✅
- Hover ✅
- ExpandTransition ✅
- FadeTransition ✅
- ScaleTransition ✅
- SlideTransition ✅

### Pickers

- DatePicker ✅
- TimePicker ✅
- ColorPicker ✅

### Providers and services

- ThemeProvider ✅
- LocaleProvider ✅
- DisplayProvider ✅
- DefaultsProvider ✅
- IconsProvider ✅
- DateAdapterProvider ✅

## Vuetify 0 alignment notes

Vuetify 0 is treated as a capability reference for a lower-level, composable UI engine rather than only a styled component catalogue. For this Rust/Floom project that means:

- keep component contracts design-system-neutral;
- keep visual tokens outside user-facing component APIs;
- allow compile-time tree-shaking through per-component crates;
- keep state and interaction behavior testable without rendering;
- later add headless primitives where component behavior should be reusable across Material, Cupertino, and Fluent.

### Batch 9 API contract (DataTable / Treeview / Calendar / Carousel / VirtualScroller)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `DataTable::columns(Vec<DataTableColumn>)` (`DataTableColumn { title: String, align: ColumnAlign }`,
  `ColumnAlign::Start | Center | End`) + `DataTable::rows(Vec<Vec<String>>)` +
  `DataTable::striped(bool)` + `DataTable::dense(bool)` + `DataTable::on_row_click(impl Fn(usize))`
  — header uses `justify_start/center/end` per-column; body rows are owned `String` data (no
  `AnyView`), so the whole table is `Clone`-free and re-renderable. Wrapped in an overflow-clipped
  container (`.overflow_x(Overflow::Clip).overflow_y(Overflow::Clip)` — there is no `clip()` style
  method in this Floom build).
- `Treeview::nodes(Vec<TreeviewNode>)` (`TreeviewNode { label: String, icon: Option<String>,
  children: Vec<TreeviewNode>, expanded: bool }`) + `Treeview::on_node_click(impl Fn(String))`
  — recursive flatten; expand/collapse mutates the `RwSignal<Vec<TreeviewNode>>` and rebuilds the
  list through `dyn_view` (the `Box<dyn Fn() -> IV>: IntoView` coercion does NOT fire for
  `Container::new(Box::new(...))`, so use `dyn_view(move || ...)` instead). The toggle callback is
  an `Rc<dyn Fn(Vec<usize>)>` (annotate the type so it coerces when passed by `&Rc<dyn Fn>` to the
  recursive helper). Each `FnMut` click closure must clone its `Vec<usize>` path *inside* the
  closure, never move it in from the outer scope.
- `Calendar::value(CalendarDate)` (`CalendarDate { year: i32, month: u32, day: u32 }`) +
  `Calendar::show_week_numbers(bool)` + `Calendar::on_change(impl Fn(CalendarDate))` — fixed 6×7
  grid; optional leading ISO week-number cell per row (reuses the DatePicker date math:
  `weekday_of_first` / `days_in_month` / `week_number`); selection sets the `CalendarDate` and
  fires the callback.
- `Carousel::slides(Vec<AnyView>)` + `Carousel::height(f32)` + `Carousel::auto_play(bool)` +
  `Carousel::interval_ms(u64)` + `Carousel::show_arrows(bool)` + `Carousel::show_dots(bool)` +
  `Carousel::on_change(impl Fn(usize))` — all slides are rendered once as absolutely-positioned
  `AnyView`s (`inset_*_pct` + `hide()` when not current); arrows/dots are overlay controls.
  `slides` takes owned `AnyView`s because `AnyView` is not `Clone`.
- `VirtualScroller::items(Vec<Box<dyn Fn() -> Box<dyn View>>>)` + `VirtualScroller::height(f32)`
  + `VirtualScroller::on_reach_end(impl Fn())` — windowed spacer list: a `RwSignal<f32>` offset is
  updated by a `listener::PointerWheel` handler that reads `e.delta` (`ScrollDelta::PixelDelta(v)
  => v.y as f32`, `LineDelta/PageDelta => y * item_height`). `ScrollDelta` lives at
  `floom::ui_events::ScrollDelta` (crate root), `PointerScrollEvent` at
  `floom::ui_events::pointer::PointerScrollEvent`. Item factories are `Box<dyn Fn() -> Box<dyn View>>`
  (lazy, since `AnyView` is not `Clone`); only the visible window + overscan is built.

### App / Main + layout provider (layout auto-insets)

The application shell is a DS-agnostic layout system. A single process-wide
`RwSignal<LayoutState>` lives in `floom-core-theme` (every primitive already depends on it),
so no shell primitive needs a `floom-widget-runtime` dependency — registration uses the
fully-qualified `floom_core_theme::layout::*` path.

- `LayoutState` aggregates six `f32` slots — `system_bar`, `app_bar`, `footer`,
  `bottom_navigation`, `nav_drawer_left`, `nav_drawer_right`. Convenience accessors:
  `top() = system_bar + app_bar`, `bottom() = footer + bottom_navigation`,
  `left() = nav_drawer_left`, `right() = nav_drawer_right`.
- `layout_signal() -> RwSignal<LayoutState>` — thread-local `OnceCell` accessor (mirrors
  `theme_signal`); the UI runs single-threaded so the `!Sync` `RwSignal` is safe.
- `register_layout(slot: LayoutSlot, value: f32)` — mutates the signal via `RwSignal::update`.
  Called by every shell primitive's `view()` with its resolved height/width.
- `layout_provider(child)` / `App::new().child(child)` — ensures the signal exists, then
  renders `child` in a full-viewport `Container`.

Neutral builder API:

- `App::child(impl IntoView)` — root frame; `view()` calls `layout_signal()` (initializes the
  signal) and paints the background. Content is meant to be a `v_stack` of `[Main, overlays]`.
- `Main::child(impl IntoView)` — content region. `view()` builds **reactive spacers**: `top`/
  `bottom` (height = `layout.top()`/`layout.bottom()`, `width_pct(100.0)`) and `left`/`right`
  (width = `layout.left()`/`layout.right()`, `height_pct(100.0)`), composing
  `[top, [left, content, right], bottom]`. Because this Floom build has no per-side padding
  (`padding_top/bottom/left/right` do not exist — only `padding_horiz`/`padding_vert`), insets
  are applied as spacers rather than padding.

**Composition contract (critical):** the shell bars (AppBar, Footer, SystemBar,
BottomNavigation) and the NavigationDrawer are **absolute overlays** over a full-viewport
`Main`; they must NOT be flex siblings of `Main`, or the `top`/`bottom` spacers would
double-inset. `NavigationDrawer` is already absolute (`inset_top(0).inset_left(0)`); wrap
`AppBar`/`Footer` in an absolute `Container` (`s.absolute().inset_top(0.0).inset_left(0.0)
.width_full()` / `inset_bottom(0.0)`) when composing the shell. The bars still register their
sizes into the signal even when wrapped, so `Main` insets correctly. Opening the drawer sets
`NavDrawerLeft` to its width via a `create_effect`, so `Main`'s left spacer reacts live.

### Batch 10 API contract (Form / ValidationMessages)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Form::child(impl IntoView)` — a grouping surface (surface bg, rounded, padded) that stacks
  its children vertically and, when `show_submit` is set, renders a submit button. The submit
  button is **validity-gated**: it is primary-filled and clickable only while `valid_signal()`
  holds `true`; when `false` it is greyed (`surface_variant`) and its click handler no-ops.
- `Form::submit_label(String)` + `Form::show_submit(bool)` (default `"Submit"` / `true`) +
  `Form::on_submit(impl Fn())` — label, visibility, and callback for the submit button.
- `Form::valid_signal() -> RwSignal<bool>` and `Form::errors_signal() -> RwSignal<Vec<String>>`
  — the component **owns** these signals (mirrors `Dialog::open_signal()`); the consumer
  obtains them and drives them. The form renders its `errors` as a reactive error list above
  the button (reusing `ValidationMessages` styling/tokens). Validity is therefore
  consumer-computed (e.g. from field `RwSignal`s via a `create_effect`), which fits Rust's
  owned-signal reactive model — fields are passed as already-built `AnyView` children, so the
  form does not introspect them.
- `ValidationMessages::messages_signal() -> RwSignal<Vec<String>>` — the standalone
  "校验提示" primitive: it owns the messages signal and renders the bound `Vec<String>` in the
  theme `error` colour at a small font size (`ValidationMessagesTokens { font_size, gap }`).
  An empty signal collapses to a zero-height `empty()` view. `messages(Vec<String>)`
  convenience sets a static list; `gap(f32)` / `font_size(f32)` override spacing/size.

### Batch 11 API contract (Stepper / Breadcrumbs)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Stepper::items(Vec<String>)` + `Stepper::selected(usize)` + `Stepper::editable(bool)` +
  `Stepper::non_linear(bool)` + `Stepper::on_change(impl Fn(usize))` — an ordered horizontal
  progression of circular step markers joined by connectors. The current step is held in an
  internal `RwSignal<usize>` (so taps update it and fire the callback) — the same pattern as
  `BottomNavigation`. Each step renders `✓` when `index < selected` (completed) and its
  1-based number otherwise; completed/active steps are primary-filled, upcoming steps are
  surface-filled with an `outline` border; the connector after a step fills with `primary`
  once that step has been passed. `editable` enables clicking completed/earlier steps
  (navigate backward); `non_linear` enables clicking any step (jump anywhere). A step is
  clickable only when `editable && index < selected || non_linear`. The trail is wrapped in a
  `dyn_view` so selection changes re-render reactively.
- `Breadcrumbs::items(Vec<String>)` + `Breadcrumbs::on_click(impl Fn(usize))` — a `>`-
  separated horizontal trail (divider comes from `BreadcrumbsTokens.divider`, default `/`).
  The last item is the current location: it is non-clickable and styled `on_surface` /
  `FontWeight::Medium`; earlier items are `on_surface_variant` and clickable (`cursor:
  Pointer`), firing `on_click(index)`. Also rendered inside a `dyn_view` for reactivity.

### Form / ValidationMessages demo wiring (kit crates; gallery built)

The gallery has a `Form` page and a `ValidationMessages` page. The `Form` page
shows a form with a submit button and one without; the `ValidationMessages`
page shows the error list it collects. The richer v1 wiring — a `Form` wrapping
two `TextField`s (Name, Email) plus a `ValidationMessages`, a `create_effect`
deriving `valid` + the error `Vec` from the field value signals (obtained via
each `TextField`'s `on_change`), empty Name/Email and a missing `@` in Email
producing messages, the submit button disabled until the list is empty, and a
"Registered!" line appearing on submit — is still **planned**: it belongs to
the demo-gallery-plan.md A–D stage, and it exercises both the gated submit and
the reactive error list.

### Batch 16 API contract (Radio / Overlay)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Radio::new(label)` + `Radio::value(String)` + `Radio::checked(bool)` +
  `Radio::disabled(bool)` + `Radio::on_change(impl Fn(bool))` — an **independent** single
  radio button (not a list model like `RadioGroup`). It binds `checked: bool` reflected into an
  internal `RwSignal<bool>`; clicking the circle/label toggles the state and fires `on_change`,
  and keyboard Enter/Space also toggle (via `set_disabled` + `keyboard_navigable`). `value` is an
  informational group label currently held for parity/host-side mutual exclusion — the component
  does NOT enforce exclusivity across sibling `Radio`s; the host wires that by resetting sibling
  `checked`/signals. `view()` renders an `h_stack` of a circular `Container` (border +
  `border_radius(9999.0)`, an inner dot shown only when `checked`) plus the optional `label_view`.
- `Overlay::new()` + `Overlay::show(bool)` + `Overlay::child(impl IntoView)` +
  `Overlay::on_click(impl Fn())` + `Overlay::opacity(f32)` (default 0.5, 0.0–1.0) +
  `Overlay::show_signal() -> RwSignal<bool>` — a scrim-based base overlay layer built on
  `Overlay::new(scrim)` (same primitive as `Dialog`/`Sheet`). The scrim is a full-viewport
  `Container` (width/height 100%) painted with the theme `scrim` colour at `with_alpha(scrim,
  opacity)`; it is `apply_if(!show.get(), hide())` so it collapses reactively when closed, taps
  the scrim run `show.set(false)` (auto-close) then fire `on_click`, and return
  `EventPropagation::Continue`. The content layer is a `Container::new(child)` (surface bg +
  `border_radius(12)`) whose own `on_event(Click, …)` returns `EventPropagation::Stop` to prevent
  interior taps from bubbling to the scrim (so clicking inside does not close the overlay). The
  stored `RwSignal<bool>` set in `new()`/`show()` is what `show_signal()` returns, so an external
  button can toggle it exactly like `Dialog::open_signal()`.

### Batch 17 API contract (Parallax / InfiniteScroller / Hover)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `Parallax::new()` + `Parallax::child(impl IntoView)` + `Parallax::factor(f32)` (default 0.3)
  + `Parallax::scroll_signal(RwSignal<f32>)` — a host-scroll-driven translate layer. The host
  owns the scroll offset (passed via `scroll_signal()`, like Radio leaves mutual exclusion to
  the host), and `view()` translates the child by `-scroll.get() * factor` using `translate_y`.
  `factor` controls how far the content lags behind the scroll (higher = more parallax).
- `InfiniteScroller::new()` + `InfiniteScroller::child(impl IntoView)` +
  `InfiniteScroller::content_height_signal(RwSignal<f32>)` + `InfiniteScroller::content_height(f32)`
  + `InfiniteScroller::viewport(f32)` + `InfiniteScroller::threshold(f32)` (default 80.0) +
  `InfiniteScroller::on_load_more(impl Fn())` + `InfiniteScroller::loading_signal() -> RwSignal<bool>`
  + `InfiniteScroller::loading(bool)` — a wheel-driven windowed viewport reusing
  `md-virtual_scroller`'s proven `PointerWheel` + `ScrollDelta` (`PixelDelta v.y as f32`,
  `LineDelta * 40`, `PageDelta * viewport`) + `RwSignal<f32>` offset + `Overflow::Clip` pattern
  (Floom's own `Scroll` widget has no `on_scroll` hook, so it is deliberately avoided). The
  offset is clamped to `[0, content_height - viewport].max(0)`; when `max > 0 && next >= max -
  threshold && !loading`, `on_load_more` fires and the component sets `loading` via
  `loading_signal()`. A `dyn_view` footer shows a "Loading…" label while `loading.get()`, else a
  spacer (the footer style closure must capture `footer_padding` by `move`, not by reference, or
  it fails the `'static` lifetime bound). The component owns `loading_signal()`.
- `Hover::new()` + `Hover::child(impl IntoView)` + `Hover::on_hover_change(impl Fn(bool))` +
  `Hover::hovered_signal() -> RwSignal<bool>` — tracks pointer hover via `listener::PointerEnter`
  / `listener::PointerLeave` (each returns `EventPropagation::Continue`) setting an internal
  `RwSignal<bool>` and firing the callback. `view()` tints the background with
  `with_alpha(on_surface, highlight_alpha as u8)` when hovered (default `highlight_alpha` 8,
  `transition_ms` 120 — `with_alpha` takes a `u8`, not `f32`). The callback is wrapped in
  `Rc<Option<Box<dyn Fn(bool)>>>` with `on_enter`/`on_leave` clones so the single `Fn` feeds both
  handlers; `resolved_style()` must be called before moving `self.props`.

### Batch 18 API contract (Fade / Scale / Expand / Slide Transitions)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `FadeTransition::new()` + `FadeTransition::child(impl Fn() -> impl IntoView)` (factory
  closure) + `FadeTransition::show_signal() -> RwSignal<bool>` — a zero-dependency enter/leave
  wrapper. `view()` uses `dyn_view` to swap a `Container` (the child) in/out of an `Empty` when
  `show` flips; the Container carries an `Animation` with `run_on_create(true)` +
  `run_on_remove(true)`, so Floom plays keyframe 0→100 (opacity 0→1) on mount and reverses
  100→0 on unmount. The child is a factory closure because it is re-created on every enter/leave
  (`AnyView` is not `Clone`, so it cannot be shared between the two `dyn_view` branches).
- `ScaleTransition::new()` + `ScaleTransition::child(...)` + `ScaleTransition::show_signal()` —
  same `dyn_view` + `Animation` shape; the animation interpolates `scale` from `min_scale` (token,
  default 0.8) to 1.0 (keyframe 0 `s.scale((min_scale*100.0).pct())`, keyframe 100
  `s.scale(100.pct())`).
- `ExpandTransition::new()` + `ExpandTransition::child(...)` + `ExpandTransition::show_signal()`
  — animates `height` 0→natural; the Container is `Overflow::Clip`-clipped on both axes so the
  content does not spill during collapse (keyframe 0 `s.height(0.0)`, keyframe 100
  `f.computed_style()` = the natural height).
- `SlideTransition::new()` + `SlideTransition::direction(SlideDirection)` (`X` default / `Y`) +
  `SlideTransition::child(...)` + `SlideTransition::show_signal()` — translates the child by
  `offset` px (token, default 100.0) on the chosen axis at keyframe 0 (`translate_x`/`translate_y`)
  settling at 0 (`f.computed_style()`). The Copy `direction`/`offset` are pre-extracted and the
  animation closure is `move`, so it is `'static` — a bare closure would borrow `style` from the
  outer `dyn_view` closure and fail the lifetime bound (this was the one build-blocker for
  Batch 18: the same `'static`-closure fix had to be applied to Fade/Scale/Expand too).

The `show_signal()` getter mirrors `Dialog::open_signal()` / `Overlay::show_signal()`: call it on
`&self` **before** `view(self)`, keep the returned `RwSignal<bool>`, and flip it from an external
button to play the enter or leave animation (`show_signal().update(|v| *v = !*v)`). The demo wires
each transition to a "Toggle …" button.

### Batch 19 API contract (IconButton / ExpansionPanels / SpeedDial / ChipGroup / ListGroup / Counter / Responsive / CheckboxBtn / ListSubheader)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `IconButton::new(icon: impl Into<String>)` + `IconButton::size(f32)` +
  `IconButton::variant(IconButtonVariant)` (`Text`/`Contained`/`Outlined`, default `Text`) +
  `IconButton::disabled(bool)` + `IconButton::on_click(impl Fn() + 'static)` — an icon button that
  renders the `icon` through Floom `svg()`, so **pass a real SVG string, not an icon name** (unlike
  `Icon`, which uses the icon font). `Text` is the transparent/flat variant; `Contained`/`Outlined`
  add a `primary`/`outline` background + border.
- `ExpansionPanels::new()` + `ExpansionPanels::panel(title: impl Into<String>, content: impl
  Fn() -> AnyView + 'static)` (repeatable) + `ExpansionPanels::multiple(bool)` +
  `ExpansionPanels::on_change(impl Fn(usize, bool))` — a vertical list of collapsible panels. The
  header shows a `▸`/`▾` glyph that flips with the panel's expanded state; `content` is a factory
  closure rebuilt on toggle (the panel body is swapped in/out of an `Empty` via `dyn_view`). With
  `multiple(false)` only one panel is open at a time.
- `SpeedDial::new()` + `SpeedDial::action(icon: impl Into<String>, label: Option<String>)` +
  `SpeedDial::direction(SpeedDialDirection)` (`Up`/`Down`/`Left`/`Right`, default `Up`) +
  `SpeedDial::fab_icon(String)` + `SpeedDial::open(bool)` + `SpeedDial::disabled(bool)` +
  `SpeedDial::on_action(impl Fn(usize))` — a floating-action-button that reveals a cluster of action
  buttons. `fab_icon` defaults to a plus SVG when empty; actions render as `svg()` icons (pass SVGs);
  tapping the FAB flips `open` and the actions `dyn_view` appears above/below/beside the FAB per
  `direction`. `on_action` reports the clicked action index.
- `ChipGroup::new()` + `ChipGroup::chip(label: impl Into<String>)` + `ChipGroup::multiple(bool)` +
  `ChipGroup::mandatory(bool)` + `ChipGroup::on_change(impl Fn(Vec<usize>))` — a horizontal,
  selectable chip row. Single-select (`multiple(false)`) clears others; `mandatory` prevents
  deselecting the last selected chip. Selected chips use the `secondary_container` role.
- `ListGroup::new()` + `ListGroup::item(label: impl Into<String>)` + `ListGroup::title(String)` +
  `ListGroup::expanded(bool)` + `ListGroup::inset(bool)` + `ListGroup::on_toggle(impl Fn(bool))` —
  a collapsible list with a header (the `title`, or empty) and a `▸`/`▾` glyph. Items are rebuilt
  inside a `dyn_view` from a `Vec<String>` clone (the body uses `indent` vs `content_padding` for
  `inset`). Tapping the header toggles and fires `on_toggle`.
- `Counter::new()` + `Counter::value(i64)` + `Counter::min(i64)` + `Counter::max(i64)` +
  `Counter::step(i64)` + `Counter::on_change(impl Fn(i64))` — a numeric stepper: `−` button, the
  current value, `+` button. `clamp_value` keeps the value within `[min, max]` (both `i64`, not
  `Option` — the builder wraps them); `step` is added/subtracted on tap and `on_change` fires.
- `Responsive::new(aspect_ratio: f32)` + `Responsive::child(impl Fn() -> AnyView + 'static)` — a
  container that locks a width:height ratio via `aspect_ratio(aspect).width_full()` (there is **no**
  `min_height` style in this Floom build, so the ratio is enforced by `aspect_ratio`, not a min
  height). The child is a factory closure rebuilt on mount (like the transitions).
- `CheckboxBtn::new()` + `CheckboxBtn::checked(bool)` + `CheckboxBtn::disabled(bool)` +
  `CheckboxBtn::label(String)` + `CheckboxBtn::on_change(impl Fn(bool))` — a button-style checkbox
  (a `✓` mark on a filled/outlined rounded box plus an optional label). Click toggles and fires
  `on_change`; `Key::Enter` / `Key::Character(" ")` also toggle (via `keyboard_navigable`).
- `ListSubheader::new(text: impl Into<String>)` + `ListSubheader::inset(bool)` — a section divider
  label. `inset(true)` widens the horizontal padding (`inset_padding`) vs the standard
  `padding_horizontal`; styled `on_surface_variant` at the token `font_size`.

### Batch 20 API contract (NumberInput / PullToRefresh / ConfirmEdit)

Neutral builder API shared by all three design systems (verify in the kit crates):

- `NumberInput::new()` + `NumberInput::value(i64)` + `NumberInput::min(i64)` +
  `NumberInput::max(i64)` + `NumberInput::step(i64)` + `NumberInput::label(impl Into<String>)` +
  `NumberInput::disabled(bool)` + `NumberInput::on_change(impl Fn(i64) + 'static)` — a numeric
  field with `−` / `+` stepper buttons wrapping a `TextInput` (the `md-text-field` entry point,
  `TextInput::new(RwSignal<String>)` — **not** the `text_input()` function). `min`/`max` are
  wrapped into `Option` and a shared `clamp_value` keeps both the typed text and the stepper
  results inside `[min, max]`; parsing the `TextInput` back to `i64` on change fires `on_change`
  (the same clamped-stepper shape as `Counter`, but the value is freely editable as text). With a
  `label`, the field stacks above the input in a `v_stack`; otherwise it is the bare field
  (`field.into_any()`). `disabled` dims and `set_disabled` on both the input and the field.
- `PullToRefresh::new()` + `PullToRefresh::child(impl Fn() -> AnyView + 'static)` +
  `PullToRefresh::loading(bool)` + `PullToRefresh::threshold(f32)` +
  `PullToRefresh::on_refresh(impl Fn() + 'static)` +
  `PullToRefresh::refreshing_signal() -> RwSignal<bool>` — a downward-drag wrapper. Pointer events
  use `listener::PointerDown`/`Move`/`Up` (the same pattern as `md-range_slider`): `PointerDown`
  records `pbe.state.logical_point().y` and `cx.request_pointer_capture(pointer_id)`; `PointerMove`
  computes `dy = current.y - start.y` and clamps a `pull` signal to `[0, max_pull]` (from
  `PullToRefreshTokens`); on release, if `pull >= threshold`, it sets the internal `refreshing`
  flag and fires `on_refresh`. A `dyn_view` renders the indicator: hidden when `pull <= 0.5` and not
  refreshing, `↓ Pull to refresh` while dragging (`height = pull.min(28)`), or `↻ Refreshing…`
  while `refreshing`. **There is no built-in timer in this Floom build, so the host must clear the
  refresh state via `refreshing_signal().set(false)`** once its async work completes — the demo
  does this with a `create_effect` that watches a request signal. The child is a factory closure
  (`AnyView` is not `Clone`, so it is rebuilt on mount, falling back to `Empty::new().into_any()`).
- `ConfirmEdit::new()` + `ConfirmEdit::child(impl Fn() -> AnyView + 'static)` +
  `ConfirmEdit::value(impl Into<String>)` + `ConfirmEdit::on_save(impl Fn(String) + 'static)` — a
  click-to-edit inline wrapper. In display mode it shows `child` (or the `value` text) plus a `✎`
  edit affordance; clicking toggles an internal `editing` `RwSignal<bool>` and a `dyn_view` swaps in
  a `TextInput` (bound to a `draft` signal seeded from `value`) plus `✓` (save) / `✗` (cancel)
  buttons. Save commits `draft` to the internal `value`, fires `on_save`, and exits edit mode;
  cancel resets `draft` from `value` and exits. The child is a factory closure (rebuilt on mount).

## Implementation order

1. Stabilize the current contract components across Material, Cupertino, and Fluent.
2. Add missing high-use Vuetify controls: `RadioGroup` ✅, `Slider` ✅, `Tooltip` ✅, `Tabs` ✅, `List` ✅ (done 2026-08-10); `Select` ✅, `Textarea` ✅, `Snackbar` ✅, `Chip` ✅, `Badge` ✅ (Batch 3, 2026-08-10). All high-use controls from this step are now implemented.
3. Add layout primitives: done for `Container` ✅, `Grid` ✅, `Divider` ✅, `Sheet` ✅,
   `Toolbar` ✅, `Footer` ✅, `SystemBar` ✅, `AppBar` ✅, `NavigationDrawer` ✅,
   `BottomNavigation` ✅, and `App`/`Main` + the layout provider (auto-insets via the
   shared `LayoutState` signal) ✅ (done 2026-08-11).
4. Add data-heavy components: `DataTable` ✅, `Treeview` ✅, `Calendar` ✅, `Carousel` ✅,
   `Timeline` ✅, `VirtualScroller` ✅, `Table` ✅, `DataIterator` ✅ (Batch 9 / Batch 15, done 2026-08-11).
5. Add providers and defaults system so app-level theme, locale, display, and icon settings can
   flow into every component: `ThemeProvider` ✅, `LocaleProvider` ✅, `DisplayProvider` ✅,
   `DefaultsProvider` ✅, `IconsProvider` ✅, `DateAdapterProvider` ✅ (Batch 13, done 2026-08-11).
6. Batches 14–20 (actions/controls, media/motion, transitions, tier-2, inputs & interaction)
   ✅ — 98 components × 3 design systems (2026-08-11/12).

## Acceptance criteria for a new component

Every new contract component must include:

- neutral props in `floom-core-theme/src/props/`;
- component tokens in `floom-core-theme/src/component_tokens/`;
- Material, Cupertino, and Fluent kit crates (Harmony is consolidated-only today);
- neutral re-exports from each kit crate (there is no `floom-elements` facade);
- tests for Material behavior and compilation for all design systems (satellite
  `cargo test --workspace`, headless);
- no hard-coded visual values in public component contracts.
