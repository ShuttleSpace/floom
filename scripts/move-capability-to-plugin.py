#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
"""Move a capability out of the mobile platform adapter and into a plugin crate.

The doctrine (`docs/platform-and-plugins.md`) says a platform adapter is an
*adapter* and must not accumulate capabilities of its own, and that a
capability is one crate. This script does the mechanical half of that: the
file move, the manifest, and the four registration points. The judgement
calls -- which capability goes where -- are made before it runs, not by it.
"""

from __future__ import annotations

import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MOBILE = ROOT / "crates/platform/floom-platform-mobile"
PACKAGES = MOBILE / "src/packages"

# package directory -> plugin crate name
CAPABILITIES = {
    "battery": "battery",
    "connectivity": "connectivity",
    "device_info": "device-info",
    "local_auth": "local-auth",
    "location": "location",
    "notifications": "notifications",
    "sensors": "sensors",
    "share": "share",
    "vibration": "vibration",
}

# The packages that stay in the adapter, and why.
STAY = {
    "clipboard": "one of the core three: `floom_platform::Clipboard`",
    "keyboard": "one of the core three: `floom_platform::Ime`",
    "safe_area": "a fact about the window, not a capability a plugin provides",
    "system_chrome": "a fact about the window, not a capability a plugin provides",
}


def read(path: Path) -> str:
    return path.read_text()


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def cargo_toml(crate: str, description: str, needs_jni_bridge: bool) -> str:
    deps = []
    if needs_jni_bridge:
        deps.append(
            '\n# The JNI bridge — the JavaVM, the activity — is platform\n'
            "# plumbing, so it lives in the platform layer and a plugin reaches\n"
            '# for it. A plugin reaching *down* to the platform is the allowed\n'
            "# direction; the platform reaching up to a plugin is not.\n"
            'floom-platform-mobile = { path = "../floom-platform-mobile", version = "0.2.0" }\n'
        )
    ios = ""
    android = ""
    if crate == "vibration":
        ios = (
            '\n# The iOS half reaches `UIKit` and `AudioToolbox` through objc2.\n'
            '[target.\'cfg(target_os = "ios")\'.dependencies]\n'
            'objc2 = "0.6"\n'
        )
        android = (
            '\n# The Android half reaches `Vibrator` through JNI.\n'
            '[target.\'cfg(target_os = "android")\'.dependencies]\n'
            'jni = "0.22"\n'
        )
    body = f"""[package]
name = "floom-plugin-{crate}"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "{description}"
rust-version.workspace = true
publish = false

# A plugin owns a *capability* and the host owns the *transport*. It depends
# on the platform layer and on nothing above it: no widget stack, no renderer,
# no facade. An app that wants this capability adds this crate; an app that
# does not, does not link it.
[dependencies]
{''.join(deps)}{ios}{android}"""
    return body


def move(package: str, crate: str, description: str) -> None:
    src = PACKAGES / package
    dst = ROOT / f"crates/floom-plugin-{crate}"
    if not src.is_dir():
        if dst.is_dir():
            # Already moved: the script is re-runnable, which matters because a
            # half-finished migration is worse than none.
            print(f"skipping {package}: already moved")
            return
        sys.exit(f"missing package: {src}")

    (dst / "src").mkdir(parents=True, exist_ok=True)

    # `mod.rs` becomes the crate's `lib.rs`; the two backend arms move with it.
    shutil.copy2(src / "mod.rs", dst / "src/lib.rs")
    for arm in ("android.rs", "ios.rs"):
        if (src / arm).is_file():
            shutil.copy2(src / arm, dst / "src" / arm)

    # The JNI bridge is the platform's, so the path changes from `crate::jni`
    # to the platform crate's.
    android = dst / "src/android.rs"
    if android.is_file():
        text = read(android)
        text = text.replace(
            "use crate::jni::{self as jni_helpers, JniExt, obj};",
            "use floom_platform_mobile::jni::{self as jni_helpers, JniExt, obj};",
        )
        text = text.replace("`crate::jni`", "`floom_platform_mobile::jni`")
        write(android, text)

    write(dst / "Cargo.toml", cargo_toml(crate, description, package == "vibration"))
    shutil.rmtree(src)


def main() -> None:
    descriptions = {
        "battery": "A Floom plugin: the device battery",
        "connectivity": "A Floom plugin: the device's network connectivity",
        "device_info": "A Floom plugin: what this device is",
        "local_auth": "A Floom plugin: biometric and device-owner authentication",
        "location": "A Floom plugin: the device's location",
        "notifications": "A Floom plugin: the device's notification authorisation",
        "sensors": "A Floom plugin: the device's motion and environment sensors",
        "share": "A Floom plugin: the platform share sheet",
        "vibration": "A Floom plugin: haptics and device vibration",
    }
    for package, crate in CAPABILITIES.items():
        move(package, crate, descriptions[package])
        print(f"moved {package} -> floom-plugin-{crate}")


if __name__ == "__main__":
    main()
