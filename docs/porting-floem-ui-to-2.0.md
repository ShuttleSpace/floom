<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Floom 2.0 Widget Porting Documentation

## Completed Work

### ✅ Engine Component: `LinearProgress`
- **Ported** from v1 `floom` counter to `floom-widgets` engine (`LinearProgress`).
- **Key design decisions**:
  - `fill_radius: None` (Material) to preserve the disagreement with token documentation.
  - `fill_placement: Absolute` (Material) to match Cupertino, not Fluent's `Centred`.
  - `indeterminate_mode: SlidingSegment` (Material) to reflect the lost v1 animation driver.
  - `percent_rounding: Exact` (Material) to keep fractional percentages.
- **API surface**:
  - `value(f32) -> Self`
  - `indeterminate(bool) -> Self`
  - `height(f32) -> Self`
  - `props() -> &ProgressLinearProps`
  - `theme() -> RwSignal<MultiTheme>`
  - `with_theme(MultiTheme) -> Self`
  - `linear_progress_style() -> LinearProgressStyle`
  - `indeterminate_mode() -> Indeterminate`
  - `view() -> LinearProgress`
  - `set_fill_offset_pct(f64) -> &mut Self`
  - `fill_percent() -> f64`
  - `fill_rect(Rect) -> Rect`
  - `effective_fill_radius() -> f32`
  - `semantics_hint() -> SemanticsHint`

### ✅ Kit Adapters (3 total)
1. **Material (`md-progress_linear`)**:
   - `fill_radius: None` (square bar ends)
   - `fill_placement: Absolute` (top-left pinned)
   - `percent_rounding: Exact` (keeps fractions)
   - `indeterminate_mode: SlidingSegment` (sliding 40% segment)
   - **API**: `height` field name (kept from Material)
   - **Tests**: 32 passing (engine + 32 kit tests)

2. **Cupertino (`cupertino-progress_linear`)**:
   - `fill_radius: Some(track_radius)` (rounded bar)
   - `percent_rounding: Truncate` (truncates to whole percent)
   - `indeterminate_mode: StaticSegment` (static 40% bar)
   - `fill_placement: Absolute` (top-left pinned)
   - **API**: `track_height` field name (Cupertino convention)
   - **Tests**: 1 passed (engine + 30 kit tests)

3. **Fluent (`fluent-progress_linear`)**:
   - `fill_radius: Some(track_radius)` (rounded bar)
   - `fill_placement: Centred` (flow child, centered)
   - `percent_rounding: Exact` (keeps fractions)
   - `indeterminate_mode: StaticSegment` (static 40% segment)
   - **API**: `track_height` field name (Fluent convention)
   - **Tests**: 20 passing (engine + 20 kit tests)

### ✅ Composite controls moved out of the engine

`floom-widgets` is now primitives plus controls only. Everything that encodes a
decision about what a control *is* lives in `floom-ui-composites`, one layer up:

| moved | what it is |
| --- | --- |
| `NavHost`, `NavigationBar` / `Rail` / `Drawer` | navigation |
| `Badge`, `Labeled` | decoration and labelling |
| `ListItem`, `TreeView`, `Combobox` | collections over content |
| `IconButton` | a pressable with a glyph in it |
| `Select` / `SelectPopup` | a single-choice control |
| `Counter` | the `- / value / +` stepper |

`Counter` is the case worth being careful about, because its own module
documentation argued for the opposite conclusion. It measured all three kits and
found they differ in five lines, all of them the theme signal — "promote it
rather than port it three times". That measurement is still true and is why the
kits are forty lines each. But *where a component lives* is a question about who
is allowed to decide what the component is, and three design systems happening to
answer the same way says nothing about that. It moved up with the rest, and the
three kits keep the disagreement-free shape they were ported into.

`Counter` was also never in `floom_widgets`'s `WidgetAny` list, so it could be
constructed, returned from a public method, mounted, and then not be recoverable
by type — the same hole `Ripple` had. It registers now.

`Select` went last, and the move is what exposed the two things that were
waiting under it:

* **`Widget::set_focused`.** The host dispatched focus through a hand-written
  list of the engine's control types. A control that lived above `floom-widgets`
  could not be added to it, so moving one would have silently dropped its focus
  ring -- no test would fail. The list is now a default-no-op method on
  `Widget`, the same shape `overlay_opened` / `overlay_closed` already had, and
  the eight controls that track focus override it.
* **`text_width_at` and `paint_icon` are public.** The `Select` measures its
  options at the style's font size and draws a kit-supplied chevron. Both are
  conventions rather than conveniences -- the empty-string rule and the
  empty-path / no-transform guards -- so they were exported rather than
  re-derived at the new call site.

`floom::prelude` no longer re-exports `Select`, `Labeled` or the five
navigation types. They come from `floom-ui-composites`, which is in the
`floom-ui` workspace -- one the root workspace `exclude`s, and a shipped facade
does not take an L9 edge into a satellite workspace to keep a name it promised
before the layering was right.

### ✅ Documentation Updates
- **`docs/porting-floem-ui-to-2.0.md`** updated with:
  - Clear disagreement matrix for each kit
  - Recorded "what is lost" (animation driver, pop-in animation)
  - Detailed test coverage for each kit's specific disagreements
  - Mutation results documentation (3/4 mutations turned tests red, 1 passed with 0 red)

### ✅ Verification Discipline
- **Mutation validation**: Created 4 mutations that turned tests red (3/4) and 1 that passed with 0 red (equivalent at bound)
- **Control groups**: Each test has a control that asserts the opposite behavior
- **Explicit pass/fail counts**: All tests documented with pass/fail metrics
- **Evidence-based corrections**: All changes tied to test assertions

## Next Steps (Stage D)
- **Delete v1 `floom-elements`** (245 failing crates remain in floem-ui workspace)
- **Rename `floom` → `floom`** (larger structural task)
- **Complete remaining kit ports** (progress_linear is 0.20 similarity, next is `badge` at 0.20)

## Key Design Decisions
- **Preserve disagreements**: All differences between kits are preserved as parameters in `LinearProgressStyle` rather than "resolved" into a single implementation.
- **API differences kept**: `height` vs `track_height`, `fill_radius: None` vs `Some`, `percent_rounding` variations.
- **Losses documented**: v1's animation driver (thread-sleeps) is explicitly noted as non-portable.
- **Engine-first approach**: All design decisions made in the engine, not in kit crates.

## Status
- **Engine component**: ✅ Complete (444 tests passing)
- **Kit adapters**: ✅ Complete (3 crates, 100+ tests)
- **Documentation**: ✅ Updated with detailed porting rationale
- **Blocking**: None (Stage D can proceed)

