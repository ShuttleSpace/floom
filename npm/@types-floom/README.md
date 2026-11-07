<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->
# `@types/floom`

TypeScript declarations for writing Floom applications in TSX. They give the
editor completion and type checking for `<Stack>`, `<Text>`, the event
handlers, and the `import … from "floom"` runtime surface.

## Editor-only, not a runtime dependency

`index.d.ts` is consumed by the **TypeScript compiler / IDE only**. `floom run`
does not read it and does not need it to build an app; the TSX front end
(`floom-tsx`) parses the source on its own.

```sh
npm install --save-dev @types/floom
```

`@types/floom` is auto-included from `node_modules/@types`, so a project that
already has a `tsconfig.json` needs no `types` entry.

## Required tsconfig setting

```jsonc
{
  "compilerOptions": {
    "jsx": "preserve",
    "lib": ["ES2020"] // no DOM: `Text` / `Image` are Floom widgets here
  }
}
```

`lib.dom` declares `Text` and `Image` constructors, which collide with the
`Text` / `Image` Floom components. Leave the DOM lib out (a Floom app has no
`document` anyway). `Map` is an ES2015 built-in, so the declarations extend
`MapConstructor` instead of redeclaring the variable. An app's own
`function App()` shadows the ambient `<App>` wrapper locally, which is how the
parser's root wrapper still type-checks.

## Generated — do not edit by hand

`index.d.ts` is produced by `floom_tsx::generate_dts()` from the IR schema
tables, so it always matches what the parser accepts. Regenerate after any
schema/API change:

```sh
SYNC_DTS=1 rustup run 1.99.1 cargo test -p floom-tsx --test dts_sync
```

`cargo test -p floom-tsx` fails when the committed artifact drifts from the
generator, and when `package.json`'s version does not match the `floom-tsx`
crate version. Both are kept in step in the same commit as the API change.

## Publishing

Version, publish, and tag together:

```sh
# 1. bump [workspace].version in Cargo.toml
# 2. sync npm/@types-floom/package.json to the same version
# 3. regenerate index.d.ts (see above)
npm publish ./npm/@types-floom
```

Publishing needs npm credentials for the `@types` scope and is done by a
maintainer, not by the build.
