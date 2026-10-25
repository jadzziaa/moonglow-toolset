#!/usr/bin/env python3
"""Make the custom palette probe for the Aurora oracle, and say how its
captures were made (crates/mg-corpus-tests/tests/aurora_palettes.rs).

    capture_palettes.py

It reads `captures/placement-probe.mod` (an Aurora-saved module with one
blueprint of each type) and writes into `captures/palettes/`:

- `before.mod` (also as `palette_probe.mod` in the oracle's `modules/`): its
  item palette's Longswords category (36) lists, out of order, `md_text`
  under a stale name ("Stale Sword Name"; the blueprint is "Module Sword"),
  `hk_strref` (a hak's, STRREF 91065), `gh_missing` (no blueprint
  anywhere), `md_strref` (STRREF 13411), `hk_text` (a hak's, with a field
  of its own, `XTRA`), `mgp_uti`, `md_strname` (NAME "Longsword" for a
  talk-table name) and `ov_item` (the override folder's); its Claw category
  (13) `md_moved` and `hk_moved` (both of category 36). `md_unlisted` and
  `hk_unlisted` are in no entry. The module lists the hak `mgpal`.
- `mgpal.hak` (also in the oracle's `hak/`): `hk_strref`, `hk_text`,
  `hk_unlisted`, `hk_moved`.
- `ov_item.uti` (also in the oracle's `override/`).
- `nopal-before.mod` (also as `palette_nopal.mod`): the same without its
  item palette.

Then, in Aurora (tools/aurora/headless.sh, run-aurora.sh; xdrive.py):

1. Open `palette_probe`, File > Save As `pal_saveas`: `saveas.mod`.
2. Open the area, the item palette's Custom tab, Weapons > Bladed >
   Longswords: right-click "Stale Sword Name" > Edit, OK, File > Save:
   `edit-ok.mod`. The same on "Potion of Cure Light Wounds"
   (`edit-strref.mod`), on the second "Longsword", `md_strname`
   (`edit-strname.mod`), and on Creature Items > Claw > "Moved Module
   Sword" (`edit-moved.mod`).
3. Right-click "Potion of Cure Light Wounds" > Edit Copy, OK, Save:
   `edit-copy.mod` (`md_strref001`).
4. Right-click > Refresh Palette, Save: `refresh.mod`.
5. Right-click "Unlisted Module Sword" > Delete, Yes, Yes, Save:
   `delete.mod`.
6. Open `palette_nopal`, Save As `pal_nopal_saved`: `nopal-saveas.mod`;
   Build > Build Module, Advanced Controls, only Compile > Palettes, Build,
   Done, Save: `nopal-build.mod`.

Each capture is the saved module copied from the oracle's `modules/`.
Another session may have the game client on the same off-screen display:
wait for its window to close before clicking where it lies.
"""

import copy
import json
import os
import shutil
import subprocess
import tempfile

BIN = os.environ.get("NWN_TOOLS_BIN", os.path.expanduser("~/.local/opt/neverwinter/bin"))
ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
CAPTURES = os.path.join(ORACLE, "captures")
OUT = os.path.join(CAPTURES, "palettes")
USER = os.path.join(ORACLE, "userdir")


def gff_to_json(path):
    out = subprocess.run([f"{BIN}/nwn_gff", "-i", path, "-k", "json"], capture_output=True, check=True)
    return json.loads(out.stdout)


def json_to_gff(data, path):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(data, f)
    subprocess.run([f"{BIN}/nwn_gff", "-i", f.name, "-l", "json", "-o", path, "-k", "gff"],
                   check=True, capture_output=True)
    os.unlink(f.name)


def pack(folder, path):
    files = sorted(os.path.join(folder, f) for f in os.listdir(folder))
    if os.path.exists(path):
        os.unlink(path)
    subprocess.run([f"{BIN}/nwn_erf", "-c", "-f", path] + files, check=True, capture_output=True)


def main():
    os.makedirs(OUT, exist_ok=True)
    base = tempfile.mkdtemp()
    subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", os.path.join(CAPTURES, "placement-probe.mod")],
                   cwd=base, check=True, capture_output=True)
    sword = gff_to_json(os.path.join(base, "mgp_uti.uti"))

    def item(resref, name, palette_id=36):
        g = copy.deepcopy(sword)
        value = {"id": name} if isinstance(name, int) else {"0": name}
        g["LocalizedName"] = {"type": "cexolocstring", "value": value}
        g["Tag"] = {"type": "cexostring", "value": resref.upper()}
        g["TemplateResRef"] = {"type": "resref", "value": resref}
        g["PaletteID"] = {"type": "byte", "value": palette_id}
        return g

    hak = tempfile.mkdtemp()
    for resref, name in [("md_text", "Module Sword"), ("md_strref", 13411), ("md_strname", 166),
                         ("md_unlisted", "Unlisted Module Sword"), ("md_moved", "Moved Module Sword")]:
        json_to_gff(item(resref, name), os.path.join(base, f"{resref}.uti"))
    for resref, name in [("hk_strref", 91065), ("hk_text", "Hak Sword"),
                         ("hk_unlisted", "Unlisted Hak Sword"), ("hk_moved", "Moved Hak Sword")]:
        json_to_gff(item(resref, name), os.path.join(hak, f"{resref}.uti"))
    json_to_gff(item("ov_item", "Override Sword"), os.path.join(OUT, "ov_item.uti"))

    def entry(resref, name=None, strref=None, extra=None):
        e = {"__struct_id": 0}
        if name is not None:
            e["NAME"] = {"type": "cexostring", "value": name}
        e["RESREF"] = {"type": "resref", "value": resref}
        if strref is not None:
            e["STRREF"] = {"type": "dword", "value": strref}
        e.update(extra or {})
        return e

    pal = gff_to_json(os.path.join(base, "itempalcus.itp"))

    def category(nodes, id_):
        for s in nodes:
            if s.get("ID", {}).get("value") == id_:
                return s
            found = category(s.get("LIST", {}).get("value", []), id_)
            if found:
                return found

    category(pal["MAIN"]["value"], 36)["LIST"]["value"] = [
        entry("md_text", name="Stale Sword Name"),
        entry("hk_strref", strref=91065),
        entry("gh_missing", name="Ghost Sword"),
        entry("md_strref", strref=13411),
        entry("hk_text", name="Hak Sword", extra={"XTRA": {"type": "dword", "value": 7}}),
        entry("mgp_uti", name="Longsword"),
        entry("md_strname", name="Longsword"),
        entry("ov_item", name="Override Sword"),
    ]
    category(pal["MAIN"]["value"], 13)["LIST"] = {"type": "list", "value": [
        entry("md_moved", name="Moved Module Sword"),
        entry("hk_moved", name="Moved Hak Sword"),
    ]}
    json_to_gff(pal, os.path.join(base, "itempalcus.itp"))
    ifo = gff_to_json(os.path.join(base, "module.ifo"))
    ifo["Mod_HakList"] = {"type": "list", "value": [
        {"__struct_id": 8, "Mod_Hak": {"type": "cexostring", "value": "mgpal"}}]}
    ifo["Mod_Name"] = {"type": "cexolocstring", "value": {"0": "palette_probe"}}
    json_to_gff(ifo, os.path.join(base, "module.ifo"))

    pack(base, os.path.join(OUT, "before.mod"))
    pack(hak, os.path.join(OUT, "mgpal.hak"))
    os.unlink(os.path.join(base, "itempalcus.itp"))
    pack(base, os.path.join(OUT, "nopal-before.mod"))
    shutil.rmtree(base)
    shutil.rmtree(hak)

    for folder in ("modules", "hak", "override"):
        os.makedirs(os.path.join(USER, folder), exist_ok=True)
    shutil.copy(os.path.join(OUT, "before.mod"), os.path.join(USER, "modules", "palette_probe.mod"))
    shutil.copy(os.path.join(OUT, "nopal-before.mod"), os.path.join(USER, "modules", "palette_nopal.mod"))
    shutil.copy(os.path.join(OUT, "mgpal.hak"), os.path.join(USER, "hak", "mgpal.hak"))
    shutil.copy(os.path.join(OUT, "ov_item.uti"), os.path.join(USER, "override", "ov_item.uti"))
    print(f"made {OUT}; now capture with Aurora as this script's doc says")


if __name__ == "__main__":
    main()
