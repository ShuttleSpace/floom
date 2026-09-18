# tgui-winit-ohos → floem-winit-core retarget patch

Fork of `tgui-winit-ohos` (v0.4.11) with its `winit-core` dependency retargeted onto
`floem-winit-core` (the core of lapce's `floem-winit` fork used by floem).

## Why this is needed

floem's `Application` implements `floem-winit-core`'s `ApplicationHandler`.
`tgui-winit-ohos`'s `export_ohos_winit_app!` factory requires
`RuntimeApplication = winit_core::ApplicationHandler + Send` (the **upstream**
`winit_core` crate). Those are two distinct crates, so `Application` does **not**
satisfy `RuntimeApplication` and floem's `export_ohos_winit_app!(floem_app_factory)`
will not compile — until both sides share one `ApplicationHandler` definition.

This patch makes tgui depend on `floem-winit-core` instead of `winit-core`, so the
trait floem implements and the trait tgui requires become the *same* trait.

## Drift quantification (the actual finding)

Comparing `winit-core 0.31.0-beta.2` (registry) with `floem-winit-core`
(`lapce/winit` @ `133268de675a4fda70816d78a7a1435eb972fea1`, git checkout):

- **Only 6 of ~17 source files differ.** `application/mod.rs` (the `ApplicationHandler`
  trait) is **byte-identical** — not among the changed files.
- The 6 changed files: `error.rs`, `event_loop/mod.rs`, `event_loop/run_on_demand.rs`,
  `event.rs`, `keyboard.rs`, `window.rs`. Of these:
  - `error.rs`, `event_loop/mod.rs`, `run_on_demand.rs`: cosmetic only
    (brace formatting, import reorder, doc comments, removal of internal
    `never_return`/`register` modules that tgui does not use).
  - `keyboard.rs`: private `eq` impls only — no public API change.
  - `window.rs`: ~100 lines of change, but almost entirely `#[cfg(test)]`
    `assert!` reformatting plus one internal IME method rename
    (`with_hint_and_purpose` → `without_hint_and_purpose`) inside `window.rs`'s own
    impl — **not** called by tgui.
  - `event.rs`: the only real public-API delta —
    `MouseButton` lost `#[repr(u8)]` + `Button6..=Button32` (now `Other(u16)`),
    and `ButtonSource::mouse_button()` returns `MouseButton` instead of
    `Option<MouseButton>`.

- **tgui does not touch any of the changed APIs.** It only ever constructs
  `MouseButton::{Left,Right,Middle,Back,Forward}` (all preserved), never calls
  `mouse_button()`, never uses `TabletToolButton`/`try_from_u8`/`Button6..32`, and
  never calls the renamed IME methods. `floem-winit-core` exports the same modules
  tgui imports (`application`, `cursor`, `error`, `event`, `event_loop`, `keyboard`,
  `monitor`, `window`, `icon`), each with the items tgui uses.

**Conclusion: the retarget is mechanical — a crate rename, not a code port.
Zero logic changes are required in tgui's OHOS backend.**

## What the patch does

- `Cargo.toml`: `[dependencies.winit-core] = "0.31.0-beta.2"` →
  `[dependencies.floem-winit-core]` pointing at the lapce git repo; the `serde`
  feature reference is updated to `floem-winit-core/serde`.
- `src/{event_loop,ffi,keycodes,lib}.rs`: every `winit_core::` path becomes
  `floem_winit_core::` (20 occurrences).

`src/app.rs` and `src/log.rs` contain no `winit_core` references, so they are
untouched.

## Applying on a fork

From the root of your `tgui-winit-ohos` fork checkout:

```sh
patch -p1 < retarget-to-floem-winit-core.patch
```

Or, equivalently, do it by hand:

```sh
# 1) rename the crate everywhere in source
find src -name '*.rs' -print0 | xargs -0 sed -i '' 's/winit_core/floem_winit_core/g'
# 2) edit Cargo.toml: replace the winit-core dependency block with:
#    [dependencies.floem-winit-core]
#    git = "https://github.com/lapce/winit"
#    rev = "133268de675a4fda70816d78a7a1435eb972fea1"
#    package = "floem-winit-core"
#    and change `serde = ["winit-core/serde"]` -> `serde = ["floem-winit-core/serde"]`
```

> The patch was generated against the crates.io-normalized `Cargo.toml`. If `patch`
> rejects the `Cargo.toml` hunk on a fresh fork, apply step 2 manually — the `.rs`
> hunks apply regardless because the `.rs` files are byte-identical between the
> registry copy and the GitHub source.

## Verification (requires the OHOS toolchain)

This patch cannot be compiled on the host (macOS): tgui-winit-ohos is OHOS-only.
Verify with:

```sh
# in the floem workspace, after pointing floem's tgui-winit-ohos dep at your fork
cargo check -p floem --target aarch64-unknown-linux-ohos
```

Prerequisites for a full build/run: complete DevEco Studio + OpenHarmony SDK, the
macOS cross-platform patch for `cargo-ohos-app` (see the separate
`harmony-app-rs-patch` deliverable), and the N-API bridges for the still-`false`
capabilities (clipboard / file_dialog / open_external / receive_files /
accessibility).
