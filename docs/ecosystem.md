# Floem ecosystem

Official satellites (not part of the floem 1.0 core promise).

They live **inside this monorepo** under `ecosystem/`, each as a nested
workspace. The directory is listed in the root `Cargo.toml`'s `exclude`, so the
core `cargo test --workspace` stays focused on the 2.0 crates; build a satellite
by running cargo from its own directory.

| Satellite | Role | Depends on |
|-----------|------|------------|
| [`ecosystem/floem-ui`](../ecosystem/floem-ui) | Material / Cupertino / Fluent kit (`floem-elements`), ~350 crates | `floem` public API (path, `[patch]`ed to this checkout) |
| [`ecosystem/floem-editor`](../ecosystem/floem-editor) | Code/text editor views + `floem-editor-core` | `floem` public API (path) |
| [`ecosystem/flameo`](../ecosystem/flameo) | Demo application exercising the floem-ui kit | `floem` + the floem-ui crates |

`floem-ui` was spelled **`flome-ui`** (a typo) until it moved in here.

> **Nested-workspace caveat.** A `[patch]` only applies to the workspace that
> declares it, so `floem-editor` and `flameo` re-declare the lapce/winit and
> lapce/floem patches that the root `Cargo.toml` carries. Without that, a path
> dependency on `floem` would resolve `floem-winit-core` / `dpi` from git while
> the rest of the graph uses the vendored copies — two instances of one type.
>
> Two packages are named `floem-editor`: the 2.0 crate at `text/floem-editor`
> and the satellite at `ecosystem/floem-editor/floem-editor`. They never share a
> resolve graph as *members* (the satellite is excluded), but
> `examples/{custom_keymap,widget-gallery}` pull the satellite in by path, so
> the root graph does contain both. `scripts/check-architecture-deps.sh`
> resolves nodes among `workspace_members` first for exactly this reason.

## Compatibility matrix

| floem | floem-ui | floem-editor | Notes |
|-------|----------|--------------|-------|
| 0.2.x (pre-extract) | path/git | in-tree `editor` feature (removed) | historical |
| 0.2.x (industrial-1.0) | path/git | separate `floem-editor` repo | editor extracted |
| 1.0.0 | (fill on release) | (fill on release) | |
| monorepo (2026-09-18) | [`ecosystem/floem-ui`](../ecosystem/floem-ui) | [`ecosystem/floem-editor`](../ecosystem/floem-editor) | both imported into this repo |

Update this table when tagging floem or publishing satellites.
