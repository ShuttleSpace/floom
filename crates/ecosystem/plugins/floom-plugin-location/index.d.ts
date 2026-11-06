// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-location`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string; `Option<T>` becomes `T |
// null`. This is a declaration only: the capability is implemented by the Rust
// crate, not by JavaScript.

declare module "floom-plugin-location" {
  export type LocationAuthorization =
    | "NotDetermined"
    | "Restricted"
    | "Denied"
    | "AuthorizedWhenInUse"
    | "AuthorizedAlways";

  export interface Location {
    latitude: number;
    longitude: number;
    altitude: number;
    horizontal_accuracy: number;
    vertical_accuracy: number;
    /** Unix milliseconds. */
    timestamp: number;
  }

  export function authorization_status(): LocationAuthorization;
  export function is_location_enabled(): boolean;
  /** Throws when the platform cannot ask or the user declines. */
  export function request_authorization(): void;
  /** Throws when there is no fix or no permission. */
  export function current_location(): Location;
}
