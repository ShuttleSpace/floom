# A Floom app written entirely in TSX

`app.tsx` is the whole application. There is no `Cargo.toml`, no `main.rs`, and
no build script — the Floom toolchain owns the build.

```sh
# from the Floom checkout
cargo run -p floom-cli -- run examples/floom-tsx-app/app.tsx

# or, with the CLI installed
floom run app.tsx
```

## What happens

1. `floom run` writes a throwaway crate under `.floom/run/`.
2. That crate's `build.rs` compiles `app.tsx` with `floom-tsx`:
   TSX → `floom_ui_ir::UiDocument` → `floom_dsl::emit` → Rust.
3. Cargo builds the crate, and the generated `main.rs` mounts the emitted tree
   through `floom::run`, the single platform entry point.
4. The binary runs in whichever backend the build enabled
   (`desktop,renderer-gpu` by default).

## Declaring the window

The root `<App>` wrapper is not a widget: the compiler unwraps it and reads the
window from its attributes. Everything inside it is the widget tree.

```tsx
<App title="Counter" size="640x420">
  <Stack axis="vertical" gap={16} padding={24}>…</Stack>
</App>
```

Without the wrapper the app still compiles, with the default window
(`floom`, 480×320).

## Useful flags

| Flag | Effect |
|---|---|
| `--no-run` | Compile the generated crate without launching it |
| `--release` | Build in release mode |
| `--features <LIST>` | Backend bundle (default `desktop,renderer-gpu`) |
| `--floom-path <PATH>` | Use a local Floom checkout instead of reinstalling |

`.floom/` is generated output and should be git-ignored.
