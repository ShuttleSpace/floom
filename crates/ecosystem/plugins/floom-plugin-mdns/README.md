<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-mdns

Discover services on the local network — the mDNS / DNS-SD question.

This is `multicast_dns`'s question, and the catalog's C6
(`docs/plugin-catalog.md`): "who on this LAN is serving `_http._tcp`?" —
find the printer, the cast target, the other player, without knowing an
address beforehand.

A **capability plugin**: one capability, one crate, opt-in by dependency.

# Why there are no platform arms

mDNS is a *protocol* (RFC 6762/6763), not a platform API: the wire format
is the same everywhere, so one implementation serves every target this
crate compiles for — the way `std`'s sockets serve
`floom-plugin-network-info`'s `local_ip`. There is no `android.rs` to rot
and no host-inert arm to keep honest; the platform-specific socket code
lives inside `mdns-sd`, and the cross-target checks compile it for the
phone and Windows targets where a host build never would.

# What `browse` promises

A bounded window, not a stream: send the questions, collect what
resolves before the deadline, return one snapshot. Within the window a
re-resolved instance replaces its earlier answer and a removed instance
leaves the result — the snapshot is *as of the deadline*, which is the
only honest thing a local-network census can say.

An empty result is "nobody answered in the window": a real answer, not a
failure. On a phone it can additionally mean the Android caveat below.

# The Android caveat, named

Android's Wi-Fi stack may filter multicast packets outright; the
platform's remedy is `WifiManager.MulticastLock` — a Java API that needs
the `CHANGE_WIFI_MULTICAST_STATE` permission in the app's manifest, which
the `hasCode="false"` example apps do not declare. This v1 browses
without the lock: on a phone, an empty result may mean filtered multicast
rather than an empty network. The follow-up arm is the platform JNI
bridge acquiring the lock for an app that declares the permission — a
door named here so the next person finds it.

# What this crate deliberately does not do

No *advertising* ("tell the network I am here"): Flutter's package does
not either, and answering is a server posture with its own lifecycle — a
different crate if it is ever wanted. No continuous monitoring: an
unbounded stream of events is a UI concern (a `floom-signal` fed by a
poll of this function), not a query API's job. No unicast DNS-SD, no
sub-type browsing, no hostname resolution — the catalog's question is
the plain one, and v1 answers it.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | pure Rust (no native half) |
| iOS | ✅ Real | pure Rust (no native half) |
| macOS | ✅ Real | pure Rust (no native half) |
| Windows | ✅ Real | pure Rust (no native half) |
| Linux | ✅ Real | pure Rust (no native half) |
| OpenHarmony | ✅ Real | pure Rust (no native half) |
| Web | ✅ Real | pure Rust (no native half) |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `browse` — Browse the local network for instances of `service_type`, for `duration`.


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-mdns = { path = "crates/ecosystem/plugins/floom-plugin-mdns" }
```

A minimal use:

```rust,ignore
let found = floom_plugin_mdns::browse(
    "_http._tcp.local.",
    std::time::Duration::from_secs(2),
)?;
for service in found {
    println!("{}:{}", service.host, service.port);
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct Service {`
- `pub fn browse(service_type: &str, duration: Duration) -> Result<Vec<Service>, String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_mdns.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-mdns
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-mdns/example/Cargo.toml
```
