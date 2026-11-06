// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-local-auth`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-local-auth" {
  export type BiometricKind = "None" | "Fingerprint" | "Face" | "Iris";

  export function available_biometric(): BiometricKind;
  /** Throws when the user cancels or the host cannot authenticate. */
  export function authenticate(reason: string): void;
}
