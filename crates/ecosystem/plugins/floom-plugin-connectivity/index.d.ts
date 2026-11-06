// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-connectivity`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-connectivity" {
  export type ConnectionType =
    | "None"
    | "Wifi"
    | "Mobile"
    | "Ethernet"
    | "Bluetooth"
    | "Other";

  export interface ConnectivityStatus {
    connection_type: ConnectionType;
    is_connected: boolean;
    is_metered: boolean;
  }

  export function connection_type(): ConnectionType;
  export function connectivity_status(): ConnectivityStatus;
  export function is_connected(): boolean;
  export function is_metered(): boolean;
}
