// SPDX-License-Identifier: MIT
// TypeScript facade for `floom-plugin-camera`.
//
// Mirrors the crate's public Rust API name for name. `Result<T, E>` becomes a
// return value that throws the Rust error string. This is a declaration only:
// the capability is implemented by the Rust crate's native half, not by
// JavaScript, and `provider` is a process-wide native-view handle rather than
// something a browser can construct.

declare module "floom-plugin-camera" {
  export function version(): string;
  /** The capability id, stable across releases. */
  export function capability(): number;
  /** Throws when the native half refuses to enable. */
  export function enable(): void;
  export function disable(): void;
  /** Registers the plugin in the process-wide table. */
  export function register(): void;

  /**
   * The provider for this plugin, if the native half has handed one over.
   * `hostWindow` is the platform's own window handle (an `NSView *` on macOS,
   * an `ANativeWindow *` on Android); the caller keeps it alive.
   */
  export function provider(hostWindow: unknown): unknown;
}
