# floom-demo — the component gallery

A runnable gallery of **every one of the 98 components** the `floom-ui`
satellite provides, each shown in **Material, Cupertino and Fluent side by
side**, with live samples and the component's own API.

```bash
cargo run -p floom-demo            # the gallery, in a window
cargo run -p floom-demo -- --headless   # mounts every page, prints a painted count, exits
cargo test -p floom-demo            # the headless gate: every page, every sample, every interaction
```

## What the three columns mean

Each sample is written **once** and shown **three times** — once per design
system, each column carrying that design system's real tokens. The columns are
the same component, not three different components:

| Column | Design system | Comes from |
|---|---|---|
| Material | Material 3 | `floom-material` (one crate, per-component features) |
| Cupertino | Cupertino / HIG | `floom-cupertino` (one crate, per-component features) |
| Fluent | Fluent | `floom-fluent` (one crate, per-component features) |

The gallery holds all three in **one binary**, so switching between them is
switching between columns rather than between builds. The theme each column
carries is pinned to its design system (`MultiTheme::pinned`), which is why the
columns look different even though the sample body is the same code.

## What a page holds

Every component has a page, and a page holds:

- **an introduction** — one line on what the component is for;
- **live samples** — at least two, each a working example, with a note beside
  the ones that need one (a component that draws nothing of its own, an
  overlay that has to be revealed, a resting state that only changes on a
  hover);
- **a props table** — parsed at run time out of `floom-core-theme`'s own
  props sources, so it cannot drift from the struct it describes;
- **an events table** — what the component fires, and with what;
- **a methods table** — the builders it exposes and the node it resolves to.

## The gate

`cargo test -p floom-demo` is the gallery's contract with the library:

| Test file | What it holds to account |
|---|---|
| `tests/generated.rs` | the generator emits one alias per component, per design system |
| `tests/registry.rs` | the catalogue holds 98 pages, one per component, none duplicated |
| `tests/categories.rs` | every component has samples, states what it does, and says so when it draws nothing |
| `tests/props.rs` | every props table matches the library's own props struct, field for field |
| `tests/pages.rs` | every page, and every sample on it, mounts and paints |
| `tests/interaction.rs` | pressing a button reaches its handler; ticking a checkbox reaches its handler |
| `tests/shell.rs` | the two-pane shell mounts and lists every page |
| `tests/compare.rs` | the compare row shows three columns, including for a terminal that takes arguments |

## Adding a component

The gallery derives what it can from the satellite, so adding a component is
three steps.

1. **Regenerate.** `python3 examples/floom-demo/tools/gen.py` re-reads the
   satellite's kit crates and rewrites `Cargo.toml`, `src/ds.rs` and
   `src/props_sources.rs`. A component added to all three design systems
   appears in all three alias modules with no hand editing; one added to only
   some is skipped, with a note saying so.
2. **Write the page.** Add the component to the `COMPONENTS` list of its
   category module and write its page: an introduction, two samples, and the
   events and methods tables. The samples are written once and shown three
   times with `compare_children!`:

   ```rust
   let (m, c, f) = compare_children!(
       ds::material::Button,
       ds::cupertino::Button,
       ds::fluent::Button,
       |b, theme| b::new().with_theme(theme).label("Click me").view()
   );
   Box::new(compare_row((m, c, f)))
   ```

   The body is instantiated three times, each with `$b` bound to that design
   system's type and `$theme` pinned to it — so the three columns are the same
   code under three sets of tokens, and a builder that one design system spells
   differently fails to compile rather than quietly showing two columns.
3. **Register it.** Add the page to its category module's `pages()`, and add
   the component's name to the right category's list in `tests/categories.rs`
   so the category contract covers it.

Then `cargo test -p floom-demo`.

## Layout

```
examples/floom-demo/
├── Cargo.toml            generated — 294 kit path dependencies
├── tools/gen.py          the generator
└── src/
    ├── main.rs           the entry point, and the --headless check
    ├── api.rs            ApiRow / ApiDoc — one row of a table
    ├── ds.rs             generated — the per-design-system component types
    ├── macros.rs         compare_children!, compare_with!, themed_widget!
    ├── pages/            the six categories, one module each
    ├── probe.rs          the read-out the interaction tests watch
    ├── props_sources.rs  generated — the library's own props sources
    ├── props_src.rs      the parser that turns them into tables
    ├── registry.rs       the catalogue of 98 pages
    ├── shell.rs          the two-pane shell
    └── widgets.rs        the chrome: labels, tables, compare rows
```

## From the satellite

The gallery lives in the **root** workspace, not in the satellite's, so the
satellite stays headless. From the satellite directory:

```bash
just demo           # the gallery, in a window
just demo-headless  # mounts every page, prints a painted count, exits
```
