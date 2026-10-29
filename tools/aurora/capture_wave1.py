#!/usr/bin/env python3
"""Make the probes of the deferred sprint's Aurora checks (wave 1,
2026-10-29), and say how their captures were made.

    capture_wave1.py

It reads `captures/placement-probe.mod` and the game (through `mg cat`)
and writes into `captures/wave1/` (and the oracle's `modules/`, `hak/`,
`erf/`):

- `before.mod` (`w1probe.mod`): a script; the area's GIT with grass of its
  own (`GrassDefDisabled`, `GrassOverrides`), the placed chest, creature
  and store holding `w1_held` under an old name (Charges 7, stacks of 3
  and 2), the creature's held with Dropable and Pickpocketable, a store
  at mark up 137 and down 35 selling it (Infinite) and 50 arrows of no
  cost; mirrors (appearance 447) with visual transforms: scale 0.46 and X
  Rotation -50.8, Y Rotation 40, a static one at scale 2, a plain one; a
  sound of random ranges 6 and 2, distances 3 and 12; lighting scheme 5;
  items named in German then French (`w1_lang`) and English then German,
  of categories 200 and 255, `nw_w1copy007`; a placeable blueprint with a
  visual transform (`w1_vtbp`); its own placeable skeleton with Special's
  categories named every way, its palcus to match; a stale creature
  entry (CR 9, Merchant); a portrait of its own (`po_w1po_*`, row 824 of
  the hak `w1hak`'s portraits.2da). `erf/w1_existing.erf` holds a script.
- `pal2-before.mod` (`w1pal2.mod`): a skeleton whose categories are named
  every way and a new one (23), a palcus with other names (`Cus A`...)
  and its top level reversed; items named German then English, English
  then German, German then French; the creature holding `w1_held` (stack
  2, both flags) and its equipped sword with both flags; a dynamic
  placeable blueprint with a visual transform (`w1_vtdyn`).

Then, in Aurora (tools/aurora/headless.sh, run-aurora.sh; xdrive.py), each
capture the saved module copied from the oracle's `modules/`:

1. Open `w1probe`, File > Save As: `saveas.mod` (palettes re-sorted, the
   stale creature entry made right).
2. Open the area. Toolbar Use Area Lighting off and on (`unlit-pitch.png`,
   `lit-pitch.png` in target/test-output); the sound's ranges from above
   and pitched (`sound-pitched.png`); the mirrors (`aurora-vt.png`).
   Area Properties > Visual > Customize Environment: type past each
   field's limits; Always Dark; OK, OK, Save: `area-saved.mod` (the grass
   gone, a static mirror's transform gone).
3. Item palette, Custom, Refresh Palette, Save: `refresh-items.mod`.
   Longswords > NW Copy Sword > Edit Copy: the ResRef offered (Cancel).
4. The placed creature's Inventory: Dropable ticked on the potion,
   Pickpocketable cleared on the held sword, Dropable ticked and cleared
   on the arrows; OK, OK, Save: `inv-flags.mod`. Its Add To Palette, each
   item's ResRef offered, OK each, Save: `add-palette.mod`. (The new
   blueprint's Feats and Special Abilities lists read top and bottom.)
5. New Held Name > Update Instances, Yes, Yes, No, Save: `update-held.mod`.
6. VT Blueprint > Edit, Useable off, Static on, OK, Save: `static-bp.mod`.
7. Drag the chest, Escape before letting go, Save: `esc-drag.mod`. One
   Ctrl + wheel notch over the selected small mirror, Save: `wheel1.mod`;
   with nothing selected (the view unchanged).
8. Area Wizard, City Interior, Small: the Fade Geometry modes (in the
   field, `fadeF-*.png`); Interior Evil, Save: `scheme16.mod`; Tile
   Properties > Defaults on two tiles, OK, Save: `tile-defaults.mod`; OK
   alone on a third: `tile-ok.mod`.
9. w1_script: type, Ctrl+S (temp0 gets the .nss and .ncs, the .mod is not
   written). VT Blueprint > Export to `erf/w1_existing.erf`: Yes (append):
   `w1_existing-appended.erf`.
10. The store's limits and Inventory prices; a sound's, a trigger's and an
    encounter's limits (Cancel); Select Portrait (Human Male, Outsider
    Male); a creature's class list; a door's Generic Appearance list.
11. Hide creatures and encounters, Save, File > Exit; start again: every
    kind is shown.
12. Open `w1pal2`: Special shows `Cus A`... (the palcus's names). Save
    As: `pal2-saveas.mod`; Chest > Edit > Category > Sk New Category, OK,
    Save: `pal2-newcat.mod` (put in Battlefield); item Refresh Palette,
    Save: `pal2-items.mod`; New Held Name > Update Instances: as in 5,
    `pal2-update.mod`; the creature's Add To Palette: `pal2-addpal.mod`;
    placeable Refresh Palette, Save: `pal2-refresh2.mod`; VT Dynamic >
    Edit, OK, Save: `pal2-vtdyn.mod`. Place Field 3 1x2 in the corner,
    copy a tile and paste it over the group: refused.
"""

import copy
import os
import shutil
import subprocess
import tempfile
import types
import json

BIN = os.environ.get("NWN_TOOLS_BIN", os.path.expanduser("~/.local/opt/neverwinter/bin"))
ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
CAPTURES = os.path.join(ORACLE, "captures")
OUT = os.path.join(CAPTURES, "wave1")
USER = os.path.join(ORACLE, "userdir")
GIT = "field." + "git"
GAME = tempfile.mkdtemp()


def _load(path):
    out = subprocess.run([f"{BIN}/nwn_gff", "-i", path, "-k", "json"], capture_output=True, check=True)
    return json.loads(out.stdout)


def _save(data, path):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(data, f)
    subprocess.run([f"{BIN}/nwn_gff", "-i", f.name, "-l", "json", "-o", path, "-k", "gff"],
                   check=True, capture_output=True)
    os.unlink(f.name)


gffu = types.SimpleNamespace(load=_load, save=_save, BIN=BIN)


def game_file(name, resource=None):
    """A game resource, from the game's own files (mg cat), in a scratch folder."""
    path = os.path.join(GAME, name)
    if not os.path.exists(path):
        with open(path, "wb") as f:
            subprocess.run(["mg", "--no-user-dir", "cat", resource or name], stdout=f, check=True)
    return path


def T(t, v):
    return {"type": t, "value": v}


def axis(v):
    return {"__struct_id": 0, "TimerType": T("int", 0), "ValueTo": T("float", v), "LerpType": T("int", 0)}


def vt(scale=1.0, rot=(0.0, 0.0, 0.0)):
    e = {"__struct_id": 6, "Scope": T("int", 0), "AnimationSpeed": T("struct", axis(1.0))}
    for a, s in zip("XYZ", (scale,) * 3):
        e["Scale" + a] = T("struct", axis(s))
    for a, r in zip("XYZ", rot):
        e["Rotate" + a] = T("struct", axis(r))
    for a in "XYZ":
        e["Translate" + a] = T("struct", axis(0.0))
    for k in list(e):
        if isinstance(e[k], dict) and e[k].get("type") == "struct":
            e[k]["__struct_id"] = 0
    return T("list", [e])


def held(uti, **fields):
    """A placed copy of item blueprint `uti` (as a GIT's ItemList entry)."""
    g = copy.deepcopy(uti)
    for k in ("Comment", "PaletteID", "__data_type"):
        g.pop(k, None)
    g["__struct_id"] = 0
    for k, v in {"Repos_PosX": T("word", 0), "Repos_Posy": T("word", 0)}.items():
        g.setdefault(k, v)
    for k, v in fields.items():
        g[k] = v
    return g


def make_probe():
    os.makedirs(OUT, exist_ok=True)
    base = tempfile.mkdtemp()
    subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", os.path.join(CAPTURES, "placement-probe.mod")],
                   cwd=base, check=True, capture_output=True)
    p = lambda n: os.path.join(base, n)  # noqa: E731

    with open(p("w1_script.nss"), "w") as f:
        f.write("void main()\n{\n}\n")

    sword = gffu.load(p("mgp_uti.uti"))

    def item(resref, name, **fields):
        g = copy.deepcopy(sword)
        g["LocalizedName"] = T("cexolocstring", name)
        g["Tag"] = T("cexostring", resref.upper())
        g["TemplateResRef"] = T("resref", resref)
        for k, v in fields.items():
            g[k] = v
        gffu.save(g, p(resref + ".uti"))
        return g

    # German (2) and French (1) only, German first in the file.
    item("w1_lang", {"2": "Deutscher Name", "1": "Nom francais"})
    item("w1_langen", {"0": "English Name", "2": "Deutscher Name"})
    held_bp = item("w1_held", {"0": "New Held Name"}, Cost=T("dword", 30))
    item("w1_nocat", {"0": "No Category Sword"}, PaletteID=T("byte", 200))
    item("w1_cat255", {"0": "Category 255 Sword"}, PaletteID=T("byte", 255))
    arrow = item("w1_arrow", {"0": "Probe Arrows"}, BaseItem=T("int", 20), StackSize=T("word", 50),
                 Cost=T("dword", 0), ModelPart1=T("byte", 11), ModelPart2=T("byte", 11),
                 ModelPart3=T("byte", 11))
    # An nw_ blueprint of the module's own, for Copy against Add to Palette.
    item("nw_w1copy007", {"0": "NW Copy Sword"})

    # Placeable blueprint with a visual transform, dynamic.
    utp = gffu.load(p("mgp_utp.utp"))
    vtbp = copy.deepcopy(utp)
    vtbp["TemplateResRef"] = T("resref", "w1_vtbp")
    vtbp["Tag"] = T("cexostring", "W1_VTBP")
    vtbp["LocName"] = T("cexolocstring", {"0": "VT Blueprint"})
    vtbp["Appearance"] = T("dword", 447)
    vtbp["HasInventory"] = T("byte", 0)
    vtbp["ItemList"] = T("list", [])
    vtbp["Static"] = T("byte", 0)
    vtbp["VisTransformList"] = vt(0.46, (-50.8, 0.0, 0.0))
    gffu.save(vtbp, p("w1_vtbp.utp"))

    git = gffu.load(p(GIT))
    ap = git["AreaProperties"]["value"]
    ap["GrassDefDisabled"] = T("byte", 1)
    ap["GrassOverrides"] = {"__struct_id": 101, "type": "struct", "value": {
        "__struct_id": 101,
        "GrassOvrList": T("list", [{
            "__struct_id": 0, "GrassMaterialId": T("byte", 3),
            "GrassTexture": T("cexostring", "grass_ttr01"),
            "GrassDensity": T("float", 0.5), "GrassHeight": T("float", 1.25),
            "GrassAmbientX": T("float", 0.2), "GrassAmbientY": T("float", 0.3),
            "GrassAmbientZ": T("float", 0.4), "GrassDiffuseX": T("float", 0.9),
            "GrassDiffuseY": T("float", 0.1), "GrassDiffuseZ": T("float", 0.1)}])}}

    # Held items: the chest's and the creature's packs, the store's pages.
    old = dict(LocalizedName=T("cexolocstring", {"0": "Old Held Name"}), Cost=T("dword", 99),
               Charges=T("byte", 7))
    chest = git["Placeable List"]["value"][0]
    chest["ItemList"]["value"].append(held(held_bp, StackSize=T("word", 3), **old,
                                           Repos_PosX=T("word", 2)))
    cre = git["Creature List"]["value"][0]
    cre["ItemList"]["value"].append(held(held_bp, StackSize=T("word", 2), Dropable=T("byte", 1),
                                         Pickpocketable=T("byte", 1), **old,
                                         Repos_PosX=T("word", 2)))
    cre["ItemList"]["value"].append(held(arrow, Repos_PosX=T("word", 4)))
    store = git["StoreList"]["value"][0]
    store["MarkUp"] = T("int", 137)
    store["MarkDown"] = T("int", 35)
    pages = store["StoreList"]["value"]
    # Page struct ids: 0 armor, 1 misc, 2 potions, 3 rings, 4 weapons (the probe's).
    weapons = next(pg for pg in pages if pg["__struct_id"] == 4)
    weapons["ItemList"]["value"].append(held(held_bp, Infinite=T("byte", 1), **old,
                                             Repos_PosX=T("word", 2)))
    weapons["ItemList"]["value"].append(held(arrow, Repos_PosX=T("word", 4)))

    # Mirrors with a visual transform: dynamic, and static.
    for tag, x, static, scale, rot in [("W1_VT_DYN", 25.0, 0, 0.46, (-50.8, 0.0, 0.0)),
                                       ("W1_VT_TILT", 28.0, 0, 1.0, (0.0, 40.0, 0.0)),
                                       ("W1_VT_STATIC", 31.0, 1, 2.0, (0.0, 0.0, 0.0)),
                                       ("W1_PLAIN", 34.0, 0, 1.0, (0.0, 0.0, 0.0))]:
        m = copy.deepcopy(chest)
        m["Tag"] = T("cexostring", tag)
        m["LocName"] = T("cexolocstring", {"0": tag})
        m["Appearance"] = T("dword", 447)
        m["HasInventory"] = T("byte", 0)
        m["ItemList"] = T("list", [])
        m["Static"] = T("byte", static)
        m["X"] = T("float", x)
        m["Y"] = T("float", 20.0)
        if tag != "W1_PLAIN":
            m["VisTransformList"] = vt(scale, rot)
        git["Placeable List"]["value"].append(m)
    snd = git["SoundList"]["value"][0]
    snd["RandomRangeX"] = T("float", 6.0)
    snd["RandomRangeY"] = T("float", 2.0)
    snd["MinDistance"] = T("float", 3.0)
    snd["MaxDistance"] = T("float", 12.0)
    gffu.save(git, p(GIT))
    gic = gffu.load(p("field.gic"))
    for _ in range(4):
        gic["Placeable List"]["value"].append(
            {"__struct_id": 9, "Comment": T("cexostring", "")})
    gffu.save(gic, p("field.gic"))

    are = gffu.load(p("field.are"))
    are["LightingScheme"] = T("byte", 5)
    gffu.save(are, p("field.are"))

    # The module's own placeable skeleton: names of every kind, out of order.
    skel = gffu.load(game_file("placeablepal.itp"))
    special = next(n for n in skel["MAIN"]["value"] if n.get("DELETE_ME", {}).get("value") == "Special")
    cats = special["LIST"]["value"]

    def node(id_, **names):
        n = {"__struct_id": 1, "ID": T("byte", id_)}
        for k, v in names.items():
            n[k] = T("dword", v) if k == "STRREF" else T("cexostring", v)
        return n

    cats[:] = [
        node(0, NAME="Zeta Written", STRREF=6689),          # NAME and STRREF ("Custom 2")
        node(1, DELETE_ME="Delete Me Only"),                 # DELETE_ME alone
        node(2, DELETE_ME="Delete Me Text", STRREF=6690),   # DELETE_ME and STRREF ("Custom 3")
        node(3, NAME="Alpha Written"),                       # NAME alone
    ] + [c for c in cats if c["ID"]["value"] > 3]
    gffu.save(skel, p("placeablepal.itp"))
    # The custom palette made to match it, in the same order, with mgp_utp
    # (category 6 in the game's) moved to Alpha Written (3).
    pal = gffu.load(p("placeablepalcus.itp"))

    special_cus = next(n for n in pal["MAIN"]["value"] if n.get("STRREF", {}).get("value") == 6687)
    entry = {"__struct_id": 0, "NAME": T("cexostring", "Chest"), "RESREF": T("resref", "mgp_utp")}

    def drop_entry(nodes):
        for s in nodes:
            if "LIST" in s:
                s["LIST"]["value"] = [e for e in s["LIST"]["value"]
                                      if e.get("RESREF", {}).get("value") != "mgp_utp"]
                drop_entry(s["LIST"]["value"])

    drop_entry(pal["MAIN"]["value"])
    special_cus["LIST"]["value"] = [
        {"__struct_id": 1, "ID": T("byte", 0), "STRREF": T("dword", 6689)},
        {"__struct_id": 1, "ID": T("byte", 1)},
        {"__struct_id": 1, "ID": T("byte", 2), "STRREF": T("dword", 6690)},
        {"__struct_id": 1, "ID": T("byte", 3), "LIST": T("list", [entry])},
    ] + [c for c in special_cus["LIST"]["value"] if c.get("ID", {}).get("value", 0) > 3]
    gffu.save(pal, p("placeablepalcus.itp"))
    mgp = gffu.load(p("mgp_utp.utp"))
    mgp["PaletteID"] = T("byte", 3)
    gffu.save(mgp, p("mgp_utp.utp"))

    # A stale creature entry: the blueprint is CR 0.5 and Hostile.
    cpal = gffu.load(p("creaturepalcus.itp"))

    def stale(nodes):
        for s in nodes:
            if s.get("RESREF", {}).get("value") == "mgp_utc":
                s["CR"] = T("float", 9.0)
                s["FACTION"] = T("cexostring", "Merchant")
            if "LIST" in s:
                stale(s["LIST"]["value"])

    stale(cpal["MAIN"]["value"])
    gffu.save(cpal, p("creaturepalcus.itp"))

    # A portrait of the module's own (its row in the hak's portraits.2da).
    for size in "hlmst":
        shutil.copy(game_file(f"po_{size}.tga", f"po_hu_m_01_{size}.tga"), p(f"po_w1po_{size}.tga"))
    hak = tempfile.mkdtemp()
    lines = open(game_file("portraits.2da"), encoding="latin-1").read().split("\n")
    for i, line in enumerate(lines):
        if line.startswith("824 "):
            lines[i] = "824   w1po_        0    6    ****          0    ****     "
    open(os.path.join(hak, "portraits.2da"), "w").write("\n".join(lines))
    hak_path = os.path.join(OUT, "w1hak.hak")
    if os.path.exists(hak_path):
        os.unlink(hak_path)
    subprocess.run([f"{BIN}/nwn_erf", "-e", "HAK", "-c", "-f", hak_path, "portraits.2da"], cwd=hak,
                   check=True, capture_output=True)
    shutil.rmtree(hak)
    shutil.copy(hak_path, os.path.join(USER, "hak", "w1hak.hak"))

    ifo = gffu.load(p("module.ifo"))
    ifo["Mod_Name"] = T("cexolocstring", {"0": "w1probe"})
    ifo["Mod_HakList"] = T("list", [{"__struct_id": 8, "Mod_Hak": T("cexostring", "w1hak")}])
    gffu.save(ifo, p("module.ifo"))

    out = os.path.join(OUT, "before.mod")
    if os.path.exists(out):
        os.unlink(out)
    files = sorted(os.listdir(base))
    subprocess.run([f"{BIN}/nwn_erf", "-e", "MOD", "-c", "-f", out] + files, cwd=base, check=True,
                   capture_output=True)
    shutil.rmtree(base)
    shutil.copy(out, os.path.join(USER, "modules", "w1probe.mod"))
    # An archive that exists, for Export.
    os.makedirs(os.path.join(USER, "erf"), exist_ok=True)
    erf = os.path.join(USER, "erf", "w1_existing.erf")
    tmp = tempfile.mkdtemp()
    with open(os.path.join(tmp, "w1_keep.nss"), "w") as f:
        f.write("void main() {}\n")
    if os.path.exists(erf):
        os.unlink(erf)
    subprocess.run([f"{BIN}/nwn_erf", "-e", "ERF", "-c", "-f", erf, "w1_keep.nss"], cwd=tmp,
                   check=True, capture_output=True)
    shutil.copy(erf, os.path.join(OUT, "w1_existing-before.erf"))
    shutil.rmtree(tmp)
    print("made", out)


def make_pal2():
    base = tempfile.mkdtemp()
    subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", os.path.join(CAPTURES, "placement-probe.mod")],
                   cwd=base, check=True, capture_output=True)
    p = lambda n: os.path.join(base, n)  # noqa: E731
    sword = gffu.load(p("mgp_uti.uti"))

    def item(resref, name, **fields):
        g = copy.deepcopy(sword)
        g["LocalizedName"] = T("cexolocstring", name)
        g["Tag"] = T("cexostring", resref.upper())
        g["TemplateResRef"] = T("resref", resref)
        g.update(fields)
        gffu.save(g, p(resref + ".uti"))
        return g

    item("w1_lang", {"2": "Deutscher Name", "1": "Nom francais"})
    item("w1_langde", {"2": "Deutsch Zuerst", "0": "English Second"})
    item("w1_langen", {"0": "English First", "2": "Deutsch Zweite"})
    held_bp = item("w1_held", {"0": "New Held Name"})

    utp = gffu.load(p("mgp_utp.utp"))
    dyn = copy.deepcopy(utp)
    dyn["TemplateResRef"] = T("resref", "w1_vtdyn")
    dyn["Tag"] = T("cexostring", "W1_VTDYN")
    dyn["LocName"] = T("cexolocstring", {"0": "VT Dynamic"})
    dyn["Appearance"] = T("dword", 447)
    dyn["HasInventory"] = T("byte", 0)
    dyn["ItemList"] = T("list", [])
    dyn["VisTransformList"] = vt(0.46, (-50.8, 0.0, 0.0))
    gffu.save(dyn, p("w1_vtdyn.utp"))

    git = gffu.load(p(GIT))
    cre = git["Creature List"]["value"][0]
    cre["ItemList"]["value"].append(held(held_bp, StackSize=T("word", 2), Dropable=T("byte", 1),
                                         Pickpocketable=T("byte", 1), Charges=T("byte", 7),
                                         LocalizedName=T("cexolocstring", {"0": "Old Held Name"}),
                                         Repos_PosX=T("word", 2)))
    for it in cre["Equip_ItemList"]["value"]:
        if it.get("TemplateResRef", {}).get("value") == "nw_wswls001":
            it["Dropable"] = T("byte", 1)
            it["Pickpocketable"] = T("byte", 1)
    gffu.save(git, p(GIT))

    skel = gffu.load(game_file("placeablepal.itp"))
    special = next(n for n in skel["MAIN"]["value"] if n.get("DELETE_ME", {}).get("value") == "Special")

    def node(id_, **names):
        n = {"__struct_id": 1, "ID": T("byte", id_)}
        for k, v in names.items():
            n[k] = T("dword", v) if k == "STRREF" else T("cexostring", v)
        return n

    special["LIST"]["value"] = [
        node(0, NAME="Sk Name And Strref", STRREF=6688),
        node(1, DELETE_ME="Sk Delete Me"),
        node(2, NAME="Sk Name Only"),
        node(3, STRREF=6691),
        node(4, NAME="Sk Both Moonglow", DELETE_ME="Sk Both Moonglow"),
        node(23, NAME="Sk New Category"),
    ]
    skel["NEXT_USEABLE_ID"] = T("byte", 24)
    gffu.save(skel, p("placeablepal.itp"))

    pal = gffu.load(p("placeablepalcus.itp"))
    sp = next(n for n in pal["MAIN"]["value"] if n.get("STRREF", {}).get("value") == 6687)
    cus = lambda id_, name: {"__struct_id": 1, "ID": T("byte", id_), "NAME": T("cexostring", name)}  # noqa: E731
    sp["LIST"]["value"] = [cus(0, "Cus A"), cus(1, "Cus B"), cus(2, "Cus C"), cus(3, "Cus D"),
                           cus(4, "Cus E")]
    pal["MAIN"]["value"].reverse()
    gffu.save(pal, p("placeablepalcus.itp"))

    ifo = gffu.load(p("module.ifo"))
    ifo["Mod_Name"] = T("cexolocstring", {"0": "w1pal2"})
    gffu.save(ifo, p("module.ifo"))
    out = os.path.join(OUT, "pal2-before.mod")
    if os.path.exists(out):
        os.unlink(out)
    subprocess.run([f"{BIN}/nwn_erf", "-e", "MOD", "-c", "-f", out] + sorted(os.listdir(base)),
                   cwd=base, check=True, capture_output=True)
    shutil.rmtree(base)
    shutil.copy(out, os.path.join(USER, "modules", "w1pal2.mod"))
    print("made", out)


def main():
    os.makedirs(OUT, exist_ok=True)
    for folder in ("modules", "hak", "erf"):
        os.makedirs(os.path.join(USER, folder), exist_ok=True)
    make_probe()
    make_pal2()
    shutil.rmtree(GAME)
    print(f"made {OUT}; now capture with Aurora as this script's doc says")


if __name__ == "__main__":
    main()
