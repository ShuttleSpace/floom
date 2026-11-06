<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-network-info

The current Wi-Fi network's name, and this device's address on it.

This is `network_info_plus`'s territory, and the catalog's A7
(`docs/plugin-catalog.md`) — with one design correction the Flutter
package invites: it conflates **two questions with different privacy
profiles** under one "network info" roof.

* [`local_ip`] — "what is *my* address". Not private in any meaningful
sense (every peer you talk to learns it), answerable everywhere, and
the same question on a phone and on a desktop.
* [`wifi_info`] — "what is the *network's* name". Genuinely private —
SSIDs are location-identifying, and both phone platforms gate them
behind permissions or entitlements for exactly that reason.

Splitting them is why this crate has two functions instead of one
struct: a caller who only wants to print "connected from 192.168.1.4"
should not have to route through the capability that drags in the
location-permission story.

A **capability plugin**: one capability, one crate, opt-in by dependency.

# What `wifi_info` answers, per platform

| Platform | `Ok(..)` with `None` fields | `Err` |
|---|---|---|
| Android | the SSID came back `<unknown ssid>` — location permission not granted (or location, or Wi-Fi, is off); the platform is *able* to talk Wi-Fi but not willing to name it | no `WifiManager` service, or the bridge is down |
| iOS / macOS | no Wi-Fi interface to ask about | `CNCopyCurrentNetworkInfo` returned nothing — on iOS 13+ that is the missing `com.apple.developer.network-information` entitlement as often as it is "not associated" (see the arm's docs: the API conflates the two) |
| Linux | — | NetworkManager over D-Bus is the only door, and this arm does not start a system-service conversation |
| Windows | — | `WlanQueryInterface` (wlanapi) is the door; deliberately not in v1 — the crate doc records it as a named follow-up rather than shipping an arm this build cannot run |

# What this crate deliberately does not do

No subnet mask, gateway, or broadcast address: `network_info_plus` has
them, but every platform answers them through a different mechanism
(ioctl in Java, routing sockets on Apple, MIB queries on Windows), and
v1 answers the two questions callers actually ask. No DNS, no
reachability — that is `floom-plugin-connectivity`'s question, and it is
already answered there.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ✅ Real | `src/apple.rs` |
| macOS | ✅ Real | `src/apple.rs` |
| Windows | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Linux | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `local_ip` — This device's address on the network its default route uses.
- `wifi_info` — The current Wi-Fi network's SSID and BSSID, where the platform is


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-network-info = { path = "crates/ecosystem/plugins/floom-plugin-network-info" }
```

A minimal use:

```rust,ignore
println!("local address: {:?}", floom_plugin_network_info::local_ip());
match floom_plugin_network_info::wifi_info() {
    Ok(info) => println!("ssid: {:?}", info.ssid),
    Err(reason) => println!("wifi_info: {reason}"),
}
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub struct WifiInfo {`
- `pub fn local_ip() -> Option<IpAddr> {`
- `pub fn wifi_info() -> Result<WifiInfo, String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_network-info.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-network-info
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-network-info/example/Cargo.toml
```
