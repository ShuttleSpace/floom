# floom-tsx TypeScript declarations (`.d.ts`) — design

- **Date**: 2026-11-06
- **Status**: approved for planning
- **Area**: `crates/frontend/floom-tsx`, `crates/app/floom-cli`

## Problem

TSX authors get no editor support today:

- `crates/frontend/floom-tsx/src/decl.rs` only emits interfaces for the kinds a
  *single document* happens to use (`doc.kinds()`), so an unused component has
  no completion.
- The emitted text has no `declare module "floom"` wrapper, no
  `JSX.IntrinsicElements`, and references an undefined `ReadonlySignal<T>`.
  Editors therefore cannot resolve either `import … from "floom"` or `<Stack>`.
- `generate_dts()` is called by one test only; the CLI and build script never
  write a file, so nothing lands where an editor can find it.
- There is no `tsconfig.json` in the repo or in the example app.

The user also requires that **every API advertised in the declarations is one
the parser actually accepts** — a declaration that the TSX front end then
rejects is a bug, not just a missing feature.

## Goals

1. Generate a complete declaration file covering the whole parser-accepted
   surface: runtime imports, every widget component, every accepted prop, every
   emitted event, and the global JSX namespace.
2. Make completion work for components the app has not used yet.
3. Enforce a parity contract: **declared ⊆ parser-accepted**. A test builds a
   TSX document out of the declared surface and compiles it through
   `floom-tsx`; any mismatch fails.
4. Wire generation into the normal `floom run` path (no extra command), and
   make an editor pick it up with zero configuration.

## Non-goals

- A runtime JavaScript package or npm publish.
- Per-file / per-document trimming of the declarations.
- Plugin TypeScript facades (`floom-plugin-*`). That work exists separately in
  the working tree and is not part of this design.
- Changing widget behaviour merely to match a declaration. The declaration is
  derived from the existing schema; where codegen emits a placeholder, the
  declaration stays limited to what the parser accepts.

## Approach

Chosen: **schema-driven, full vocabulary** (A).

| Option | Summary | Trade-off |
|---|---|---|
| **A (chosen)** | Walk `NodeKind::ALL` + `prop_spec` / `events_for` / domains and emit a complete file | Complete completion; schema remains the single source of truth; no drift by construction |
| B | Keep `doc.kinds()`, add module/JSX prelude | Small file, but unused components are not completed — fails the “all APIs” requirement |
| C | Hand-write a static `.d.ts` | Full control, but becomes a second source of truth that drifts from the schema |

## Generated surface

The file is a **script** (no top-level `import`/`export`), so
`declare module "floom"` is a true ambient module declaration and
`declare namespace JSX` is global.

### `declare namespace Floom`

- `Accessor<T>`: callable `(): T`.
- `Setter<T>`: `(value: T): void`.
- One `{Kind}Props` interface per `NodeKind::ALL`, built from
  `prop_spec(kind, name)` over `all_prop_names()`:
  - `Bool` → `boolean` (optional field);
  - `Int` / `Number` → `number`;
  - `Text` / `Color` → `string`;
  - `Ident` with a `domain` → a literal union (`"horizontal" | "vertical"`);
  - `Ident` without a domain → `string`;
  - `List` → `unknown[]`.
- `class?: string` on every kind (the lowerer accepts a `class` attribute).
- `children?: JSX.Element | JSX.Element[] | string` on containers and `Text`.
- Event handlers named `onXxx`, typed from the builder that consumes them:

| Event | Kind | Type |
|---|---|---|
| `Click`, `Focus`, `Blur`, `Dismiss` | all emitting kinds | `() => void` |
| `Toggle` | Checkbox, Switch | `(checked: boolean) => void` |
| `Toggle` | Radio | `() => void` |
| `Change` | Slider | `(value: number) => void` |
| `Change` | Select, Tabs | `(index: number) => void` |
| `Input`, `Submit` | TextField | `(value: string) => void` |
| `Scroll` | List | `(offset: number) => void` |

### `declare namespace JSX`

- `interface Element {}`
- `interface ElementChildrenAttribute { children: {} }`
- `interface IntrinsicElements` mapping every TSX tag name to
  `Floom.{Kind}Props`. Native kinds use the tag the parser accepts: `Web`,
  `Video`, `Map`, `TextInput` (not the IR spelling `NativeWeb`).

### `declare module "floom"`

- `createSignal<T>(value: T): [Floom.Accessor<T>, Floom.Setter<T>]`
- `createMemo<T>(fn: () => T): Floom.Accessor<T>`
- `createEffect(fn: () => void): void`
- `provideContext<T>(name: string, value: T): void`
- `useContext<T = string>(name: string): T`
- `type Component = () => JSX.Element`
- `mount(component: Component): void`
- `app: { setName(name: string): void }`
- `window: { setTitle; setSize; setSize("WxH"); setResizable }`
- `platform: { require(capability: Capability, onUnsupported: Policy): void }`
- `Capability` / `Policy` literal unions matching `entry.rs` exactly.

## Parser-parity contract

**Invariant:** every kind, prop, event, and entry method named in the
declarations must be accepted by `floom-tsx`.

Enforced by tests, not by review:

1. **Tag round-trip** — for every `NodeKind::ALL`, the tag emitted into
   `IntrinsicElements` satisfies `NodeKind::parse(tag) == Some(kind)`.
   This catches the current `NativeWeb`-vs-`Web` class of bug.
2. **Prop acceptance** — every `{Kind}Props` field resolves through
   `prop_spec(kind, name)`, or is `class` / `children`.
3. **Event acceptance** — every `onXxx` maps back through
   `EventName::parse(…)`, and an inline arrow handler using it lowers without
   error.
4. **Entry acceptance** — a generated TSX module calling every declared
   `app` / `window` / `platform` method resolves through `entry::resolve`.
5. **Shim acceptance** — a module using `createSignal`, `createMemo`,
   `createEffect`, `provideContext`, `useContext`, and `mount` compiles.
6. **Full-document round-trip** — the test renders one TSX element per kind
   using that kind’s declared props and events, then runs
   `floom_tsx::compile` over the whole document. Success proves the declared
   surface is parseable end to end.
7. **Encouraged syntax** — the declaration file suggests forms the parser must
   handle:
   - `createSignal<number>(0)` (explicit type argument),
   - `const [n, setN]: [Floom.Accessor<number>, Floom.Setter<number>] = …`,
   - `function App(): JSX.Element { … }`,
   - `const App: Floom.Component = () => …`,
   - `import type { Component } from "floom"`.

Any test that fails is fixed on the parser side (or the declaration is
narrowed if the form is intentionally unsupported), never silenced.

## Delivery and wiring

- `floom-tsx::Generated` gains `types: String`.
- The generated app build script writes
  `<app-dir>/.floom/types/floom.d.ts` on every compile. `.floom/` is already
  git-ignored, so this does not dirty the tree.
- `floom run`’s `write_project` creates `<app-dir>/tsconfig.json` **only when
  absent** (never rewrites a user file):

  ```json
  {
    "compilerOptions": {
      "target": "ES2020",
      "module": "ESNext",
      "moduleResolution": "Bundler",
      "jsx": "preserve",
      "strict": true,
      "noEmit": true,
      "skipLibCheck": true
    },
    "include": [".floom/types/**/*.d.ts", "**/*.tsx", "**/*.ts"]
  }
  ```

- `examples/floom-tsx-app/README.md` documents the one-time generation.
- Existing working-tree code that writes `<app>.d.ts` beside the app is
  adapted to the path above; it is not duplicated.

## Testing

- Rust unit/golden tests for the generated text (module block, JSX block,
  literal unions, event signatures, native names).
- Parity tests 1–7 above.
- A `tsc` smoke test runs only when `tsc` is on `PATH`; otherwise it prints a
  skip reason. CI must not depend on Node.
- Manual end-to-end: `cargo run -p floom-cli -- run examples/floom-tsx-app/app.tsx --build-only`,
  then `npx tsc --noEmit -p examples/floom-tsx-app` → zero errors.
- User check: open `app.tsx` in the editor and confirm completion for an
  unused component and for `onChange` on `Slider`.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| `tsc` absent in CI | Guarded test, skip with a message |
| Build script writes into the source tree | Only under the ignored `.floom/types/` |
| Generated file drifts from schema | Generated from the same tables the parser validates against; parity tests |
| Two competing `.d.ts` locations | Adapt the existing WIP to a single path |
| `strict` flags the example app | Run `tsc` against the example; narrow declarations rather than adopting `any` |
