# notices

Third-party license compliance, lore-style. This directory holds the
configuration for [`cargo-deny`](https://embarkstudios.github.io/cargo-deny/) and
[`cargo-about`](https://embarkstudios.github.io/cargo-about/), used to audit the
dependency graph and generate a `THIRD-PARTY` notice.

## Files

- `deny.toml` — advisory / license / ban / source checks.
- `about.toml` — which licenses are accepted when generating the notice.
- `manual-additions.toml` — license annotations for Floem's in-tree forks
  (under `vendor/`), which are not on crates.io.

## Forked / vendored crates

Floem vendors third-party forks under [`vendor/`](../vendor/). Each fork carries a
`NOTICE.md` documenting its upstream source and the local delta:

- `vendor/floem-winit` — fork of [lapce/winit](https://github.com/lapce/winit)
  (iOS open-URL forwarding).
- `vendor/tgui-winit-ohos` — fork of `tgui-winit-ohos` 0.4.11 (OHOS
  `floem-winit-core` retarget).

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
