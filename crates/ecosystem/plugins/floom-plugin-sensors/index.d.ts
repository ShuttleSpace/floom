// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-sensors`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-sensors" {
  export type SensorKind =
    | "Accelerometer"
    | "Gyroscope"
    | "Magnetometer"
    | "Barometer"
    | "Proximity"
    | "AmbientLight";

  export const all: SensorKind[];
  export function available_sensors(): SensorKind[];
  export function is_sensor_available(kind: SensorKind): boolean;
}
