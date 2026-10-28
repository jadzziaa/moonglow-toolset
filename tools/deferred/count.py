#!/usr/bin/env python3
"""Counts the open entries of docs/deferred.md, and lists them.

    tools/deferred/count.py                 # the counts, by section
    tools/deferred/count.py --json OUT      # every entry, with an id, as JSON

An entry is a top-level `- ` item of an open section (every section but
the last, "Settled"); its items are the `- ` lines indented under it. An
id is the entry's section and its bold title (or first words) as a slug,
so it stays the same while the entry is edited elsewhere.
"""

import json
import re
import sys
from pathlib import Path

DEFERRED = Path(__file__).resolve().parents[2] / "docs" / "deferred.md"


def slug(text, words=8):
    text = re.sub(r"[`*_\[\]()]", "", text)
    return "-".join(re.findall(r"[a-z0-9]+", text.lower())[:words])


def entries(path=DEFERRED):
    section, current, found = None, None, []
    for number, line in enumerate(path.read_text().split("\n"), 1):
        if line.startswith("## "):
            section, current = line[3:].strip(), None
            continue
        if section is None or section.startswith("Settled"):
            continue
        if line.startswith("- "):
            text = line[2:]
            bold = re.match(r"\*\*(.+?)\*\*", text)
            title = (bold.group(1) if bold else " ".join(text.split()[:10])).rstrip(":")
            current = {"section": section, "title": title, "line": number, "items": 0, "text": text}
            found.append(current)
        elif current and re.match(r" {2,}- ", line):
            current["items"] += 1
            current["text"] += "\n" + line.strip()
        elif current and line.startswith("  "):
            current["text"] += " " + line.strip()
    seen = {}
    for e in found:
        base = slug(e["section"].split(" (")[0], 4) + "/" + slug(e["title"])
        seen[base] = seen.get(base, 0) + 1
        e["id"] = base if seen[base] == 1 else f"{base}-{seen[base]}"
        sizes = re.findall(r"\((S|M|L|S–M|M–L|\?)[;,)\s]", e["text"])
        e["sizes"] = sizes
    return found


def main():
    found = entries()
    if len(sys.argv) == 3 and sys.argv[1] == "--json":
        Path(sys.argv[2]).write_text(json.dumps(found, indent=1, ensure_ascii=False) + "\n")
        print(f"{len(found)} entries written to {sys.argv[2]}")
        return
    sections = {}
    for e in found:
        n, items = sections.get(e["section"], (0, 0))
        sections[e["section"]] = (n + 1, items + max(1, e["items"]))
    for name, (n, items) in sections.items():
        print(f"{n:4} {items:4}  {name.split(' (')[0]}")
    items = sum(max(1, e["items"]) for e in found)
    print(f"{len(found):4} {items:4}  open entries, items (an entry's items counted one by one)")


if __name__ == "__main__":
    main()
