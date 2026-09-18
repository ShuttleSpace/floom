# Migration from floem_packages

As of 2026-08-31, Floem vendors live in-tree. They were first imported under
`platform-compat/` and, on 2026-09-17, relocated to **`vendor/`** — the name used
by the [EpicGames/lore](https://github.com/EpicGames/lore) workspace model that
this tree now follows (see `docs/workspace-conventions.md`). Each fork carries a
`NOTICE.md` recording its upstream, local delta, and exit criterion; the
`platform-compat/` name no longer exists.

Do not use `/Users/humphrey/Project/Rust/floem_packages` for new work.
Safe to archive or delete that directory after confirming:
- `cargo check -p floem` works
- iOS/OHOS example docs build steps use `vendor/` paths
