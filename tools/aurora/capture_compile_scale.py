#!/usr/bin/env python3
"""Make the compile-at-scale probes for the Aurora oracle, and say how
their captures were made (crates/mg-corpus-tests/tests/aurora_scripts.rs).

    capture_compile_scale.py

A builder's report: a module compiled in Aurora (Build, no errors) and
saved, then Compile All in Moonglow, said "41 changed": 41 of 1864
compiled scripts were not the bytes Aurora had saved.

1. `captures/script-compile-scale/before.mod` (also `compile_scale.mod`
   in the oracle's `modules/`): `captures/placement-probe.mod` (an
   Aurora-saved module with one area) with every script source of the
   base game (`*.nss` in its keys, but `nwscript`) as module-local
   copies, as the game has them (CRLF line ends, a few LF), and every
   third one in name order turned to LF line ends, so that a difference
   that follows the line ends would show.
2. `hak-order.mod` (also `hak_order.mod`): the same area with
   HAK_SCRIPTS, and `hak_order.hak` (copied into the oracle's `hak/` and
   attached to it) with HAK_SCRIPTS_IN_HAK: an include and a script that
   are both in the module and in the hak, each copy saying where it is,
   so the compiled scripts show which copy a compiler read (the game's
   resource manager puts a hak above the module).

Then, in Aurora (tools/aurora/headless.sh, run-aurora.sh; xdrive.py), as
capture_scripts.py says, for each: open it, Build > Build Module,
Advanced Controls, Compile only Scripts (Palettes off), Build, Done,
then File > Save; copy the saved module from the oracle's `modules/` to
`build.mod` and `hak-order-build.mod` beside the others.
"""

import os
import shutil
import subprocess
import sys
import tempfile

BIN = os.environ.get("NWN_TOOLS_BIN", os.path.expanduser("~/.local/opt/neverwinter/bin"))
ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
NWN_ROOT = os.environ.get(
    "NWN_ROOT", os.path.expanduser("~/.local/share/Steam/steamapps/common/Neverwinter Nights"))
CAPTURES = os.path.join(ORACLE, "captures")
OUT = os.path.join(CAPTURES, "script-compile-scale")
USER = os.path.join(ORACLE, "userdir")
PROBE = os.path.join(CAPTURES, "placement-probe.mod")
# Every STEP-th script of the base game (`capture_compile_scale.py STEP`).
STEP = int(sys.argv[1]) if len(sys.argv) > 1 else 4

HAK_SCRIPTS = {
    "ho_inc": 'string HoWhere() { return "module"; }\r\n',
    "ho_self": 'void main() { PrintString("module"); }\r\n',
    "ho_use": '#include "ho_inc"\r\nvoid main() { PrintString(HoWhere()); }\r\n',
}
HAK_SCRIPTS_IN_HAK = {
    "ho_inc": 'string HoWhere() { return "hak"; }\r\n',
    "ho_self": 'void main() { PrintString("hak"); }\r\n',
}


# Constructs a compiler of another date may make other code of
# (`codegen.mod`): constant expressions where a value is wanted at run
# time (folded always since neverwinter.nim db755db7, 2025-08; before it
# only with the FoldConstants flag), unary minus, strings, floats,
# comparisons, the conditional operator, switch, structs, vectors, and a
# StartingConditional script whose include has a main (d20453e3, 2026-08),
# and scripts that are only an include of a file with a main or a
# StartingConditional.
CODEGEN_SCRIPTS = {
    "cg_int": 'void main() { int n = 2 * 3 + 1; PrintInteger(n); }\r\n',
    "cg_arg": 'void main() { PrintInteger(60 * 6); }\r\n',
    "cg_const": 'const int A = 2;\r\nvoid main() { PrintInteger(A * 3); }\r\n',
    "cg_float": 'void main() { PrintFloat(6.0 * 2.0 + 0.5); }\r\n',
    "cg_str": 'void main() { PrintString("a" + "b"); }\r\n',
    "cg_neg": 'void main() { int n = -5; PrintInteger(n); PrintInteger(-n); }\r\n',
    "cg_cmp": 'void main() { if (1 == 1) PrintString("y"); if (2 > 3) PrintString("n"); }\r\n',
    "cg_tern": 'void main() { PrintInteger(TRUE ? 1 : 2); }\r\n',
    "cg_shift": 'void main() { PrintInteger((1 << 4) | 3); PrintInteger(~0 & 255); }\r\n',
    "cg_mixed": 'void main() { float f = 2 * 1.5; PrintFloat(f); }\r\n',
    "cg_consts": 'void main() { PrintInteger(OBJECT_TYPE_CREATURE | OBJECT_TYPE_PLACEABLE); }\r\n',
    "cg_switch": 'void main()\r\n{\r\n    switch (Random(3))\r\n    {\r\n        case 0: PrintString("0"); break;\r\n        case 1 + 1: PrintString("2");\r\n        default: PrintString("d");\r\n    }\r\n}\r\n',
    "cg_struct": 'struct S { int a; float b; string c; };\r\nstruct S Make() { struct S s; s.a = 1; s.b = 2.0; s.c = "x"; return s; }\r\nvoid main() { struct S s = Make(); PrintInteger(s.a); }\r\n',
    "cg_vector": 'void main() { vector v = Vector(1.0, 2.0, 3.0); v = v * 2.0; PrintVector(v, FALSE); }\r\n',
    "cg_loop": 'void main() { int i; for (i = 0; i < 3; i++) PrintInteger(i); while (i > 0) i--; }\r\n',
    "cg_incmain": '// An include with a main.\r\nvoid main() { PrintString("include main"); }\r\n',
    "cg_cond": '#include "cg_incmain"\r\nint StartingConditional() { return 1; }\r\n',
    "cg_inconly": '#include "cg_incmain"\r\n',
    "cg_incsc": '// An include with a StartingConditional.\r\nint StartingConditional() { return 1; }\r\n',
    "cg_usesc": '#include "cg_incsc"\r\n',
    "cg_delay": 'void main() { DelayCommand(1.0 + 2.0, PrintString("later")); }\r\n',
}


def pack(kind, out, d):
    if os.path.exists(out):
        os.unlink(out)
    subprocess.run([f"{BIN}/nwn_erf", "-e", kind, "-c", "-f", out] + sorted(os.listdir(d)),
                   cwd=d, check=True, capture_output=True)


def erf(kind, out, scripts, base=None):
    """An ERF of `kind` at `out` with `scripts` (over `base`'s files)."""
    d = tempfile.mkdtemp()
    try:
        if base:
            subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", base], cwd=d, check=True,
                           capture_output=True)
        for name, text in scripts.items():
            with open(os.path.join(d, name + ".nss"), "wb") as f:
                f.write(text if isinstance(text, bytes) else text.encode("latin-1"))
        pack(kind, out, d)
    finally:
        shutil.rmtree(d)


def at_scale():
    scratch = tempfile.mkdtemp()
    try:
        nss = os.path.join(scratch, "nss")
        os.makedirs(os.path.join(scratch, "ud"))
        os.makedirs(nss)
        subprocess.run([f"{BIN}/nwn_resman_extract", "--root", NWN_ROOT, "--no-ovr",
                        "--userdirectory", os.path.join(scratch, "ud"), "-p", ".nss", "-d", nss],
                       check=True, capture_output=True)
        names = sorted(n for n in os.listdir(nss) if n.lower().endswith(".nss"))
        names = [n for n in names if n.lower() != "nwscript.nss"][::STEP]
        scripts, lf = {}, 0
        for i, name in enumerate(names):
            assert len(name) - 4 <= 16, name
            with open(os.path.join(nss, name), "rb") as f:
                text = f.read()
            if i % 3 == 0:
                text = text.replace(b"\r\n", b"\n")
                lf += 1
            scripts[name.lower()[:-4]] = text
    finally:
        shutil.rmtree(scratch)
    out = os.path.join(OUT, "before.mod")
    erf("MOD", out, scripts, PROBE)
    shutil.copy(out, os.path.join(USER, "modules", "compile_scale.mod"))
    print(f"made {out}: {len(scripts)} scripts, {lf} turned to LF")


def hak_order():
    mod = os.path.join(OUT, "hak-order.mod")
    hak = os.path.join(OUT, "hak_order.hak")
    erf("MOD", mod, HAK_SCRIPTS, PROBE)
    erf("HAK", hak, HAK_SCRIPTS_IN_HAK)
    subprocess.run(["mg", "attach", "--replace", "--user-dir", USER, mod, hak], check=True,
                   capture_output=True)
    shutil.copy(mod, os.path.join(USER, "modules", "hak_order.mod"))
    print(f"made {mod} with {hak} attached")


def codegen():
    mod = os.path.join(OUT, "codegen.mod")
    erf("MOD", mod, CODEGEN_SCRIPTS, PROBE)
    shutil.copy(mod, os.path.join(USER, "modules", "cg_probe.mod"))
    print(f"made {mod}")


def main():
    os.makedirs(OUT, exist_ok=True)
    os.makedirs(os.path.join(USER, "modules"), exist_ok=True)
    at_scale()
    hak_order()
    codegen()
    print("now capture with Aurora as this script's doc says")


if __name__ == "__main__":
    main()
