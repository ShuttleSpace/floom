// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-notifications`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-notifications" {
  export interface Notification {
    title: string;
    body: string;
    badge: number | null;
    sound: boolean;
    /** Seconds to wait before delivering, or `null` for immediate. */
    delay_seconds: number | null;
  }

  export function are_notifications_enabled(): boolean;
  export function request_authorization(): void;
  /** Throws when the host cannot deliver notifications. */
  export function schedule(notification: Notification): void;
}
