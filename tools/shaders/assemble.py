#!/usr/bin/env python3
"""Assemble a stock EE shader the way the engine does (preamble + verbatim
#include splicing), for validation with glslangValidator. Run in a directory
holding the .shd files (e.g. extracted with `mg cat NAME.shd`):

    assemble.py fslit frag > fslit.frag && glslangValidator fslit.frag
"""
import re, sys
def inline(name, depth):
    out=[]
    for line in open(f"{name}.shd", encoding="latin-1").read().splitlines():
        m=re.match(r'\s*#include\s+"([^"]+)"', line)
        if m and depth < 16:
            out.append(inline(m.group(1), depth+1))
        elif not m:
            out.append(line)
    return "\n".join(out)
name, stage = sys.argv[1], sys.argv[2]
pre = """#version 330 core
#define mediump
#define lowp
#define highp
#define MAX_NUM_LIGHTS 32
#define MAX_NUM_BONES 64
#define GAMMA_CORRECTION 1
#define FRAGMENT_LIGHTING 1
#define SHADER_QUALITY_MODE 2
#define KEYHOLING_ENABLED 0
#define SHADER_DEBUG_MODE 0
#define BUILD_VERSION 8193
#define BUILD_REVISION 37
#define NO_DISCARD 1
#define POSTPROCESSING_TYPES_ENABLED 0
#define attribute in
#define texture2D texture
#define textureCube texture
"""
if stage == "frag":
    pre += "#define varying in\n#define gl_FragColor compat_glFragColor\nout vec4 compat_glFragColor;\n"
else:
    pre += "#define varying out\n"
print(pre + inline(name, 0))
