# macOS half of this plugin

Open this directory with Xcode (AppKit). It is a real project, not a stub: the
files needed to build it are here.

## What belongs here

The platform implementation of the C ABI declared in `../floom.h`. Kotlin,
Swift, or C — whichever the platform expects.

## Where dependencies go

In this project's build file, not in the Rust `Cargo.toml`:

* Android — `build.gradle.kts`. `androidx.camera:camera-*` and every other
AAR are Maven artifacts, and Gradle is the only part of this toolchain
that can resolve one. A `build.rs` cannot.
* iOS — the `.podspec`. Same reason: CocoaPods resolves, Rust does not.
* Desktop — `CMakeLists.txt`, for anything `find_package` can express.

## What does not belong here

Application logic. A plugin does not own an activity, a bundle id or a
window: the host owns those. What the plugin does is claim a capability,
and Rust calls into it directly rather than across a message channel.
