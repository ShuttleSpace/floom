# Floom TSX Front-End (Compile-Time Route A) Design

Date: 2026-11-02

## Summary

Floom will add a TypeScript/TSX front end so applications can be authored in TSX
while **rendering and packaging stay on Floom's Rust platform**. The TSX source
is compiled at build time into Floom's existing `floom-ui-ir::UiDocument`, then
lowered through the existing `floom-dsl::emit` backend into a Rust builder
program that is compiled into the application binary and drawn by Floom's own
renderers (vello/wgpu, tiny-skia, TUI, WASM). A TSX-side reactive shim
(`createSignal`/`createMemo`/`createEffect`/`provideContext`/`useContext`) is
translated at compile time into `floom_signal`, so TSX components are genuinely
reactive with zero runtime VM.

The design deliberately mirrors the TSX development experience of
`longbridge/nsh` (which Floom was compared against) but anchors rendering to
Floom's own renderer stack rather than to platform-native views.

## Goals

- Add a `floom-tsx` crate (build-time/dev tool, NOT shipped in the application
  binary) that parses TSX with `swc_ecma_parser` and lowers it to
  `floom-ui-ir::UiDocument`.
- Cover the **full** `floom-ui-ir::NodeKind` vocabulary (primitives +
  composites) — the same closed enum `floom-dsl` already maps.
- Translate the TSX reactive shim directly into `floom_signal` at compile time
  (Approach A: 1:1 mapping, no new runtime, emitted Rust is idiomatic Floom).
- Reuse `floom-dsl::emit` to turn the `UiDocument` into a Rust builder program;
  extend emit only as needed to support reactive (signal-backed) props via effect
  wiring.
- Integrate via the application's `build.rs`: glob `src/**/*.tsx` →
  `floom-tsx` → `UiDocument` → `floom-dsl::emit` → `OUT_DIR/ui_gen.rs` →
  `include!` in `main.rs`.
- Generate `.d.ts` declarations from the IR schema so TSX authors get editor
  completions (mirrors nsh's `.nsh/types`).
- Support `class="text-primary"` token resolution into `StyleProps`.
- Carry source spans on every diagnostic (align with `floom-dsl`'s `Span`
  style).

## Non-Goals (explicitly NOT in this spec — recorded for future plan updates)

These were discussed and deliberately excluded from this first spec. Each is a
candidate for a later plan revision; they are listed so the boundary is
unambiguous and can be revisited.

- **Route B (runtime VM):** No `rquickjs`/QuickJS sandbox, no runtime loading of
  TSX packages, no signature/sandbox capability boundary. (This is the nsh-style
  "load TSX at runtime" path; deferred because the user's hard constraint is
  "rendering and packaging handled by Floom's Rust platform", which Route A
  already satisfies with lower risk.)
- **TSX hot reload:** No in-place reload of TSX during development. (Note: the
  IR `NodeKey` is dotted and stable across edits — `floom-hot-reload` already
  diffs by key — so this is a natural later add-on, but not in this spec.)
- **GPUI backend:** No GPUI renderer backend for Floom. (nsh has one; Floom
  already has vello/wgpu + tiny-skia and does not need GPUI.)
- **Native-view rendering of TSX:** TSX always renders through Floom's own
  renderer stack, never by lowering to UIKit/AppKit/Win32/GTK4 native views
  (which is nsh's model). This is a permanent boundary, not a deferral.
- **Full Tailwind utility-class library:** Only the basic `class="token"` →
  `StyleProps` resolution is in scope. A complete Tailwind utility set
  (spacing/responsive/variants) is deferred.
- **Mixed Rust+TSX projects:** This spec covers apps authored wholly in TSX. The
  nsh-style "TSX screen imports a Rust component" bridge is deferred.
- **Web-target TSX bundling specifics:** TSX compiles to Rust like any other
  Floom target; no special WASM bundling beyond what `floom-platform-web` already
  does.

## Confirmed Decisions (from brainstorming)

1. **Scope:** Compile-time Route A only.
2. **Reactivity:** Signal shim designed together with the spec (not deferred).
3. **Parser:** `swc_ecma_parser` (production-grade, build-time dependency only).
4. **Component coverage:** Full `NodeKind` vocabulary.
5. **Build integration:** `build.rs` emits `OUT_DIR/ui_gen.rs`, `include!`d.
6. **Signal binding:** Approach A — direct translation to `floom_signal` +
   effect wiring; no SolidJS-style fine-grained compiler, no React-style
   whole-component re-render.

## Architecture

```
.tsx source
   │  floom-tsx (NEW, build-time only)
   │    parser.rs  : swc_ecma_parser → TSX AST
   │    lower.rs   : TSX AST → UiDocument (+ signal-shim recognition)
   │    decl.rs    : schema → .d.ts
   │    error.rs   : span-based diagnostics
   ▼
floom-ui-ir::UiDocument        (EXISTING — the bridge)
   JSON-serializable, closed NodeKind enum, validate(), to_json/from_json
   ▼
floom-dsl::emit               (EXISTING, reused; small reactive-prop extension)
   → Rust builder program
   ▼
application binary → Floom renderers (vello/wgpu, tiny-skia, TUI, WASM)
                    + Floom platform layer + Cargo/`just build` packaging
```

The `UiDocument` is the lingua franca: `floom-dsl` already proves the
`source → UiDocument → Rust` pipeline, and `floom-tsx` only replaces the
front-end parser. floom-tsx adds nothing to the shipped binary — `swc` is a
build-time dependency only.

## TSX → IR mapping

`floom-ui-ir` types (read from source): `UiDocument { version, root, bindings,
nodes }`, `UiNode { key, kind, props, slots, events }`, `NodeKind` (closed enum:
Text/Icon/Image/Container/Spacer/Divider/Button/Checkbox/Radio/Switch/Slider/
Progress/Select/TextField/Stack/Grid/List/Conditional/Overlay/Dialog/Popup/Tabs/
NavigationBar/NavigationRail/NavigationDrawer/Native(Web|Video|Map|TextInput)),
`Prop { name, value: PropValue }`, `Slot { name, children }`, `EventBinding {
event, handler }`, `Binding { name, kind: Handler|Value(ty) }`.

| TSX | UiDocument |
|-----|-----------|
| `<Button label="hi" />` | `UiNode{kind:Button, props:[Prop{label, Str("hi")}]}` |
| `<Stack>{xs}</Stack>` | `UiNode{kind:Stack, slots:[Slot{default, xs}]}` |
| `onClick={h}` | `EventBinding{event:Click, handler:"h"}` |
| `{count()}` as prop | `PropValue::Ref("count")` → resolved in `bindings` |
| `show ? <A/> : <B/>` | `UiNode{kind:Conditional, props:[when:Ref(show)], slots:[a,b]}` |
| `items.map(i => <Row key={i.id}/>)` | `UiNode{kind:List, slots:[...]}` (key from `key=`) |
| `class="text-primary"` | token lookup → `StyleProps` in `props` |
| module-level `createSignal`/`const` | `UiDocument.bindings` |

`NodeKind::parse` is already forgiving of case and separators (`text_field`,
`TextField`, `textfield` all parse to `TextField`), so TSX component names route
straight through it.

## Signal shim (Approach A)

TSX exposes a `floom` module whose API mirrors `floom_signal`:

```tsx
import { createSignal, createMemo, createEffect, provideContext, useContext } from "floom";

export function Counter() {
  const [count, setCount] = createSignal(0);
  const double = createMemo(() => count() * 2);
  return (
    <Stack axis="vertical" gap={8}>
      <Text label={() => `Value: ${count()}`} />
      <Button label="+" onClick={() => setCount(count() + 1)} />
    </Stack>
  );
}
```

`lower.rs` recognizes these calls and emits:

- `let count = floom::reactive::create_signal(0);`
- destructuring `[get, set]` → `count.get()` / `count.set(v)`
- `count()` → `count.get()`; `setCount(v)` → `count.set(v)`
- `createMemo(fn)` → `floom::reactive::create_memo(fn)`
- `createEffect(fn)` → `floom::reactive::create_effect(fn)`
- `provideContext`/`useContext` → `floom::reactive::{provide_context, use_context}`

Closures captured into a `Child::new` builder must be `Send + Sync`; if a signal
needs to cross that boundary, follow `floom-showcase` and upgrade to
`SyncBaseSignal` / `Arc<Mutex<…>>`.

## IR → Rust emit (reuse + one seam)

Call `floom_dsl::emit::emit(&doc, WidgetSource::WithComposites)` to produce the
Rust builder program.

**The only required change to `floom-dsl::emit`:** today it emits a static tree.
To let a signal drive a widget update, emit must support a **reactive prop** — a
prop value that is a Rust closure reading a signal. The extension: when emit
encounters a signal-backed prop, it appends an effect registration inside the
generated `mount` function:

```rust
create_effect(move || {
    let v = /* closure that reads count */;
    host.widget_mut::<Text>(id).set_content(v);
});
```

This produces exactly the pattern `floom-showcase` uses (`SyncBaseSignal` +
keyed children) and reuses Floom's existing update path. Scope the change behind
a `reactive` flag so non-TSX emit callers are unaffected.

## Build integration

```rust
// app/build.rs
fn main() {
    let out = std::env::var("OUT_DIR").unwrap();
    let tsx: Vec<_> = glob::glob("src/**/*.tsx").unwrap().filter_map(Result::ok).collect();
    let doc = floom_tsx::compile_project(&tsx).expect("tsx");
    let rust = floom_dsl::emit::emit(&doc, WidgetSource::WithComposites);
    std::fs::write(format!("{out}/ui_gen.rs"), rust).unwrap();
    println!("cargo:rerun-if-changed=src");
}
```

```rust
// src/main.rs
mod ui_gen { include!(concat!(env!("OUT_DIR"), "/ui_gen.rs")); }
fn main() {
    floom::launch(|host| ui_gen::mount(host));
}
```

## Error handling

All diagnostics carry a source `Span` (align with `floom-dsl`). Categories:

- swc parse errors (with the parser's span),
- IR validation errors (reuse `UiDocument::validate` — key validity, duplicate
  key, root reachability, slot parentage, cycle detection, binding type
  resolution),
- schema type errors (prop type mismatch against `prop_spec`),
- unresolved signal bindings (a `{ref}` that names no `createSignal`).

## Testing

- **Unit:** TSX → `UiDocument` golden JSON (deterministic serialization) under
  `floom-tsx/tests/`.
- **Integration:** the emitted Rust must compile against the Floom prelude (CI
  builds the generated `ui_gen.rs`).
- **Golden example:** add `examples/floom-tsx-showcase` mounting a `counter.tsx`;
  headless pump asserts `painted > 0` and that a synthesized click changes the
  label. Reuse the existing golden-example harness.
- **`.d.ts`:** assert the generated declaration matches the IR schema.

## Phased delivery

- **P0:** static TSX (no signals) → `UiDocument` → emit → Rust, mounted in a
  demo.
- **P1:** signal shim (`createSignal`/`createMemo`/`createEffect`/context) →
  `floom_signal` translation + effect wiring seam in `floom-dsl::emit`.
- **P2:** `class=` token resolution + `.d.ts` generation.

(Non-Goals items — Route B, hot reload, mixed Rust+TSX, full Tailwind — are
explicitly out of this sequence and tracked in Non-Goals for later plan
revisions.)

## Future extensions (tracked, not in this spec)

See Non-Goals. The most likely first follow-up is **Route B (runtime VM)** once
Route A ships, because `floom-ui-ir`'s stable `NodeKey` already makes key-based
hot reload and AI-generated-UI loading low-risk additions.
