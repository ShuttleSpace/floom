// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-network-info`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-network-info" {
  export interface WifiInfo {
    ssid: string | null;
    bssid: string | null;
  }

  /** The device's address on the default route, or `null` when there is none. */
  export function local_ip(): string | null;
  /** Throws when this host cannot name the Wi-Fi network, naming the reason. */
  export function wifi_info(): WifiInfo;
}
