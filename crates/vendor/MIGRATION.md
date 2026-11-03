# Migration from floom_packages

As of 2026-08-31, Floom vendors live in-tree. They were first imported under
`platform-compat/` and, on 2026-09-17, relocated to **`crates/vendor/`** — the name used
by the [EpicGames/lore](https://github.com/EpicGames/lore) workspace model that
this tree now follows (see `docs/workspace-conventions.md`). Each fork carries a
`NOTICE.md` recording its upstream, local delta, and exit criterion; the
`platform-compat/` name no longer exists.

Do not use `/Users/humphrey/Project/Rust/floom_packages` for new work.
Safe to archive or delete that directory after confirming:
- `cargo check -p floom` works
- iOS/OHOS example docs build steps use `crates/vendor/` paths
