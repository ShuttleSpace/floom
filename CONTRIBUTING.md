<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Contributing

Floom aims to be a high performance UI library. New contributions are always welcome!

## Repository layout

The workspace root is a **virtual** manifest (no `[package]`); every crate lives
under `crates/`, flattened — one directory per crate, named after the package:

```
crates/
  floom/                 the public facade crate (its tests live in crates/app/floom/tests/)
  floom-<name>/          one directory per crate (core, runtime, render, ...)
  vendor/                forked upstream trees (separate workspaces)
  ecosystem/             satellite projects: floom-ui, floom-editor, plugins
docs/  examples/  scripts/  tests/
```

The `floom` facade keeps its integration tests inside the crate
(`crates/app/floom/tests/`), so they are picked up by normal Cargo discovery. The
top-level `tests/` directory holds CLI test fixtures, not the library's test
suite.

`crates/vendor/` and `crates/ecosystem/` are listed in the workspace `exclude`:
each is its own nested workspace, so `cargo test --workspace` stays focused on
the core. The `floom-editor` satellite is a single crate at
`crates/ecosystem/floom-editor/floom-editor` (model as `floom_editor::core`,
view as `text_editor()` / `TextEditor`); test it from that nested workspace with
`cargo test --manifest-path crates/ecosystem/floom-editor/Cargo.toml`.

Two naming rules are deliberate exceptions, because they name artifacts we do not
own and Cargo matches them against an external source:

- `crates/vendor/floem-winit/` holds a fork of `lapce/winit`, whose packages are
  genuinely named `floem-winit`, `floem-winit-core`, and so on. The git
  dependency uses `package = "floem-winit"` and
  `[patch."https://github.com/lapce/winit"]` keys on that exact name, so
  renaming either would silently stop the patch from applying.
- The legacy `floem-vger` renderer backend is the published `floem-vger` crate on
  crates.io, referenced with `package = "floem-vger"`.

## Toolchain

The dev toolchain is pinned and provisioned with [mise](https://mise.jdx.dev).
Once mise is installed, run it once to fetch the exact Rust version the project
targets:

```bash
mise install
```

`mise` then selects `rust@1.99.1` automatically whenever you enter this
repository, so plain `cargo`/`rustup` invocations use the right compiler without
manual switching. The version here mirrors `channel` in
[`rust-toolchain.toml`](rust-toolchain.toml); the two must stay in sync.
`rust-toolchain.toml` remains the authoritative pin for CI and for `cargo`
invocations, while mise owns provisioning on contributor machines. (Other
language runtimes such as Node/Python are provisioned by your global
`~/.config/mise/config.toml` when a satellite crate needs them.)

If mise complains that `mise.toml` is not trusted, run `mise trust mise.toml`
once — mise refuses to read untrusted config files by design.

## Verifying your changes

Before opening a PR, run the checks CI will run. The toolchain is provisioned by
mise (see above) and pinned in `rust-toolchain.toml`, so `cargo` uses
`rust@1.99.1` automatically.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets -- --deny warnings
cargo test --workspace
```

Platform- and feature-gated code is easy to miss, so when you touch a backend
behind a `cfg` or a feature flag, compile that target explicitly (for example
`cargo check --features desktop` or the Android/iOS `--target`), because the host
build cannot see it.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org): a type, an
optional scope, and a concise summary.

```
feat(widgets): add a virtualized grid widget
fix(renderer-vello): honour the clip rect on text runs
docs: describe the embedder API
refactor!: drop the legacy view adapter
```

Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, `perf`, `build`, `ci`.
Mark breaking changes with `!` after the type (or a `BREAKING CHANGE:` footer).
Keep each commit focused on one logical change.

## Pull requests

Keep PRs small and focused on a single logical change — the same discipline as the
commit messages above. A PR that touches one concern is faster to review, safer to
merge, and easier to bisect or revert than a catch-all branch.

The project merges with **squash**: every commit on the PR collapses into one
commit on `main`, and the PR title becomes that commit's subject line. So:

- Write the PR title as a [Conventional Commits](https://www.conventionalcommits.org)
  summary (`feat(widgets): …`, `fix(renderer-vello): …`, …). It is the commit
  message a reader of `git log` will see.
- Keep the PR body self-contained — it becomes the squashed commit's body.
- Make the branch's commits follow Conventional Commits too. The squash keeps only
  the title, but a clean branch is easier to review and rebase.

Before requesting review, run the checks under
[Verifying your changes](#verifying-your-changes) — `cargo fmt --all --check`,
`cargo clippy … --deny warnings`, `cargo test --workspace`, and the layered
architecture gate in `scripts/check-architecture-deps.sh`. CI runs the same suite,
and a green PR is reviewed sooner.

If your change spans the layered architecture, keep the `LAYERS` table in
`docs/architecture.md` and the gate in `scripts/check-architecture-deps.sh` in
sync — `crates/app/floom/tests/docs_parity.rs` fails the build if they drift.

The best way to start contributing to Floom is to learn how it works by looking at the examples. If you think there are more examples that can be added, feel free to send in a PR!

Alternatively, you can have a look at issues to see if there is anything you'd like to pick up.

Filing bugs with clear, reproducible steps is a great way to contribute as well.

It's not a good library if there isn't good documentation. So, all contributions to the documentation are very appreciated!

If you would like a conversation with the developers of Floom, you can join in the #floom channel on this [Discord](https://discord.gg/RB6cRYerXX).
