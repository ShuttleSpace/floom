#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
"""List the Android library modules of every plugin an app depends on.

The app's APK needs one thing from a plugin that Cargo cannot give it: the
Java/Kotlin classes. They are not compiled by Cargo and not linked into the
`.so`, so the packaging step has to find them. Doing that by hand is how an app
ships a `.so` that calls `dev/floom/plugin/battery/BatteryPlugin` while the APK
has no such class, and the failure appears on the device as a
`ClassNotFoundException` from inside a Rust call.

So the list comes from the dependency graph, not from a written-down list: a
plugin is packaged exactly when the app links it. A plugin owns its Java half at
`<crate>/android/src/main/java`.

Output: one line per module, `<crate>\t<crate dir>`, sorted by crate name.
"""

import argparse
import json
import os
import subprocess
import sys


def dependency_closure(metadata, package_name):
    root = next((p for p in metadata["packages"] if p["name"] == package_name), None)
    if root is None:
        sys.exit(f"android-plugin-modules: no package named `{package_name}` in the graph")

    nodes = {n["id"]: n for n in metadata["resolve"]["nodes"]}
    seen = set()
    queue = [root["id"]]
    while queue:
        package_id = queue.pop()
        if package_id in seen:
            continue
        seen.add(package_id)
        node = nodes.get(package_id)
        if node is None:
            continue
        for dependency in node["deps"]:
            if dependency["pkg"] not in seen:
                queue.append(dependency["pkg"])
    return seen


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest-path", required=True)
    parser.add_argument("--package", required=True)
    args = parser.parse_args()

    metadata = json.loads(
        subprocess.check_output(
            [
                "cargo",
                "metadata",
                "--format-version",
                "1",
                "--manifest-path",
                args.manifest_path,
            ],
            text=True,
        )
    )

    packages = {p["id"]: p for p in metadata["packages"]}
    for package_id in sorted(dependency_closure(metadata, args.package)):
        package = packages.get(package_id)
        if package is None:
            continue
        crate_dir = os.path.dirname(package["manifest_path"])
        if os.path.isdir(os.path.join(crate_dir, "android", "src", "main", "java")):
            print(f'{package["name"]}\t{crate_dir}')


if __name__ == "__main__":
    main()
