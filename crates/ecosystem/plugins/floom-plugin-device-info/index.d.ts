// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-device-info`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-device-info" {
  export interface DeviceInfo {
    model: string;
    manufacturer: string;
    os_name: string;
    os_version: string;
    /** Android API level, or `null` where the platform has no such number. */
    sdk_version: number | null;
    is_physical_device: boolean;
    locale: string;
  }

  export function device_info(): DeviceInfo;
  export function device_name(): string;
}
