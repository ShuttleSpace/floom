<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floom Authors -->

# floom-plugin-mdns

Discover services on the local network (mDNS / DNS-SD).

A capability plugin: one capability, one crate, opt-in by dependency. The SDK
ships none of these; an app that wants this capability adds the crate, and an
app that does not does not link it.

## Platforms

| Target | Arm | What it does |
|---|---|---|
| all targets | `src/lib.rs` | real: pure-Rust `mdns-sd`, no native half |

Where an arm is not written, the API still compiles and returns a named
refusal (`Err`), or the honest inert value (`None`/`false`/a default). It never
returns a value it did not measure.

## API

- `pub struct Service { pub fullname: String, pub service_type: String, pub instance: String, pub host: String, pub port: u16, pub addresses: Vec<IpAddr>, pub properties: BTreeMap<String, String> }`
- `pub fn browse(service_type: &str, duration: Duration) -> Result<Vec<Service>, String>`

## Using it

```toml
[dependencies]
floom-plugin-mdns = { path = "crates/ecosystem/plugins/floom-plugin-mdns" }
```

```rust,ignore
let found = floom_plugin_mdns::browse(
    "_http._tcp.local.",
    std::time::Duration::from_secs(2),
)?;
for service in found {
    println!("{}:{}", service.host, service.port);
}
```

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_mdns.h` — the C ABI contract a native
  half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-mdns
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-mdns/example/Cargo.toml
```
