#!/usr/bin/env python3
"""Shader spike (docs/shaders-proposal.md): assemble every program shader (engine preamble + #include
splicing, search path in order) and compile it with glslang to Vulkan SPIR-V
under relaxed rules. Usage: spike.py OUTDIR SEARCHDIR... (first wins)

The rest of the route: `spirv-opt --split-combined-image-sampler` on each
.spv, then `nagacheck/` (naga 30's spv-in, every resource binding renumbered
in the IR, validation and WGSL out).
Extract the stock shaders with an empty user directory (nwn_resman_extract
-p .shd --userdirectory EMPTY) so that the install's own copies are read."""
import os, re, subprocess, sys

out, dirs = sys.argv[1], sys.argv[2:]
os.makedirs(out, exist_ok=True)

def find(name):
    for d in dirs:
        for f in os.listdir(d):
            if f.lower() == name.lower() + ".shd":
                return os.path.join(d, f)
    return None

def inline(name, depth, missing):
    p = find(name)
    if p is None:
        missing.add(name)
        return ""
    lines = []
    for line in open(p, encoding="latin-1").read().replace("\r", "").splitlines():
        m = re.match(r'\s*#include\s+"([^"]+)"', line)
        if m and depth < 16:
            lines.append(inline(m.group(1), depth + 1, missing))
        elif not m:
            lines.append(line)
    return "\n".join(lines)

PRE = """#version 330 core
#define mediump
#define lowp
#define highp
#define MAX_NUM_LIGHTS 32
#define MAX_NUM_BONES 64
#define GAMMA_CORRECTION 1
#define FRAGMENT_LIGHTING 1
#define SHADER_QUALITY_MODE 2
#define KEYHOLING_ENABLED 1
#define SHADER_DEBUG_MODE 0
#define BUILD_VERSION 8193
#define BUILD_REVISION 37
#define NO_DISCARD 0
#define POSTPROCESSING_TYPES_ENABLED 0
#define attribute in
#define texture2D texture
#define textureCube texture
"""

def stage(name):
    n = name.lower()
    if n.startswith("inc_"):
        return None
    if n.endswith("_vs") or (n.startswith("vs") and not n.endswith("_fs")):
        return "vert"
    if n.endswith("_fs") or n.startswith("fs"):
        return "frag"
    return None

names = sorted({f[:-4] for d in dirs for f in os.listdir(d) if f.lower().endswith(".shd")}, key=str.lower)
ok = fail = 0
for name in names:
    st = stage(name)
    if st is None:
        continue
    missing = set()
    body = inline(name, 0, missing)
    if "main" not in body:
        continue
    pre = PRE + ("#define varying in\n#define gl_FragColor compat_glFragColor\nlayout(location=0) out vec4 compat_glFragColor;\n"
                 if st == "frag" else "#define varying out\n")
    src = os.path.join(out, f"{name}.{st}")
    open(src, "w").write(pre + body)
    spv = src + ".spv"
    r = subprocess.run(["glslangValidator", "-V", "-R", "--amb", "--aml", "-S", st, "-o", spv, src],
                       capture_output=True, text=True)
    good = r.returncode == 0
    ok += good; fail += not good
    err = "" if good else " | " + " ; ".join(l for l in r.stdout.splitlines() if "ERROR" in l)[:300]
    print(f"{'OK  ' if good else 'FAIL'} {name} {st} {'missing:'+','.join(missing) if missing else ''}{err}")
print(f"\n{ok} compiled, {fail} failed")
