#!/usr/bin/env python3
"""Say how the Setup Area Transition captures with triggers were made (2026-11-12), and
make the one step done outside Aurora (the doors).

    capture_transition_triggers.py inject-doors MODULE.mod

Everything else was done in Aurora itself (tools/aurora/headless.sh, run-aurora.sh,
xdrive.py), one module, `ttprobe` in the oracle's `modules/`. After each step the module
Aurora saved was copied to `captures/transition-triggers/NN-step.mod`, and the area files
of Aurora's `modules/temp0/` were copied before and after each dialog's OK (into
`captures/transition-triggers/temp0/STEP-pre`, `-post-ok`...), which tells what the dialog
wrote from what the module's Save wrote. The pictures are not compared by tests; they were
kept in `target/aurora-transition-triggers/shots/`.

00-base: Module Wizard `ttprobe`, Area Wizard `Alpha` and `Beta`, Castle Exterior, Rural
  (tno01), Small (4 x 4). Its start location (20, 20, 0) was refused as invalid when the
  area was first saved; repainted with Paint Start Location.
01-painted: triggers painted from the Standard palette, three clicks and a double-click
  on the fourth corner (a single click on the first corner does not close a trigger). In
  Alpha, Area Transition (`newtransition`) quads (0,0) (5,0) (6,3) (1,4) at (2,2) (12,2)
  (22,2) (2,12) (12,12) (22,12) (2,22) (12,22), New Generic at (22,22); in Beta an Area
  Transition at (8,8)-(16,14), New Generic at (22,8)-(28,13), Average Holy Trap at
  (30,20)-(34,24), a Waypoint near (30,30), Area Transitions at (1,30) (10,30) (18,30)
  (1,18) (10,18) (18,18).
02-retagged: Name and Tag set in each trigger's Properties: TT_A1..TT_A7 (A1 is the one
  at (12,12)), TT_B2..TT_B6, TT_SHARED on Beta's (8,8) and (18,18) triggers; Alpha's
  (12,22) keeps its blueprint's tag `AreaTransition`.
03-doors: doors TT_AD1 (34,4), TT_AD2 (34,10) in Alpha and TT_BD1 (34,8), TT_BD2 (34,14)
  in Beta written by `inject-doors` below (Aurora on the off-screen display shows a
  palette door under the pointer but places none), then opened in Aurora.
04-base: Alpha: clockwise (2,30) (3,35) (8,34) (7,30); counterclockwise (12,30) (17,30)
  (18,34) (13,35); clockwise five corners (22,30) (22,34) (25,36) (28,34) (27,30); New
  Generic (30,18)..(30,22); Area Transition (30,26)..(30,29); clockwise (30,32) (30,36)
  (35,36) (34,32) closed by a double-click on the first corner.
05-concave: Beta: Area Transition (20,1.5) (28,1.5) (28,6) (24,3) (20,6).
06-typechange: Alpha's generic (30,18) set to Area Transition, the (30,26) one to Generic.
Then Setup Area Transition (a trigger's Properties > Area Transition > Setup Area
Transition; a door's the same), each run answered Yes to every prompt unless said, and
Properties' OK, then the module saved:
07-s2-both: TT_A1 to Beta's TT_B2, Trigger, Both Ways.
08-s3a-cur: TT_A2 to TT_B3, Current To Target.
09-s3b-tgt: TT_A3 from TT_B4, Target To Current (TT_B4's link is written only at 11).
10-s4-same: TT_A4 to TT_A5 (both in Alpha), Both Ways.
11-s5a-door-to-trig: door TT_AD1 to TT_B5, Trigger, Both Ways.
12-s5b-trig-to-door: TT_A6 to door TT_BD1, Door, Both Ways.
13-s6-trig-to-wp: TT_A7 to Beta's NW_WAYPOINT001, Waypoint (Current To Target).
14-s7-relink-shared: TT_A1 again, to Beta's (18,18) TT_SHARED, Both Ways.
15-s7b-relink-b6: TT_A1 again, to TT_B6: the overwrite prompt answered No first, then
  the run repeated with Yes.
16-s8-bptag-yes-props-cancel: Alpha's (12,22) AreaTransition (its blueprint's tag) to
  Beta's (8,8) TT_SHARED: the CURRENT-tag prompt No, then again Yes; Properties
  Cancelled.
17-s9-bptag-both-z: Alpha's (12,30) to Beta's concave AreaTransition: TARGET-tag prompt
  No, then the save prompt No, then Yes to all.
18-ground-probe-wp: a palette Waypoint painted near (15, 32.4) in Alpha.
19-s3c-tgt-repeat: Alpha's (2,30) from TT_B2, Target To Current (overwrite prompt names
  TT_B2).
20-s3d-flush: Alpha's (30,18) (GenericTrigger, Type 1) to TT_B3, Both Ways: its save of
  Beta writes 19's TT_B2 link.
21-r-z-recheck: Alpha's (30,32) to its (22,30), Both Ways, the save prompt No first.
22-r-untagged: Alpha's (2,30) Tag cleared in Properties (Aurora sets NewAreaTransition).
23-r-both-onto-linked: Alpha's (2,30) to TT_B2 (linked), Both Ways.

Round two (heights and the doors' one-way links), area `Gamma` (Area Wizard, Castle
Exterior Rural, 4 x 4):
24: Terrain > Grass Raise/Lower clicked at the corner (30,30). 25: palette waypoints as
  ground probes (a waypoint lands 0.1-0.3 m from the click). 26: six 3 x 3 m Area
  Transitions at (6,6) (6,12) (12,6) (28,28) (6,30) (12,12).
27-32: the (6,6) one dragged onto the slope, back, and half onto it, probes after each.
33-35: the (12,6) one copied (Ctrl+C, Ctrl+V, a click) onto the slope, probed, dragged.
36: Redraw Polygon on that pasted one, at (2,20). 37: the copy pasted into Beta.
38-41: Raise at the corner (10,10), Lower (right-click) at (30,30), probes.
42: the (6,30) one Shift+dragged (Ctrl+drag and a right-button drag change nothing).
44: door TT_AD2 to Gamma's (12,12), Current To Target. 45: door TT_BD2 from Gamma's
  (12,6), Target To Current. 46: Gamma's (6,12) to door TT_AD1, Current To Target.
  47: Gamma's (6,30) from door TT_BD1, Target To Current. 48: Gamma's (2,20) to Beta's
  pasted trigger, Both Ways (its save of Beta writes 47's TT_BD1 link).
"""

import copy
import json
import os
import shutil
import subprocess
import sys
import tempfile

ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
WAVE2_BEFORE = os.path.join(ORACLE, "captures", "wave2", "before.mod")
DOORS = (("alpha", (("TT_AD1", 34.0, 4.0), ("TT_AD2", 34.0, 10.0))),
         ("beta", (("TT_BD1", 34.0, 8.0), ("TT_BD2", 34.0, 14.0))))


def gff(path):
    return json.loads(subprocess.run(["mg", "gff", path], check=True, capture_output=True,
                                     text=True).stdout)


def put(obj, path):
    tmp = path + ".json"
    with open(tmp, "w") as f:
        json.dump(obj, f)
    subprocess.run(["mg", "gff", tmp, "-o", path], check=True, capture_output=True)
    os.unlink(tmp)


def inject_doors(mod):
    """Four doors, wave2's W2_D1 struct (its variables and OnOpen cleared), at Z 5."""
    work = tempfile.mkdtemp()
    w2 = tempfile.mkdtemp()
    try:
        subprocess.run(["mg", "unpack", WAVE2_BEFORE, w2], check=True, capture_output=True)
        door0 = gff(os.path.join(w2, "field." + "git"))["Door List"]["value"][0]
        door0["VarTable"]["value"] = []
        door0["OnOpen"]["value"] = ""
        subprocess.run(["mg", "unpack", mod, work], check=True, capture_output=True)
        for area, doors in DOORS:
            gitp = os.path.join(work, area + "." + "git")
            gicp = os.path.join(work, area + ".gic")
            g, c = gff(gitp), gff(gicp)
            for tag, x, y in doors:
                d = copy.deepcopy(door0)
                d["Tag"]["value"] = tag
                d["LocName"]["value"] = {"0": tag}
                d["X"]["value"], d["Y"]["value"], d["Z"]["value"] = x, y, 5.0
                g["Door List"]["value"].append(d)
                c["Door List"]["value"].append(
                    {"__struct_id": 8, "Comment": {"type": "cexostring", "value": ""}})
            put(g, gitp)
            put(c, gicp)
        os.unlink(mod)
        subprocess.run(["mg", "pack", work, mod], check=True, capture_output=True)
    finally:
        shutil.rmtree(work)
        shutil.rmtree(w2)
    print("doors added to", mod)


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "inject-doors":
        inject_doors(sys.argv[2])
    else:
        print(__doc__)
        sys.exit(2)
