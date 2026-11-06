// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-battery`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-battery" {
  export interface BatteryState {
    /** Fraction in `0.0..=1.0`, or `null` when the host does not report one. */
    level: number | null;
    is_charging: boolean;
    is_full: boolean;
  }

  export function battery_state(): BatteryState;
  export function is_low_power_mode(): boolean;
}
