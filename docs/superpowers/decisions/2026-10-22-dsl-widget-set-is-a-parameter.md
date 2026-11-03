# Decision: The DSL emitter's widget set is a caller-supplied parameter

**Date:** 2026-10-22
**Status:** Accepted
**Branch:** `floem2.0`

## Context

`floom-dsl` has two halves, and only one of them is the product:

- `floom_dsl::compile(source) -> Compiled` — parse and lower to the **UI IR**.
  This is the primary API: `floom-hot-reload` and `floom-devtools` both consume
  it, and neither touches the code generator.
- `floom_dsl::emit(document, options) -> Emitted` — turn the IR into Rust
  source that calls concrete widget constructors. **No production caller.**

The emitter's last step needs concrete types, and it used to assume one
widget set: `floom_widgets`. The widget migration broke that assumption.
`Checkbox`, `Radio`, `Select`, `Slider` and `TextField` moved to
`crates/ecosystem/floom-ui/crates/floom-ui-composites`, and the emitter's
import list still named them — so `floom-dsl`'s own fixture stopped
compiling (`unresolved imports floom_widgets::Checkbox, …`).

Three facts constrain the answer:

1. **The root workspace cannot see the satellite.** `crates/ecosystem/floom-ui`
   is a separate workspace the root one `exclude`s. `floom-dsl` cannot name
   `floom_ui_composites` in its own source, and `floom-widgets` cannot
   re-export the composites because the dependency runs the other way
   (composites → widgets).
2. **The IR's vocabulary is complete and should stay that way.**
   `floom-ui-ir::NodeKind` still has every kind, and it is
   design-system-neutral on purpose — a DSL document that says "checkbox"
   should not have to know which kit implements it.
3. **Which widget set an app has is the app's decision**, not the DSL's. An app
   that depends only on `floom-widgets` has the primitives; an app that also
   depends on `floom-ui-composites` has the composite controls.

## Decision

**The widget set becomes an explicit input to the emitter.**

`EmitOptions` gains a `widgets: WidgetSource` field:

- `WidgetSource::Primitives` (the default) — only `floom_widgets`.
- `WidgetSource::WithComposites` — the primitives plus `floom_ui_composites`.
  The caller's crate must depend on `floom-ui-composites`; the emitted program
  imports it.

A kind the chosen set cannot name has **no seat**: it becomes the same
placeholder the kinds with no builder equivalent already become, and the
substitution is recorded in `Emitted::unmapped`. It is never a silent
substitution and never a call to a type that does not exist.

The import list is built from the widget source rather than fixed. The fixed
list was the actual defect: it named types the program could no longer name.

## Consequences

- The DSL's fixture compiles again, and it now compiles **twice**: once
  against the composites, once against the primitives alone. The second run is
  the one that would have caught the original breakage, because it is the case
  where the emitted program's imports have to match the widget set exactly.
- The IR and the DSL no longer diverge: the vocabulary stays in the IR, and
  only the *binding* to concrete widgets is a choice, made by the caller.
- `EmitOptions` is a breaking change for any caller that constructs it
  literally. There are none outside this crate — `emit()` has no production
  caller — so the break costs nothing today.
- A future widget set (a third crate, or a kit-specific set) is a new variant
  and a new line in the import builder, not a new mechanism.

## Alternatives considered

- **Trim the emitter's vocabulary.** Move the five composite kinds into the
  "no builder equivalent" bucket permanently, so the emitter only ever names
  `floom_widgets`. Rejected: the IR keeps the kinds, so the DSL would express
  strictly less than the IR it lowers into, and the two would diverge. It is
  also the wrong default — an app with the composites installed would get
  placeholders for controls it could have had.
- **Move the emitter into the satellite workspace.** The root keeps the parser
  and the IR; the code generator lives next to the widget set it targets.
  Rejected *for now*: it is a breaking public-API move across a workspace
  boundary, and it is the right answer once the emitter has a production
  caller — at which point the caller will say which widget set it wants, and
  the question will be settled by use rather than by argument. Worth
  revisiting when the emitter is actually used.
- **Make the emitter emit IR-level code and let a materialiser bind kinds to
  widgets.** Architecturally the cleanest, and it is what the crates are named
  for (`floom-widget-runtime` exists but binds theme and layout state, not
  kinds). Rejected as disproportionate: it is a new subsystem for a function
  with no caller.

## Verification

- `crates/floom-dsl/src/emit.rs` — `the_header_names_only_the_widgets_the_source_provides`,
  `a_composite_kind_needs_the_composite_widget_source`.
- `crates/floom-dsl/tests/compile_fixture.rs` —
  `the_emitted_program_type_checks_against_the_real_widget_api` (composites),
  `a_program_emitted_against_the_primitives_alone_still_compiles` (primitives),
  and the existing mutation control.
