#!/usr/bin/env python3
"""Writes cargo-sources.json: the crates in Cargo.lock as flatpak-builder
sources, so the Flatpak builds offline.

Each crate is an archive from crates.io (checked against Cargo.lock's
checksum), unpacked into cargo/vendor/<name>-<version> with the
.cargo-checksum.json that Cargo's vendored sources need, and cargo/config
points Cargo at them (cargo/config.toml). The same thing flatpak-builder-tools' cargo generator
writes for crates.io crates, without its Python dependencies; Moonglow has
no git dependencies (the script refuses them).

    packaging/flatpak/cargo_sources.py [Cargo.lock] > packaging/flatpak/cargo-sources.json

With --extra-sources DIR, it also links the crates Cargo has already
downloaded into DIR as flatpak-builder's --extra-sources wants them
(downloads/<sha256>/<file>), so the build need not download them again.
"""

import argparse
import json
import os
import pathlib
import sys
import tomllib

CRATES_IO = "registry+https://github.com/rust-lang/crates.io-index"
VENDOR = "cargo/vendor"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("lock", nargs="?", default="Cargo.lock")
    ap.add_argument("--extra-sources", metavar="DIR",
                    help="link Cargo's downloaded crates here for flatpak-builder")
    args = ap.parse_args()
    with open(args.lock, "rb") as f:
        lock = tomllib.load(f)
    sources = []
    for p in lock["package"]:
        source = p.get("source")
        if source is None:
            continue  # a workspace crate
        if source != CRATES_IO:
            sys.exit(f"{p['name']} {p['version']} comes from {source}: not supported")
        name, version, checksum = p["name"], p["version"], p["checksum"]
        dest = f"{VENDOR}/{name}-{version}"
        sources.append({
            "type": "archive",
            "archive-type": "tar-gzip",
            "url": f"https://static.crates.io/crates/{name}/{name}-{version}.crate",
            "sha256": checksum,
            "dest": dest,
        })
        sources.append({
            "type": "inline",
            "contents": json.dumps({"package": checksum, "files": {}}),
            "dest": dest,
            "dest-filename": ".cargo-checksum.json",
        })
    sources.append({
        "type": "inline",
        "contents": (
            "[source.vendored-sources]\n"
            f'directory = "{VENDOR}"\n\n'
            "[source.crates-io]\n"
            'replace-with = "vendored-sources"\n'
        ),
        "dest": "cargo",
        "dest-filename": "config.toml",
    })
    json.dump(sources, sys.stdout, indent=2)
    sys.stdout.write("\n")
    if args.extra_sources:
        link_cached(sources, pathlib.Path(args.extra_sources))


def link_cached(sources, out):
    """Links each archive Cargo has in its download cache into
    out/downloads/<sha256>/<file name>."""
    home = pathlib.Path(os.environ.get("CARGO_HOME") or pathlib.Path.home() / ".cargo")
    caches = [d for d in (home / "registry" / "cache").glob("*") if d.is_dir()]
    linked = 0
    for s in sources:
        if s["type"] != "archive":
            continue
        name = s["url"].rsplit("/", 1)[1]
        cached = next((c / name for c in caches if (c / name).is_file()), None)
        if cached is None:
            continue
        link = out / "downloads" / s["sha256"] / name
        link.parent.mkdir(parents=True, exist_ok=True)
        if not link.exists():
            link.symlink_to(cached)
        linked += 1
    print(f"{linked} crates from Cargo's cache", file=sys.stderr)


if __name__ == "__main__":
    main()
