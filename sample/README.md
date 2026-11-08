# floom-plugin-sample

A Floom plugin. Two halves, one binary:

| | where | who calls it |
|---|---|---|
| Rust | `src/lib.rs` | the app |
| native | `native/<platform>/` | `build.rs`, at compile time |

The Rust half calls the native half through `extern "C"` directly. There
is no method channel and no serialization: a call from the app is a
machine call, and it is resolved when the crate is linked rather than
looked up at runtime.

## Platforms

* `native/macos/` — macOS

## Using it

```toml
[dependencies]
floom-plugin-{name} = {{ path = "../plugins/{name}" }}
```

```rust,ignore
floom_plugin_{name}::register();
floom_platform::plugin("{name}").enable()?;
```

## Example

`example/` is a runnable app that depends on this plugin:

```text
cd example && cargo run
```

## Add a platform later

```text
floom create --plugin --platforms ios .
```
