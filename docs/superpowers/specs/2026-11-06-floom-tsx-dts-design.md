# floom-tsx TypeScript declarations (`.d.ts`) — design

- **Date**: 2026-11-06
- **Status**: approved for planning
- **Area**: `crates/frontend/floom-tsx`, `crates/app/floom-cli`

## Problem

TSX authors get no reliable editor support today:

- `crates/frontend/floom-tsx/src/decl.rs` only emits interfaces for the kinds a
  *single document* happens to use (`doc.kinds()`), so an unused component has
  no completion.
- The emitted text has no `declare module "floom"` wrapper, no
  `JSX.IntrinsicElements`, and references an undefined `ReadonlySignal<T>`.
  Editors therefore cannot resolve either `import … from "floom"` or `<Stack>`.
- There is no `tsconfig.json` in the repo or in the example app.

A partial implementation is currently uncommitted in the working tree
(`run_app.rs` calls `floom_tsx::generate_dts` from the generated build script
and writes `<app>.d.ts`). This design adapts that code rather than adding a
second writer. Separately, uncommitted plugin TypeScript facades
(`floom-plugin-*`) are out of scope and are left untouched.

The user also requires that **every API advertised in the declarations is one
the parser actually accepts** — a declaration that the TSX front end then
rejects is a bug, not just a missing feature.

## Goals

1. Generate a complete declaration file covering the whole parser-accepted
   surface: runtime imports, every widget component, every accepted prop, every
   emitted event, the `<App>` wrapper, and the global JSX namespace.
2. Make completion work for components the app has not used yet.
3. Enforce a parity contract: **declared ⊆ parser-accepted**. Tests read the
   *generated text*, build TSX from it, and compile that TSX through
   `floom-tsx`; any mismatch fails.
4. Wire generation into the normal `floom run` path (no extra command), and
   make an editor pick it up with zero configuration.

## Non-goals

- A runtime JavaScript package or npm publish.
- Per-file / per-document trimming of the declarations.
- Plugin TypeScript facades (`floom-plugin-*`).
- Changing widget behaviour merely to match a declaration. Where the emitter
  mounts a placeholder, the declaration says so; it does not claim the wiring
  exists.
- Non-canonical parser spellings (`Input`, `NavBar`, `<floom.Button>`, …). The
  declared surface uses canonical TSX names only; `declared ⊆ accepted` still
  holds.
- Implementing missing emitter wiring for placeholder kinds.

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

There is exactly **one** emission path:
`floom_tsx::generate_dts() -> String`, independent of a `UiDocument` (the
vocabulary is static). The build script writes its output; tests inspect the
same string. `floom_tsx::Generated` does not grow a second `types` field.

### `declare namespace Floom`

- `Accessor<T>`: callable `(): T`.
- `Setter<T>`: `(value: T): void`.
- `Component`: `() => JSX.Element`.
- `Color`: a hex template-literal union (`#rgb`, `#rrggbb`, `#rrggbbaa`). This
  is narrower than the parser (which also accepts the prefix-less form) and
  prevents `background="red"` from type-checking when the parser would reject
  it.
- One `{Kind}Props` interface per `NodeKind::ALL`, built from
  `prop_spec(kind, name)` over `all_prop_names()`:
  - `Bool` → `boolean` (optional field);
  - `Int` / `Number` → `number`;
  - `Text` → `string`; `Color` → `Floom.Color`;
  - `Ident` with a `domain` → a literal union (`"horizontal" | "vertical"`);
  - `Ident` without a domain → `string`;
  - `List` → `unknown[]`.
- `class?: string` on every kind (the lowerer accepts a `class` attribute).
- `children?: JSX.Element | JSX.Element[]` on containers and `Text`.
  - Raw JSX text (`<Text>hello</Text>`) is parser-accepted but deliberately
    **not** advertised: TypeScript cannot distinguish JSX text from
    `{someString}`, and the latter is rejected. Authors use `label=` instead.
    This is a narrowing, so `declared ⊆ accepted` still holds.
- Placeholder kinds (`Image`, `Grid`, `List`, `Overlay`, `Dialog`, `Popup`,
  `Tabs`, `Navigation*`) get a JSDoc line: the parser accepts the props and
  events, but the current emitter mounts the kind as a placeholder and does not
  wire them.
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
- The `<App>` root wrapper is included as `App: Floom.AppProps` with
  `title?: string`, `size?: string` (`"WxH"`), `width?: string`,
  `height?: string`, `children?: JSX.Element`. It is not a `NodeKind`; the
  parity test feeds it through `entry::resolve` instead.

### `declare module "floom"`

- `createSignal<T>(value: T): [Floom.Accessor<T>, Floom.Setter<T>]`
- `createMemo<T>(fn: () => T): Floom.Accessor<T>`
- `createEffect(fn: () => void): void`
- `provideContext<T>(name: string, value: T): void`
- `useContext<T = string>(name: string): T`
- `type Component = Floom.Component`
- `mount(component: Floom.Component): void`
- `app: { setName(name: string): void }`
- `window: { setTitle; setSize; setSize("WxH"); setResizable }`
- `platform: { require(capability: Capability, onUnsupported: Policy): void }`
- `Capability` / `Policy` literal unions matching `entry.rs` exactly.

## Parser-parity contract

**Invariant:** every kind, prop, event, and entry method named in the
declarations must be accepted by `floom-tsx`.

**Source of truth for tests: the rendered declaration string.** Tests parse
`generate_dts()`’s output (small helpers for `IntrinsicElements`, interface
fields, and module members), then feed the extracted names back into
`floom-tsx`. A field the emitter forgets therefore fails the test; reading the
schema tables directly would not catch that.

Enforced by tests, not by review:

1. **Tag round-trip** — every tag in `IntrinsicElements` (except `App`) satisfies
   `NodeKind::parse(tag) == Some(kind)`, and the entry count equals
   `NodeKind::ALL.len() + 1`. This catches the current `NativeWeb`-vs-`Web`
   class of bug.
2. **Prop acceptance** — every `{Kind}Props` field (except `class` /
   `children`) resolves through `prop_spec(kind, name)`.
3. **Event acceptance** — every `onXxx` field maps back through
   `EventName::parse(…)`, and an inline arrow handler using it lowers without
   error.
4. **Entry acceptance** — a TSX module calling every declared
   `app` / `window` / `platform` method resolves through `entry::resolve`, and
   `<App>` with every declared attribute resolves too.
5. **Shim acceptance** — a module using `createSignal`, `createMemo`,
   `createEffect`, `provideContext`, `useContext`, and `mount` compiles.
6. **Full-document round-trip** — the test renders one TSX element per kind
   using that kind’s declared props and events (sample values drawn from the
   schema domains, `#rrggbb` for colors), wraps them in a `<Stack>`, and runs
   `floom_tsx::compile` over the document. Success proves the declared surface
   is parseable end to end.
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

- The generated app build script writes
  `<app-dir>/.floom/types/floom.d.ts` on every compile. `.floom/` is already
  git-ignored, so this does not dirty the tree.
- `floom run`’s `write_project` creates `<app-dir>/tsconfig.json` **only when
  absent** (never rewrites a user file). Unlike `.floom/types/`, this is a
  normal project file intended to be committed and edited:

  ```json
  {
    "compilerOptions": {
      "target": "ES2020",
      "module": "ESNext",
      "moduleResolution": "Bundler",
      "jsx": "preserve",
      "strict": true,
      "noEmit": true
    },
    "include": [".floom/types/**/*.d.ts", "*.tsx", "*.ts"]
  }
  ```

  `include` is deliberately non-recursive: `floom run` compiles a single app
  file, so pulling in unrelated TypeScript would only create false errors.
  `skipLibCheck` is omitted so the generated `.d.ts` itself is checked.
- `examples/floom-tsx-app/README.md` documents the one-time generation.
- The existing working-tree code that writes `<app>.d.ts` beside the app is
  replaced by the destination above; there is no second path.
- `write_project` tests use a per-test temporary app directory (not
  `/tmp/app.tsx`), plus tests for: the create-only-when-absent rule, and the
  `.floom/types/floom.d.ts` build-script destination.

## Testing

- Rust unit/golden tests for the generated text (module block, JSX block,
  literal unions, event signatures, native names, `App` wrapper).
- Parity tests 1–7 above.
- A `tsc` smoke test runs only when `tsc` is on `PATH` **and** is version 5 or
  newer (`moduleResolution: "Bundler"`); otherwise it prints a skip reason.
  CI must not depend on Node.
- Manual end-to-end: `cargo run -p floom-cli -- run examples/floom-tsx-app/app.tsx --build-only`,
  then `npx tsc --noEmit -p examples/floom-tsx-app` → zero errors.
- User check: open `app.tsx` in the editor and confirm completion for an
  unused component and for `onChange` on `Slider`.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| `tsc` absent or older than 5 in CI | Guarded test, skip with a message |
| Build script writes into the source tree | Only under the ignored `.floom/types/` |
| `tsconfig.json` creation surprises a user | Created only when absent; documented as a normal project file |
| Generated file drifts from schema | Generated from the same tables the parser validates against; parity tests read the generated text |
| Two competing `.d.ts` locations | Existing WIP adapted to a single path |
| `strict` flags the example app | Run `tsc` against the example; narrow declarations rather than adopting `any` |
| Value-level gaps TS cannot express | `Color` narrowed to hex literals; raw text children intentionally not advertised |
