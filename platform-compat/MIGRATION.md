# Migration from floem_packages

As of 2026-08-31, Floem vendors live in-tree under `platform-compat/`.

Do not use `/Users/humphrey/Project/Rust/floem_packages` for new work.
Safe to archive or delete that directory after confirming:
- `cargo check -p floem` works
- iOS/OHOS example docs build steps use `platform-compat/` paths
