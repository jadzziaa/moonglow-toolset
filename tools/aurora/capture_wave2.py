#!/usr/bin/env python3
"""Make the probe of the deferred sprint's second Aurora batch (2026-10-30),
and say how its captures were made.

    capture_wave2.py

It reads `captures/update-aurora-doors.mod` (two areas, `field` and
`field2`, two doors each), `captures/update-aurora-defaults.mod` (its
triggers) and the game (through `mg cat`), and writes `captures/wave2/`
`before.mod` (`w2probe.mod` in the oracle's `modules/`) and the talk
tables `w2.tlk` and `w2f.tlk` (into the oracle's `tlk/`):

- Unnumbered blueprints of the module's own: the placeable `w2chest`
  (holding the item `w2gem`), the creature `w2guard`; placed in `field`
  as `W2_CHEST` (holding a `w2gem`) and `W2_GUARD` (holding nothing).
- Two creatures of a wizard's class with no feats (no Martial Weapon
  Proficiency): `W2_INV_A`, hands empty, its pack a dagger, a hand axe
  and a torch; `W2_INV_B`, a longsword in its right hand.
- `w2prop`, an item with a Damage Bonus property (`ChanceAppear` 100).
- Waypoints `W2_WP_NONE` (no `Appearance` field) and `W2_WP_ZERO`
  (`Appearance` 0) beside the probe's own (1).
- Stock armors lying in `field`, one of each torso `ACBONUS` 0 to 8:
  `nw_cloth001` 0, `nw_aarcl009` 1, `nw_aarcl002` 3, `nw_aarcl012` 4,
  `nw_aarcl010` 5, `nw_aarcl011` 6, `nw_aarcl006` 7, `nw_aarcl007` 8,
  in a row along y = 8 from x = 4, 2 m apart.
- Tiles: `field` is the rural probe area (ttr01; `field2` the City
  Interior's), its tile 5 ttr01's tile 14 (`ttr01_b06_01`, its
  `Object654` of `tilefade` 4); its tiles 0 and 1 have lights and loops of
  their own (0: main lights 5 and 6, source lights 1 and 2, loops 1 0 1;
  1: 10 and 11, 3 and 4, loops 0 1 0).
- Doors `W2_D1`, `W2_D3`, `W2_D5` in `field` and `W2_D2`, `W2_D4`,
  `W2_D6` in `field2`; transition triggers `W2_T1` in `field` and
  `W2_T2` in `field2`.
- The custom talk table `w2` (`Mod_CustomTlk`): line 0 "W2 MASCULINE
  NAME" in `w2.tlk`, "W2 FEMININE NAME" in `w2f.tlk`; line 1 "W2
  MASCULINE LINE" and "W2 FEMININE LINE". The item `w2tlk` is named by
  line 0 (StrRef 16777216), the conversation `w2conv`'s one line is line
  1 (16777217).

Then, in Aurora (tools/aurora/headless.sh, run-aurora.sh; xdrive.py), one
session, in this order, each capture the saved module copied from the
oracle's `modules/` into `captures/wave2/` (the pictures, which are not
compared by tests, in `target/test-output/wave2/`):

1. Open `w2probe` (Ctrl+O), its `field`. The waypoints (`waypoints.png`:
   none, 0 and 1 alike, the yellow arrow). The armors from above
   (`armors.png`); a Leather Armor painted beside them. From the
   waypoint palette `nw_waypoint001`, `nw_mapnote001` and `nw_post001`
   (no `Appearance`) painted; Save: `waypoint-painted.mod` (each placed
   with `Appearance` 1).
2. Creature palette, Custom, a category's New (the Creature Wizard):
   Human, Next, Next, Select Portrait (Race and Gender ticked: the wizard's
   list, `cw-portraits*.png`); Select Palette Category (`cw8.png`,
   `cw9.png`: Monsters, NPCs, Special, Tutorial). Cancel.
3. `W2_GUARD`'s Properties > Advanced > Faction's list
   (`faction-list.png`: no PC). Cancel.
4. `W2_INV_A`'s Inventory: the dagger, then (Cancel, reopened) the hand
   axe, then the torch dragged onto the secondary slot (`inv-a-*.png`:
   the missing feat asked, Yes, each in the primary; the torch in the
   secondary). `W2_INV_B`'s: the longsword dragged from the primary onto
   the secondary, No, then Yes, then No and OK (`inv-b-*.png`: it stays
   in the primary).
5. Add to Palette, the ResRef read off each dialog's field before OK:
   `W2_CHEST` (w2gem002, w2chest001), `W2_GUARD` (w2guard002), `W2_GUARD`
   again (w2guard004), `W2_CHEST` again (w2gem004, w2chest002), `W2_D1`
   (mgp_utd001), `W2_T1` (mgq_utt001: its OK refused, Cancel), the
   painted `nw_waypoint001` (waypoint001, Cancel), a painted Dye Store
   (its warning answered No), a painted Bat Cries & Flaps
   (batcriesflaps001, Cancel), a painted Evil Metal Door; Save:
   `add-palette.mod`. Then a painted Rubble (`x3_plc_rubble1`, its tag
   changed: plc_rubble1, OK; again: rubble1, OK; again: rubble002,
   Cancel) and Skull Pole (`x0_skullpole`: skullpole, OK; again:
   skullpole001, OK).
6. `w2prop` (custom palette) > Edit > Properties: Edit Property on its
   Damage Bonus (`spp1.png`: no Chance of Appearing shown); Damage Bonus
   vs. Specific Alignment, Chaotic Evil, added (`ip4.png`: the choices by
   name), its Edit Property (`spp3.png`: Parameter 1 Values in
   iprp_damagetype's order). OK, Save: `chance.mod`.
7. `w2conv` opened, its line selected (`tlk-conv2.png`: "W2 MASCULINE
   LINE"); the palette's `w2tlk` reads "W2 MASCULINE NAME".
8. Select Terrain: tiles 0 and 1 by a drag, Tile Properties (`tp1.png`),
   Main Light 1 changed alone (`tp3.png`), OK, Save: `tiles-one.mod`.
9. Setup Area Transition (Target Area Field 2): `W2_D1` Both Ways to
   `W2_D2`, `W2_D3` Current To Target to `W2_D4`, `W2_D5` Target To
   Current to `W2_D6` (the first asks about the current object's tag;
   each asks to save the area); Save: `transitions.mod`; `W2_D5` again,
   Save: `transitions2.mod`. `W2_T1`'s Setup Area Transition with Target
   Type Trigger stopped at "List index out of bounds (-1)".
10. Fade Geometry (Environment) Never and Always on tile 5, from above
   (`fade-never.png`, `fade-always.png`) and pitched from the south-west
   (`fade-*-pitched.png`: Always keeps `Object654`'s black slab, as
   `mgv render` of the tile with and without it shows, `fade-mgv-ab.png`).
11. Open `w1probe` (wave 1's). Creature Wizard, Human, Male, Select
   Portrait, scrolled through (`w1cw*.png`: `w1po_` between `hu_m_53_` and
   `hu_m_40_`). The tree: F2 on Bandit and on `w1_script` (nothing
   edited, `keys-f2*.png`); the placeable palette's Arrow-filled corpse
   chosen, Up to Battlefield: the area still in Paint (`keys-hand*.png`).
"""

import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from capture_wave1 import BIN, CAPTURES, GIT, USER, T, game_file, gffu, held  # noqa: E402

OUT = os.path.join(CAPTURES, "wave2")
GIC = "field.gic"


def extract(archive, into):
    subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", os.path.join(CAPTURES, archive)], cwd=into,
                   check=True, capture_output=True)


def game_gff(name):
    return gffu.load(game_file(name))


def placed_item(resref, **fields):
    """A stock item, as held or lying."""
    return held(game_gff(resref + ".uti"), **fields)


def tlk(path, lines):
    entries = [{"id": i, "text": t} for i, t in enumerate(lines)]
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump({"language": 0, "entries": entries}, f)
    subprocess.run([f"{BIN}/nwn_tlk", "-i", f.name, "-l", "json", "-o", path, "-k", "tlk"],
                   check=True, capture_output=True)
    os.unlink(f.name)


def make_probe():
    os.makedirs(OUT, exist_ok=True)
    base = tempfile.mkdtemp()
    extract("update-aurora-doors.mod", base)
    other = tempfile.mkdtemp()
    extract("update-aurora-defaults.mod", other)
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

    gem = item("w2gem", {"0": "W2 Gem Sword"})
    # A Damage Bonus (16) of fire (10), 1d4 (cost table 4, value 6).
    prop = {"__struct_id": 0, "PropertyName": T("word", 16), "Subtype": T("word", 10),
            "CostTable": T("byte", 4), "CostValue": T("word", 6), "Param1": T("byte", 255),
            "Param1Value": T("byte", 0), "ChanceAppear": T("byte", 100)}
    item("w2prop", {"0": "W2 Property Sword"}, PropertiesList=T("list", [prop]))
    item("w2tlk", {"id": 16777216})

    chest_bp = gffu.load(p("mgp_utp.utp"))
    chest_bp["TemplateResRef"] = T("resref", "w2chest")
    chest_bp["Tag"] = T("cexostring", "W2CHEST")
    chest_bp["LocName"] = T("cexolocstring", {"0": "W2 Chest"})
    chest_bp["HasInventory"] = T("byte", 1)
    chest_bp["ItemList"] = T("list", [{"__struct_id": 0, "InventoryRes": T("resref", "w2gem"),
                                       "Repos_PosX": T("word", 0), "Repos_Posy": T("word", 0)}])
    gffu.save(chest_bp, p("w2chest.utp"))
    guard_bp = gffu.load(p("mgp_utc.utc"))
    guard_bp["TemplateResRef"] = T("resref", "w2guard")
    guard_bp["Tag"] = T("cexostring", "W2GUARD")
    guard_bp["FirstName"] = T("cexolocstring", {"0": "W2 Guard"})
    guard_bp["Equip_ItemList"] = T("list", [])
    guard_bp["ItemList"] = T("list", [])
    gffu.save(guard_bp, p("w2guard.utc"))

    git = gffu.load(p(GIT))
    lists = git
    chest0 = lists["Placeable List"]["value"][0]
    chest = copy.deepcopy(chest0)
    for k in ("TemplateResRef", "Tag", "LocName", "HasInventory"):
        chest[k] = chest_bp[k] if k != "Tag" else T("cexostring", "W2_CHEST")
    chest["ItemList"] = T("list", [held(gem, Repos_PosX=T("word", 0))])
    chest["X"], chest["Y"] = T("float", 10.0), T("float", 30.0)
    lists["Placeable List"]["value"].append(chest)

    cre0 = lists["Creature List"]["value"][0]

    def creature(tag, x, y, **fields):
        c = copy.deepcopy(cre0)
        c["Tag"] = T("cexostring", tag)
        c["FirstName"] = T("cexolocstring", {"0": tag})
        c["LastName"] = T("cexolocstring", {})
        c["Equip_ItemList"] = T("list", [])
        c["ItemList"] = T("list", [])
        c["XPosition"], c["YPosition"] = T("float", x), T("float", y)
        c.update(fields)
        lists["Creature List"]["value"].append(c)
        return c

    creature("W2_GUARD", 14.0, 30.0, TemplateResRef=T("resref", "w2guard"))
    wizard = dict(ClassList=T("list", [{"__struct_id": 2, "Class": T("int", 10),
                                        "ClassLevel": T("short", 1)}]),
                  FeatList=T("list", []), TemplateResRef=T("resref", "mgp_utc"))
    creature("W2_INV_A", 18.0, 30.0, **wizard,
             ItemList=T("list", [placed_item("nw_wswdg001", Repos_PosX=T("word", 0)),
                                 placed_item("nw_waxhn001", Repos_PosX=T("word", 1)),
                                 placed_item("nw_it_torch001", Repos_PosX=T("word", 3))]))
    sword_held = placed_item("nw_wswls001")
    for k in ("Repos_PosX", "Repos_Posy"):
        sword_held.pop(k, None)
    sword_held["__struct_id"] = 16
    creature("W2_INV_B", 22.0, 30.0, **wizard, Equip_ItemList=T("list", [sword_held]))

    wp0 = lists["WaypointList"]["value"][0]
    for tag, x, app in [("W2_WP_NONE", 20.0, None), ("W2_WP_ZERO", 22.0, 0)]:
        w = copy.deepcopy(wp0)
        w["Tag"] = T("cexostring", tag)
        w["LocalizedName"] = T("cexolocstring", {"0": tag})
        w["XPosition"], w["YPosition"] = T("float", x), T("float", 35.0)
        if app is None:
            w.pop("Appearance", None)
        else:
            w["Appearance"] = T("byte", app)
        lists["WaypointList"]["value"].append(w)
    wp0["XPosition"], wp0["YPosition"] = T("float", 18.0), T("float", 35.0)

    lying = lists.setdefault("List", T("list", []))["value"]
    for i, ref in enumerate(["nw_cloth001", "nw_aarcl009", "nw_aarcl002", "nw_aarcl012",
                             "nw_aarcl010", "nw_aarcl011", "nw_aarcl006", "nw_aarcl007"]):
        a = placed_item(ref)
        for k in ("Repos_PosX", "Repos_Posy"):
            a.pop(k, None)
        a["__struct_id"] = 0
        a.update(XPosition=T("float", 4.0 + 2 * i), YPosition=T("float", 8.0),
                 ZPosition=T("float", 0.0), XOrientation=T("float", 0.0),
                 YOrientation=T("float", 1.0), Tag=T("cexostring", "W2_ARMOR_" + ref.upper()))
        lying.append(a)

    doors = lists["Door List"]["value"]
    d1 = copy.deepcopy(doors[0])
    doors[:] = []
    for tag, x in [("W2_D1", 6.0), ("W2_D3", 9.0), ("W2_D5", 12.0)]:
        d = copy.deepcopy(d1)
        d["Tag"] = T("cexostring", tag)
        d["LocName"] = T("cexolocstring", {"0": tag})
        d["X"], d["Y"], d["Bearing"] = T("float", x), T("float", 20.0), T("float", 0.0)
        doors.append(d)

    other_git = gffu.load(os.path.join(other, GIT))
    trig = copy.deepcopy(other_git["TriggerList"]["value"][0])
    trig["Type"] = T("int", 1)
    trig["Tag"] = T("cexostring", "W2_T1")
    trig["LocalizedName"] = T("cexolocstring", {"0": "W2_T1"})
    trig["XPosition"], trig["YPosition"] = T("float", 30.0), T("float", 8.0)
    lists.setdefault("TriggerList", T("list", []))["value"][:] = [trig]
    gffu.save(git, p(GIT))

    git2 = gffu.load(p("field2." + "git"))
    doors2 = git2["Door List"]["value"]
    doors2[:] = []
    for tag, x in [("W2_D2", 6.0), ("W2_D4", 9.0), ("W2_D6", 12.0)]:
        d = copy.deepcopy(d1)
        d["Tag"] = T("cexostring", tag)
        d["LocName"] = T("cexolocstring", {"0": tag})
        d["X"], d["Y"], d["Bearing"] = T("float", x), T("float", 20.0), T("float", 0.0)
        doors2.append(d)
    t2 = copy.deepcopy(trig)
    t2["Tag"] = T("cexostring", "W2_T2")
    t2["LocalizedName"] = T("cexolocstring", {"0": "W2_T2"})
    git2.setdefault("TriggerList", T("list", []))["value"][:] = [t2]
    gffu.save(git2, p("field2." + "git"))
    # The area's comments file follows its lists.
    for area, g in (("field", git), ("field2", git2)):
        gic = gffu.load(p(area + ".gic"))
        for name in ("Creature List", "Door List", "List", "Placeable List", "TriggerList",
                     "WaypointList"):
            n = len(g.get(name, {}).get("value", []))
            gic[name] = T("list", [{"__struct_id": 0, "Comment": T("cexostring", "")}
                                   for _ in range(n)])
        gffu.save(gic, p(area + ".gic"))

    # field is the rural probe area (ttr01, the placement probe's), where
    # Aurora draws the objects; field2 stays the City Interior (tic01).
    rural = tempfile.mkdtemp()
    extract("placement-probe.mod", rural)
    shutil.copy(os.path.join(rural, "field.are"), p("field.are"))
    shutil.rmtree(rural)

    are = gffu.load(p("field.are"))
    tiles = are["Tile_List"]["value"]
    tiles[5]["Tile_ID"] = T("int", 14)
    for i, (m1, m2, s1, s2, loops) in enumerate([(5, 6, 1, 2, (1, 0, 1)),
                                                 (10, 11, 3, 4, (0, 1, 0))]):
        tiles[i]["Tile_MainLight1"], tiles[i]["Tile_MainLight2"] = T("byte", m1), T("byte", m2)
        tiles[i]["Tile_SrcLight1"], tiles[i]["Tile_SrcLight2"] = T("byte", s1), T("byte", s2)
        for n, v in zip("123", loops):
            tiles[i]["Tile_AnimLoop" + n] = T("byte", v)
    gffu.save(are, p("field.are"))

    dlg = {"__data_type": "DLG ", "DelayEntry": T("dword", 0), "DelayReply": T("dword", 0),
           "EndConverAbort": T("resref", ""), "EndConversation": T("resref", ""),
           "NumWords": T("dword", 0), "PreventZoomIn": T("byte", 0),
           "EntryList": T("list", [{"__struct_id": 0, "Speaker": T("cexostring", ""),
                                    "AnimLoop": T("byte", 1), "Animation": T("dword", 0),
                                    "Text": T("cexolocstring", {"id": 16777217}),
                                    "Script": T("resref", ""), "Delay": T("dword", 4294967295),
                                    "Comment": T("cexostring", ""), "Sound": T("resref", ""),
                                    "Quest": T("cexostring", ""),
                                    "RepliesList": T("list", [])}]),
           "ReplyList": T("list", []),
           "StartingList": T("list", [{"__struct_id": 0, "Index": T("dword", 0),
                                       "Active": T("resref", "")}])}
    gffu.save(dlg, p("w2conv.dlg"))

    ifo = gffu.load(p("module.ifo"))
    ifo["Mod_Name"] = T("cexolocstring", {"0": "w2probe"})
    ifo["Mod_CustomTlk"] = T("cexostring", "w2")
    gffu.save(ifo, p("module.ifo"))

    os.makedirs(os.path.join(USER, "tlk"), exist_ok=True)
    for name, gender in (("w2.tlk", "MASCULINE"), ("w2f.tlk", "FEMININE")):
        path = os.path.join(OUT, name)
        tlk(path, [f"W2 {gender} NAME", f"W2 {gender} LINE"])
        shutil.copy(path, os.path.join(USER, "tlk", name))

    out = os.path.join(OUT, "before.mod")
    if os.path.exists(out):
        os.unlink(out)
    subprocess.run([f"{BIN}/nwn_erf", "-e", "MOD", "-c", "-f", out] + sorted(os.listdir(base)),
                   cwd=base, check=True, capture_output=True)
    shutil.rmtree(base)
    shutil.rmtree(other)
    shutil.copy(out, os.path.join(USER, "modules", "w2probe.mod"))
    print("made", out)


if __name__ == "__main__":
    make_probe()
