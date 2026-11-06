// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-mdns`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-mdns" {
  export interface Service {
    fullname: string;
    service_type: string;
    instance: string;
    host: string;
    port: number;
    /** IP addresses as text, in sorted order. */
    addresses: string[];
    properties: Record<string, string>;
  }

  /** `duration_ms` is the browse window; throws on a malformed service type. */
  export function browse(service_type: string, duration_ms: number): Service[];
}
