<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# notices

Third-party license compliance, lore-style. This directory holds the
configuration for [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) and
[`cargo-about`](https://embarkstudios.github.io/cargo-about/), used to audit the
dependency graph and generate a `THIRD-PARTY` notice.

## Files

- `deny.toml` — advisory / license / ban / source checks.
- `about.toml` — which licenses are accepted when generating the notice.
- `manual-additions.toml` — license annotations for Floom's in-tree forks
  (under `crates/vendor/`), which are not on crates.io.

## Forked / vendored crates

Floom vendors third-party forks under [`crates/vendor/`](../vendor/). Each fork carries a
`NOTICE.md` documenting its upstream source and the local delta:

- `crates/vendor/floem-winit` — fork of [lapce/winit](https://github.com/lapce/winit)
  (iOS open-URL forwarding).
- `crates/vendor/floom-winit-ohos` — fork of `tgui-winit-ohos` 0.4.11 (OHOS
  `floem-winit-core` retarget).
- `crates/vendor/harmony-app-rs` — patches for `taoyuan11/harmony-app-rs`
  (`cargo-ohos-app`) so the OHOS packaging flow works on macOS/Linux.

## Generating the notice

```sh
cargo install cargo-deny cargo-about

# Audit the graph
cargo deny --config notices/deny.toml check

# Produce THIRD-PARTY.html
cargo about generate --config notices/about.toml -o THIRD-PARTY.html
```

`cargo-deny` and `cargo-about` are not installed in the default environment; the
configs are committed so the audit is one install away.
