# NOTICE — tgui-winit-ohos (forked third-party crate)

## Upstream source

- **Project:** [tgui-winit-ohos](https://github.com/tgui-rs/tgui-winit-ohos)
  (published as `tgui-winit-ohos` 0.4.11 on crates.io)
- **Forked version:** 0.4.11
- **Upstream license:** see the published crate (Apache-2.0 OR MIT)

## Why it is vendored

`tgui-winit-ohos` 0.4.11 depends on upstream `winit-core` 0.31.0-beta.2, whose
`ApplicationHandler` is a *different* trait from the one Floem's `Application`
implements (`floem-winit-core`). The two don't unify, so Floem's
`export_ohos_winit_app!(floem_app_factory)` cannot compile until both sides share
one `ApplicationHandler` definition.

## Local delta

- Pure crate rename `winit_core -> floem_winit_core` (zero logic change).
- The fork's `floem-winit-core` git rev matches Floem's `floem-winit` git rev
  exactly, so the trait is byte-identical.
- Reproducible rename lives in `patch/retarget-to-floem-winit-core.patch`; see
  `patch/README.md` for the drift analysis.

## Exit criteria

Delete this tree (and the `[patch.crates-io]` entry in the root `Cargo.toml`) once
tgui-winit-ohos depends on `floem-winit-core` directly and the OHOS example gate
passes against the published crate.

## See also

- `docs/ohos.md` — retarget notes and build steps.
- `patch/README.md` — what the rename touches and why it is safe.
