// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-vibration`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-vibration" {
  export type HapticFeedback =
    | "Light"
    | "Medium"
    | "Heavy"
    | "Selection"
    | "Success"
    | "Warning"
    | "Error";

  export function can_vibrate(): boolean;
  /** Throws when the host has no haptic motor. */
  export function vibrate(duration_ms: number): void;
  export function haptic_feedback(feedback: HapticFeedback): void;
}
