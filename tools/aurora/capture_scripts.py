#!/usr/bin/env python3
"""Make the script compile probe for the Aurora oracle, and say how its
captures were made (crates/mg-corpus-tests/tests/aurora_scripts.rs).

    capture_scripts.py

It reads `captures/placement-probe.mod` (an Aurora-saved module with one
area) and writes `captures/script-compile/before.mod` (also as
`script_probe.mod` in the oracle's `modules/`): that module with the
scripts in SCRIPTS below added, the cases where compilers might differ:

- `cut_*`: the base game's `x2_inc_cutscene` included (it declares
  `object CutCreateObjectCopy(…)` and defines it `void`), not called,
  called, and its result used; `rt_*`: the same mismatch in the script's
  own code, both ways, its result used and not, and never called.
- `inc_*`: include files (no `main`): with an error, without one, one
  that leans on what an includer would bring, one with a syntax error;
  `uses_inc_bad` includes the one with an error.
- `cm_*`: comments: a `//` line with `/*` in it, a block never closed,
  "nested" blocks, `#include` in a `//` line and in a block, a `//` line
  ending in a backslash (once before code, once before an error), the
  file ending in a comment without a newline, `**/` and `/*/`, code
  commented out, `main` only in a comment, CRLF line ends.

Then, in Aurora (tools/aurora/headless.sh, run-aurora.sh; xdrive.py):

1. Open `script_probe` (the welcome screen's Open an existing Module).
2. Build > Build Module; Advanced Controls on; under Compile only Scripts
   (Palettes off); Build. The Results list (paged with the arrow keys,
   `build-results.png`) reads "<script>.nss - Compile Error" for:
   cm_backslash2, cm_nested, inc_bad, inc_lean, inc_syntax, uses_inc_bad;
   nothing for the others.
3. Done, File > Save: `build.mod` (the saved module copied from the
   oracle's `modules/`): its compiled scripts are those Aurora made.
"""

import os
import shutil
import subprocess
import tempfile

BIN = os.environ.get("NWN_TOOLS_BIN", os.path.expanduser("~/.local/opt/neverwinter/bin"))
ORACLE = os.environ.get("MOONGLOW_ORACLE", os.path.expanduser("~/.local/share/moonglow-oracle"))
CAPTURES = os.path.join(ORACLE, "captures")
OUT = os.path.join(CAPTURES, "script-compile")
USER = os.path.join(ORACLE, "userdir")

SCRIPTS = {
    "cut_inc": '#include "x2_inc_cutscene"\nvoid main() {}\n',
    "cut_call": '#include "x2_inc_cutscene"\nvoid main()\n{\n    CutCreateObjectCopy(0.0, OBJECT_SELF, GetLocation(OBJECT_SELF));\n}\n',
    "cut_use": '#include "x2_inc_cutscene"\nvoid main()\n{\n    object o = CutCreateObjectCopy(0.0, OBJECT_SELF, GetLocation(OBJECT_SELF));\n    SetLocalObject(OBJECT_SELF, "copy", o);\n}\n',
    "rt_declint": 'int Foo();\nvoid main() { Foo(); }\nvoid Foo() { PrintString("x"); }\n',
    "rt_usedint": 'int Foo();\nvoid main() { int n = Foo(); PrintInteger(n); }\nvoid Foo() { PrintString("x"); }\n',
    "rt_declvoid": 'void Foo();\nvoid main() { Foo(); }\nint Foo() { return 1; }\n',
    "rt_unused": 'int Foo();\nvoid Foo() { PrintString("x"); }\nvoid main() {}\n',
    "inc_bad": '// An include with an error.\nint IncBad() { return Nope(); }\n',
    "inc_good": '// An include without errors.\nint IncGood() { return 1; }\n',
    "inc_lean": '// An include leaning on its includer.\nint IncLean() { return FromIncluder(); }\n',
    "inc_syntax": '// An include with a syntax error.\nint IncSyntax( { return 1; }\n',
    "cm_slashstar": '// a line comment with /* in it\nvoid main() { PrintString("a"); }\n// and */ here\n',
    "cm_unterm": 'void main() { PrintString("a"); }\n/* never closed\nint x = ;\n',
    "cm_nested": '/* outer /* inner */ still outer? */\nvoid main() {}\n',
    "cm_incl_line": '// #include "no_such_inc"\nvoid main() {}\n',
    "cm_incl_block": '/*\n#include "no_such_inc"\n*/\nvoid main() {}\n',
    "cm_incl_block2": '/* #include "inc_bad" */\nvoid main() {}\n',
    "cm_backslash": '// a comment ending in a backslash \\\nvoid main() { PrintString("a"); }\n',
    "cm_backslash2": 'void main()\n{\n    // a comment ending in a backslash \\\n    PrintString(Nope);\n}\n',
    "cm_eof_line": 'void main() {}\n// last line, no newline',
    "cm_eof_block": 'void main() {}\n/* last, no newline */',
    "cm_eof_open": 'void main() {}\n/* open at the end, no newline',
    "cm_starslash": '/* stars **/\n/*/ a slash after the opener */\nvoid main() {}\n',
    "cm_code": 'void main()\n{\n    // PrintString(Nope);\n    /* PrintString(Nope2); */\n    /*\n    int x = Nope3();\n    */\n}\n',
    "cm_main_block": '/*\nvoid main() {}\n*/\nint NotMain() { return 1; }\n',
    "cm_crlf": 'void main()\r\n{\r\n    // comment \\\r\n    PrintString("a");\r\n}\r\n',
    "uses_inc_bad": '#include "inc_bad"\nvoid main() {}\n',
}


def main():
    os.makedirs(OUT, exist_ok=True)
    base = tempfile.mkdtemp()
    subprocess.run([f"{BIN}/nwn_erf", "-x", "-f", os.path.join(CAPTURES, "placement-probe.mod")],
                   cwd=base, check=True, capture_output=True)
    for name, text in SCRIPTS.items():
        assert len(name) <= 16, name
        with open(os.path.join(base, name + ".nss"), "w", newline="") as f:
            f.write(text)
    out = os.path.join(OUT, "before.mod")
    if os.path.exists(out):
        os.unlink(out)
    files = sorted(os.listdir(base))
    subprocess.run([f"{BIN}/nwn_erf", "-e", "MOD", "-c", "-f", out] + files, cwd=base, check=True,
                   capture_output=True)
    shutil.rmtree(base)
    os.makedirs(os.path.join(USER, "modules"), exist_ok=True)
    shutil.copy(out, os.path.join(USER, "modules", "script_probe.mod"))
    print(f"made {out}; now capture with Aurora as this script's doc says")


if __name__ == "__main__":
    main()
