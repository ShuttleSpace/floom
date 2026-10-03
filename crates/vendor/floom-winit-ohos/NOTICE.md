# NOTICE — floom-winit-ohos (forked third-party crate)

## Upstream source

- **Project:** [taoyuan11/tgui-winit-ohos](https://github.com/taoyuan11/tgui-winit-ohos)
  (published as `tgui-winit-ohos` 0.4.11 on crates.io; the URL is taken from the
  published crate's own `homepage` / `repository` fields)
- **Forked version:** 0.4.11
- **Upstream license:** Apache-2.0 (per the published crate)

## Why it is vendored

`tgui-winit-ohos` 0.4.11 depends on upstream `winit-core` 0.31.0-beta.2, whose
`ApplicationHandler` is a *different* trait from the one Floom's `Application`
implements (`floem-winit-core`). The two don't unify, so Floom's
`export_ohos_winit_app!(floom_app_factory)` cannot compile until both sides share
one `ApplicationHandler` definition.

## Local delta

Two renames, no logic change:

1. **Retarget (2026-08-31).** `winit_core -> floem_winit_core`, so the trait
   Floom implements and the trait this crate requires are the same trait. The
   fork's `floem-winit-core` git rev matches Floom's `floem-winit` git rev
   exactly, so the trait is byte-identical. Reproducible rename lives in
   `patch/retarget-to-floem-winit-core.patch`; `patch/README.md` carries the
   drift analysis.
2. **Fork rename (2026-09-18).** The tree was renamed off the `tgui` namespace
   so that the vendored fork is distinguishable from its upstream:
   - package `tgui-winit-ohos` → `floom-winit-ohos`
   - lib `tgui_winit_ohos` → `floom_winit_ohos`
   - the runtime log tag string (`DEFAULT_LOG_TAG`, the worker-panic message and
     the event-loop name) follows the lib name
   - `Cargo.toml` keeps its upstream `homepage` / `repository` / `documentation`
     fields verbatim; `Cargo.toml.orig` is the untouched upstream manifest record

   Because a `[patch.crates-io]` entry must carry the name of the package it
   replaces, the rename also turned Floom's wiring into a plain path dependency:
   `floom-winit-ohos = { path = "crates/vendor/floom-winit-ohos" }` in
   `[workspace.dependencies]`.

Nothing under `src/` other than the identifiers above was touched — this is not
a port, and `patch/README.md`'s conclusion (the retarget is mechanical) still
holds for the rename.

## Exit criteria

Delete this tree and the `floom-winit-ohos` entry in the root
`[workspace.dependencies]` once `tgui-winit-ohos` depends on `floem-winit-core`
directly and the OHOS example gate passes against the published crate.

## See also

- `docs/ohos.md` — retarget notes and build steps.
- `patch/README.md` — what the retarget rename touches and why it is safe.
