# Working in floom

floom is a cross-platform GUI library for Rust: one widget tree, one plugin
surface, self-drawn on wgpu with no web view in the loop. The goal and the four
criteria that decide whether a change helps are in
[`docs/vision.md`](docs/vision.md); the current state of every ledger item is in
[`docs/progress.md`](docs/progress.md). Read the ledger before starting work and
update it when you finish.

## Language

Code, comments, rustdoc, commit messages and this file are written in English.
User-facing documents under `docs/` are Chinese, because that is the language
their reader uses — do not translate them to match this file.

## Layout

Seven layers, one job each: `core` (types, signals, layout, scene),
`system` (animation, accuracy), `widgets`, `platform`, `render`, `native`,
`devtools`, `frontend` (DSL/TSX), `app` (the `floom` façade and CLI).
`crates/ecosystem` holds three **separate workspaces** (`floom-ui`,
`floom-editor`, `plugins`) that the root workspace does not build.

`crates/vendor` holds upstream forks; keep their layout and formatting so a
re-sync stays a copy rather than a merge.

A module is `src/name.rs`, never `src/name/mod.rs`. `just gate module-paths`
enforces it, because both spellings compile and nothing else would notice the
tree drifting back to two conventions.

## Style

- Take the shortest diff that fully works, then make it read like the file it
  lands in. No wrapper layers, no one-off directories, no `*_impl` shadow APIs.
- Comments state the reason, or the reason a thing is not done yet. Three lines
  is the budget. Delete any comment that restates the code.
- A colour in a design-system widget comes from that crate's `tokens.rs`;
  `just gate style` fails on literals.
- Absolute values that only hold on one machine belong in a `shortcut:` comment
  with what would replace them.
- `crates/app/floom` is `#![forbid(unsafe_code)]`. Raw pointers, FFI structs and
  `unsafe impl Send` live in the platform crate that owns the pointer, behind a
  safe wrapper; `#[unsafe(no_mangle)]` in the façade is fine.

## Tests and gates

New behaviour gets a test that fails without it; a new branch, loop, parser or
FFI entry point is never "obviously fine". Prove a gate can go red before
trusting it green — mutate the code it guards and watch it fail.

`scripts/gate.sh` (or `just gate`) is the real gate. CI runs four workflows:
`ci-core` (three-OS `cargo test`, clippy, MSRV, fmt, typos, deny, architecture,
module paths), `ci-platform` (android/iOS/OHOS cross checks and the device
probe), `ci-plugins`, and `docs`.

CI green is a claim about the commands it ran, not about the product: read
"What CI actually proves" in [`docs/developer-experience.md`](docs/developer-experience.md)
before claiming a feature works. `cargo check` is not a test run, a
cross-target `check` never executes a line, and a test that needs a feature the
job does not enable has not run.

## Standing rules

1. Every CI claim must be true and checkable: name the command, the target and
   the evidence, or say it is unverified. "Passed" for something that never ran
   is the one failure mode this project cannot afford.
2. When touching code, align it with the surrounding conventions (Google
   Flutter is the reference for layout of files, tests and comments): one file
   one responsibility, clean directory, comments that explain rather than
   narrate. A diff should leave the repository tidier than it found it.
