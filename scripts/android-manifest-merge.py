#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
"""Merge plugin manifests into the app's, and set `android:hasCode` to match.

`aapt2 link` takes one manifest. AGP merges library manifests into the app's;
this script is the same step for the SDK-only pipeline, and it is deliberately
narrow: a plugin may contribute `<uses-permission>` and `<uses-feature>`, and
nothing else. A plugin that declares an `<activity>`, `<service>`,
`<receiver>`, `<provider>` or an `<application>` attribute is refused by name,
because those change what the host app *is* — merging them needs real manifest
semantics (priority, placeholders, authorities), and guessing at them is how a
plugin silently renames or re-exports the app.

`android:hasCode="false"` with a `classes.dex` in the APK is the other silent
failure: the package is installed, the dex is on disk, and none of its classes
load. So the flag is derived from whether this build produced a dex.
"""

import argparse
import re
import sys

COMMENT = re.compile(r"<!--.*?-->", re.S)
XML_DECLARATION = re.compile(r"<\?xml[^>]*\?>")
MANIFEST_OPEN = re.compile(r"<manifest\b[^>]*>")
USES = re.compile(r"<uses-(?:permission|feature)\b[^>]*?/>", re.S)
NAME = re.compile(r'android:name="([^"]+)"')
APPLICATION_TAG = re.compile(r"<application\b")


def fail(message):
    sys.exit(f"android-manifest-merge: {message}")


def declared_names(text):
    return {m.group(1) for m in NAME.finditer(text)}


def plugin_additions(label, text):
    body = COMMENT.sub("", text)
    body = XML_DECLARATION.sub("", body)
    body = MANIFEST_OPEN.sub("", body)
    body = body.replace("</manifest>", "")

    leftovers = USES.sub("", body).strip()
    if leftovers:
        fail(
            f"{label} declares something other than <uses-permission> / "
            f"<uses-feature>, which this pipeline cannot merge: {leftovers[:120]}"
        )
    return [m.group(0) for m in USES.finditer(body)]


def set_has_code(text, has_code):
    value = "true" if has_code else "false"
    if re.search(r'android:hasCode="[^"]*"', text):
        return re.sub(r'android:hasCode="[^"]*"', f'android:hasCode="{value}"', text)
    match = APPLICATION_TAG.search(text)
    if match is None:
        return text
    end = match.end()
    return f'{text[:end]}\n        android:hasCode="{value}"{text[end:]}'


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--app", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--has-code", choices=("true", "false"), required=True)
    parser.add_argument("--plugin", action="append", default=[])
    args = parser.parse_args()

    with open(args.app, encoding="utf-8") as handle:
        manifest = handle.read()

    seen = declared_names(manifest)
    additions = []
    for path in args.plugin:
        with open(path, encoding="utf-8") as handle:
            plugin_manifest = handle.read()
        for element in plugin_additions(path, plugin_manifest):
            name = NAME.search(element)
            if name is not None:
                if name.group(1) in seen:
                    continue
                seen.add(name.group(1))
            additions.append(element)

    if additions:
        manifest = manifest.replace(
            "</manifest>", "\n".join(["    " + a for a in additions]) + "\n</manifest>"
        )

    manifest = set_has_code(manifest, args.has_code == "true")

    with open(args.out, "w", encoding="utf-8") as handle:
        handle.write(manifest)


if __name__ == "__main__":
    main()
