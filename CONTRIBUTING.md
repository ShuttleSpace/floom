# Contributing

Floom aims to be a high performance UI library. New contributions are always welcome!

## Repository layout

The workspace root is a **virtual** manifest (no `[package]`); every crate lives
under `crates/`, flattened — one directory per crate, named after the package:

```
crates/
  floom/                 the main `floom` library crate
  floom-<name>/          one directory per crate (core, runtime, render, ...)
  vendor/                forked upstream trees (separate workspaces)
  ecosystem/             satellite projects: floom-ui, floom-editor, flameo
benches/  docs/  examples/  tests/  scripts/  notices/
```

`benches/` and `tests/` belong to the `floom` package but intentionally stay at
the repository top level, so `crates/floom/Cargo.toml` declares them with
explicit `path = "../../benches/..."` / `"../../tests/..."` targets.

`crates/vendor/` and `crates/ecosystem/` are listed in the workspace `exclude`:
each is its own nested workspace, so `cargo test --workspace` stays focused on
the core.

Two naming rules are deliberate exceptions, because they name artifacts we do not
own and Cargo matches them against an external source:

- `crates/vendor/floem-winit/` holds a fork of `lapce/winit`, whose packages are
  genuinely named `floem-winit`, `floem-winit-core`, and so on. The git
  dependency uses `package = "floem-winit"` and
  `[patch."https://github.com/lapce/winit"]` keys on that exact name, so
  renaming either would silently stop the patch from applying.
- The legacy `floem-vger` renderer backend is the published `floem-vger` crate on
  crates.io, referenced with `package = "floem-vger"`.

The best way to start contributing to Floom is to learn how it works by looking at the examples. If you think there are more examples that can be added, feel free to send in a PR!

Alternatively, you can have a look at issues to see if there is anything you'd like to pick up.

Filing bugs with clear, reproducible steps is a great way to contribute as well.

It's not a good library if there isn't good documentation. So, all contributions to the documentation are very appreciated!

If you would like a conversation with the developers of Floom, you can join in the #floom channel on this [Discord](https://discord.gg/RB6cRYerXX).
