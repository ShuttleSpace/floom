<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->

# floom-plugin-background-work

Ask the OS to bring this app back at a time.

This is the catalog's A8 (`docs/plugin-catalog.md`), which merged
`android_alarm_manager_plus` and `workmanager` into one crate on the
theory that both answer "make some code run later". Writing the arm
found a harder truth, and the crate is the honest narrowing of it.

# What an app in this shape can actually be promised

The example apps here are `android:hasCode="false"` `NativeActivity`
builds: no Java classes of their own. That closes every Flutter-shaped
door at once —

* an **alarm callback** needs a `<receiver>` in the manifest and Java
code for the system to call;
* **WorkManager** needs a `Worker` class, which is Java code;
* **iOS `BGTaskScheduler`** needs a task identifier in the `Info.plist`
and an Objective-C handler registered at startup — the Shape-B native
half this plugin family deliberately does not carry
(`docs/plugin-catalog.md` §8.1).

The one primitive left standing needs **no Java code at all**: an
`AlarmManager` alarm whose payload is a `PendingIntent` that starts the
activity itself. The system holds the intent, survives the process, and
at the appointed time brings the app up. That is
[`schedule_app_launch`], and its semantics are exactly what the name
says — the *app's own lifecycle is the callback*: the activity comes to
the front (or is recreated), and the app's startup path runs.

# What `Ok` means, and what the alarm is worth

`Ok(())` claims the **system accepted the ask** — the "true launch"
category from the catalog's verification matrix. The alarm uses
`setAndAllowWhileIdle`, which is *inexact*: the OS batches alarms to
save power, so "at 9:00" is really "around 9:00, in Doze possibly
noticeably later". Exact alarms (`setExactAndAllowWhileIdle`) need the
`SCHEDULE_EXACT_ALARM` permission on API 31+, which is the app's
manifest to write — the crate does not quietly require it. A time
already in the past fires immediately; that is the platform's behaviour
and this crate reports it rather than second-guessing it.

# The notifications recipe

This is what un-blocks `floom-plugin-notifications`' Android delayed
arm, which refuses because posting *now* at the wrong time is worse
than not posting. The recipe, in the app's own code:

1. `schedule_app_launch(at)` — the system brings the app back at `at`;
2. before scheduling, persist "a notification is owed at `at`" (a
`floom-plugin-prefs` row, or the app's own store);
3. in startup, check the row and post the notification *immediately*
(delay `None`), then clear it.

The user sees the app come up and the notification with it — heavier
than a silent post, and that is the honest cost of a no-Java build.

# What this crate deliberately does not do

No opportunistic background execution (the workmanager question): the
Android and iOS halves are the native classes above, and pretending to
a `submit()` that cannot run work would be a stub with a launch button.
No in-process timers: a closure that runs later while the process
lives is `floom-scheduler`'s job, and calling it "background work"
would be the dishonest version of this crate.

## Supported platforms

| Platform | Support | Backend |
|----------|---------|---------|
| Android | ✅ Real | `src/android.rs` |
| iOS | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| macOS | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Windows | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Linux | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| OpenHarmony | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |
| Web | ⚪ Inert fallback | `src/lib.rs` returns an honest default / `Err` |

Every target **compiles and runs** this plugin. Where a platform has no native
arm, the call returns an honest inert value or a named `Err` — never a value it
did not measure. The matrix above shows which platforms ship a real
implementation versus the inert fallback.

## Capabilities

This plugin provides:

- `schedule_app_launch` — Ask the OS to start this app at `at`.
- `cancel_scheduled_app_launch` — Withdraw the ask made by [`schedule_app_launch`].


## Integration

Add the crate as a dependency of your app:

```toml
[dependencies]
floom-plugin-background-work = { path = "crates/ecosystem/plugins/floom-plugin-background-work" }
```

A minimal use:

```rust,ignore
floom_plugin_background_work::schedule_app_launch(
    std::time::SystemTime::now() + std::time::Duration::from_secs(300),
)?;
```

## API

The public Rust surface (mirrored by `index.d.ts`):

- `pub fn schedule_app_launch(at: SystemTime) -> Result<(), String> {`
- `pub fn cancel_scheduled_app_launch() -> Result<(), String> {`

## Layout

This plugin follows the capability-plugin template in
[`docs/platform-and-plugins.md`](../../../../docs/platform-and-plugins.md#the-layout):

- `src/lib.rs` — the unified Rust API and the per-target dispatch.
- `src/<target>.rs` — each target's arm; see `native/README.md` for the mapping.
- `native/common/floom_background-work.h` — the C ABI contract a native half implements.
- `index.d.ts` — the TypeScript facade, mirroring the Rust API name for name.
- `example/` — a runnable host that prints what the plugin answers here.

## Verification

```bash
cargo test --manifest-path crates/ecosystem/plugins/Cargo.toml -p floom-plugin-background-work
cargo run  --manifest-path crates/ecosystem/plugins/floom-plugin-background-work/example/Cargo.toml
```
