#!/usr/bin/env python3
"""Writes the licences of the crates built into Moonglow's programs.

Moonglow is GPL-3.0-only; the crates it is built from carry their own
licences (MIT, Apache-2.0, MPL-2.0, ...), whose notices travel with the
binaries. This lists every crate the programs link (normal dependencies
for one target, not build or dev ones) with its licence, followed by the
licence files each ships, the same text printed once for all the crates
that share it.

    packaging/third_party_licenses.py [--target TRIPLE] > THIRD-PARTY-LICENSES.txt
"""

import argparse
import json
import pathlib
import subprocess
import sys

PROGRAMS = ["moonglow", "mg"]
LICENSE_FILES = ("license", "licence", "copying", "notice", "unlicense", "copyright")
# Folders with the licences of what a crate bundles (fonts, ...).
BUNDLED = ("fonts", "licenses", "licences")


def licence_files(root):
    """A crate's licence files, and those of what it bundles."""
    found = []
    for d in [root] + [root / b for b in BUNDLED if (root / b).is_dir()]:
        for f in sorted(d.iterdir()):
            name = f.name.lower()
            bundled = d != root and name.endswith(".txt")
            if f.is_file() and (name.startswith(LICENSE_FILES) or bundled):
                found.append(f)
    return found


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--target", help="target triple (default: the host's)")
    args = ap.parse_args()
    target = args.target or host()
    meta = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--format-version", "1", "--locked",
         "--filter-platform", target]))
    packages = {p["id"]: p for p in meta["packages"]}
    nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
    workspace = set(meta["workspace_members"])
    roots = [i for i in workspace if packages[i]["name"] in PROGRAMS]
    seen, todo = set(), list(roots)
    while todo:
        i = todo.pop()
        if i in seen:
            continue
        seen.add(i)
        for d in nodes[i]["deps"]:
            if any(k["kind"] is None for k in d["dep_kinds"]):
                todo.append(d["pkg"])
    crates = sorted((packages[i] for i in seen - workspace),
                    key=lambda p: (p["name"], p["version"]))

    out = sys.stdout
    out.write("Moonglow Toolset is licensed under the GNU General Public License,\n"
              "version 3 (LICENSE). It includes Beamdog's NWScript compiler\n"
              "(GPL-3.0) with neverwinter.nim's C API over it (MIT, copyright\n"
              "the neverwinter.nim authors, https://github.com/niv/neverwinter.nim).\n"
              "It is built from these\n"
              f"crates, under their own licences (target {target}):\n\n")
    for p in crates:
        out.write(f"  {p['name']} {p['version']}: {p['license'] or p.get('license_file') or '?'}\n")
    texts = {}
    for p in crates:
        root = pathlib.Path(p["manifest_path"]).parent
        files = licence_files(root)
        if p.get("license_file"):
            files.append(root / p["license_file"])
        if not files:
            # Some crates are published without their licence file: name
            # the licence and its holders; the licence's text is among the
            # others.
            authors = ", ".join(p.get("authors") or []) or "its authors"
            text = (f"{p['name']} {p['version']} is published without its licence "
                    f"file. It is licensed under {p['license']}, copyright {authors}"
                    f" ({p.get('repository') or 'crates.io'}); the licence texts are "
                    "given in this file for other crates.")
            texts.setdefault(text, []).append(f"{p['name']} {p['version']}")
        for f in dict.fromkeys(files):
            try:
                text = f.read_text(encoding="utf-8", errors="replace").strip()
            except OSError:
                continue
            texts.setdefault(text, []).append(f"{p['name']} {p['version']} ({f.name})")
    for text, users in texts.items():
        out.write("\n" + "=" * 78 + "\n")
        out.write("".join(f"{u}\n" for u in users))
        out.write("-" * 78 + "\n\n" + text + "\n")


def host():
    for line in subprocess.check_output(["rustc", "-vV"], text=True).splitlines():
        if line.startswith("host: "):
            return line[6:]
    sys.exit("no host triple from rustc -vV")


if __name__ == "__main__":
    main()
