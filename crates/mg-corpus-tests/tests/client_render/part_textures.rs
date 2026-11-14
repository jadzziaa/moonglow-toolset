//! Which texture the game client draws on a creature's body part whose
//! mesh names another one (a builder's report on 4.0: "if a body part
//! model has an attached texture in the model, then Moonglow does render
//! the texture instead of the given PLT of the body part as the game
//! does"), and which of a PLT and a DDS or TGA of one name it draws.
//!
//! Authored parts in the scratch user directory's `override` (some rows'
//! textures in a hak, `mg_parts`, the module has): each a box for a chest
//! (`p<g><r><pheno>_chest2NN`) and a smaller one for a head
//! (`..._head2NN`), one mesh or two (the two halves side by side), each
//! mesh naming the texture its row says. The textures are flat colors:
//! TGAs red (`mg_red`), green (`mg_green`), blue (another part's name,
//! `pmh0_chest299`), yellow (a TGA or DDS of the part's own name beside
//! its PLT) and magenta (the part's own name as a TGA alone), a gray TGA
//! of 8 bits as the game's own beside their PLTs; PLTs each of one layer
//! whose palette the override makes one flat color: Cloth 1 orange (the
//! part's own PLT, `pmh0_chest2NN.plt`), Leather 1 purple (`mg_pltx`, no
//! part's name) and Hair cyan (another part's, `pmh0_chest298`). Besides
//! them the game's own dwarf (head 1) and human (chest, head and hands 3),
//! a chest worn as an armor's torso and a helmet (`helm_250`). The heads
//! are the boxes, so the hair palette colors nothing else; the cloth
//! palette colors the bodies' loincloths too.
//!
//! The rows ([`ROWS`]), shot in groups ([`GROUPS`], `MG_GROUPS=abcde,no`
//! for some), west to east, north of the player and facing it, from near
//! (`SetCameraFacing(90, 5, 75)`), beside Moonglow's area view of the same
//! creatures. The client's pictures and Moonglow's go to
//! `target/test-output/part_textures/` (`<group>_client.png`,
//! `<group>_<side>.png` with `MG_SIDE` naming Moonglow's, `moonglow` by
//! default); `MG_MOONGLOW_ONLY=1` draws Moonglow's side alone beside a
//! client picture kept there.
//!
//! Seen (2026-11-10, EE 89.8193.37): a PLT of the part's own name, or of
//! the first of its fallbacks that has one (phenotype 0, human of its
//! gender, human man; a head's stop at its race's phenotype 0), is what
//! every mesh of the part is drawn with, whatever each names (a, b, c, f,
//! g, i, j, k, l, m, D, y; F a mesh naming no texture; V one naming a
//! material without a `texture0`), robes as any part (M, N); a DDS or TGA
//! of the part's name alone replaces nothing (h, C), nor does a human's
//! PLT on an elf's or a woman's head (j, l, y, z: a head's mesh naming its
//! own missing texture is drawn untextured, light gray). A mesh whose
//! `bitmap` or `materialname` names a material with a `texture0` keeps the
//! material (G, H, I, L, K); the name a mesh is given in place of its own
//! is drawn without its material (J: an MTR of the part's own name, its
//! texture0 pink, is not drawn on a mesh that names something else, the
//! PLT is). The name so drawn is a DDS (a DXT1 of gray texels too, P) or
//! true-color TGA of it where there is one, wherever in the load order
//! (e, r, s, t, u, v, x, q, and the helmet, E), and the PLT over a gray
//! TGA (n, o, A, B) or a DDS of one luminance channel (Q). Such a DDS
//! alone the client draws in colors of no rule (R, S, T, U: 60 blue, 128
//! dark red, 0 and 255 black), where Moonglow draws it gray.
//!
//! Helmets, seen (2026-11-13, GitHub issue 45: the game's hood
//! `helm_033`, whose mesh names `helm`, a texture that is nowhere, drawn
//! untextured and shiny): a helmet is drawn as a part, its own PLT on a
//! mesh naming a texture that is nowhere (W), another helmet's PLT (X),
//! a TGA of no part's name (Y) or none (0); a mesh naming a material
//! with a `texture0` keeps it (1); a TGA of the helmet's name alone
//! replaces nothing (2), and without a PLT of its name the mesh is drawn
//! untextured, gray (Z). Of two meshes, one naming such a material keeps
//! it and the other takes the PLT ($); a TGA beside the PLT of the
//! helmet's name is drawn on a mesh naming another name (&), as on one
//! naming its own (E). The game's hood (3) is drawn textured in its PLT's
//! metal layers; `helm_006` (6) looks the same at this distance in its
//! own PLT and in `helm_003`'s, which its mesh names.
//!
//! Normal maps (2026-11-13; groups `4578` for helmets and `%+#=` for
//! parts, from above in a darker light, [`CLOSE`]): each mesh a box whose
//! faces each hold the whole texture, the materials naming normal maps of
//! stripes (`mg_stripe2`, `mg_stripe6`: the normal straight out and
//! tipped nearly flat by turns; the client reads x and y alone, so a
//! normal into the surface showed nothing), so the sunlit tops show which
//! one is drawn. A mesh naming its own name with an MTR of it shows 6
//! stripes (4, %); one naming another name whose MTR names a normal map
//! shows that one's 2 stripes, whether or not the part's or helmet's own
//! name has an MTR (5, 8, +, =); one naming a TGA with no MTR, beside an
//! MTR of the own name, shows none (7, #). The material (its normal and
//! specular maps) is the name the mesh gives, not the one it is drawn in;
//! Moonglow draws the same.
//!
//! A mesh whose texture is nowhere (2026-11-13, group `Z9@^~`): the
//! client draws it as a white TGA of alpha 0 (@), the object's
//! environment map reflected over the whole of it, a helmet's (Z) and a
//! part's (~) alike; a white TGA of 24 bits is drawn white (9, ^).
//! Moonglow draws it so too, but its reflection of `chrome1` on such a
//! surface is darker than the client's, black on the tops (Z, @, ~).
//!
//! [`part_textures_tdod_look`] reads the owner's own module's parts the
//! rule changes, beside Moonglow's area view as before the rule
//! (`MG_SIDE=before` with HEAD's rule) and after it.
//!
//! The details are in `docs/findings.md`, "A body part is drawn in its
//! own PLT".
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render part_textures -- --ignored --nocapture`.

use super::readings::{INVISIBLE, Reading, enter, print_log};
use super::*;

/// A texture authored for the rows.
#[derive(Clone, Copy)]
enum Tex {
    /// A TGA of one color (RGB).
    Tga([u8; 3]),
    /// A TGA of one gray (8 bits, uncompressed: as the game's own gray
    /// TGAs beside its PLTs, `pmd0_head001`, `pmh0_handl003`).
    Gray(u8),
    /// [`Tex::Gray`] whose header says no alpha bits (the game's say 8).
    GrayNoAlpha(u8),
    /// A standard DDS (DXT1) of one color (RGB).
    Dds([u8; 3]),
    /// A standard DDS (DXT1) of one gray.
    DdsGray(u8),
    /// A standard uncompressed DDS of one channel (luminance, 8 bits) of
    /// one gray.
    DdsLuminance(u8),
    /// An MTR, its text.
    Mtr(&'static str),
    /// A PLT all of one layer.
    Plt(u8),
    /// A normal map (64×64 TGA) of horizontal stripes, the normal straight
    /// out of the surface and tipped nearly flat by turns (the client
    /// reads x and y alone: a normal into the surface, z below 0, drew
    /// as one straight out): `bands` of them.
    Stripes(u8),
    /// A 32-bit TGA of one color and one alpha.
    Tga32([u8; 3], u8),
}

/// A texture's file: its extension and contents.
fn file_of(t: Tex) -> (&'static str, Vec<u8>) {
    match t {
        Tex::Tga(c) => ("tga", flat_tga(8, 8, c)),
        Tex::Gray(v) => {
            let mut t = vec![0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 8, 8];
            t.extend([v; 64]);
            ("tga", t)
        }
        Tex::GrayNoAlpha(v) => {
            let mut t = vec![0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 8, 0];
            t.extend([v; 64]);
            ("tga", t)
        }
        Tex::Dds([r, g, b]) => {
            let c = (u16::from(r) >> 3) << 11 | (u16::from(g) >> 2) << 5 | u16::from(b) >> 3;
            let block = move |_, _| [c.to_le_bytes(), c.to_le_bytes(), [0, 0], [0, 0]].concat();
            ("dds", super::programs::dds(b"DXT1", block))
        }
        Tex::DdsGray(v) => file_of(Tex::Dds([v, v, v])),
        Tex::DdsLuminance(v) => {
            let mut h = b"DDS ".to_vec();
            let mut u = |x: u32| h.extend(x.to_le_bytes());
            // (Size; flags: caps, height, width, pixel format, pitch; the
            // pitch; no depth or levels.)
            u(124);
            u(0x1 | 0x2 | 0x4 | 0x8 | 0x1000);
            u(8);
            u(8);
            u(8);
            u(0);
            u(0);
            for _ in 0..11 {
                u(0);
            }
            // The pixel format: luminance, 8 bits, the mask its byte.
            u(32);
            u(0x2_0000);
            u(0);
            u(8);
            u(0xFF);
            u(0);
            u(0);
            u(0);
            u(0x1000);
            for _ in 0..4 {
                u(0);
            }
            h.extend([v; 64]);
            ("dds", h)
        }
        Tex::Mtr(text) => ("mtr", text.as_bytes().to_vec()),
        Tex::Plt(layer) => ("plt", flat_plt(layer)),
        Tex::Stripes(bands) => {
            let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 0, 64, 0, 24, 0];
            let band = 64 / u32::from(bands);
            for y in 0..64u32 {
                let out = (y / band) % 2 == 0;
                for _ in 0..64 {
                    // (B, G, R: the normal's z, y and x.)
                    t.extend([255, 128, if out { 128 } else { 252 }]);
                }
            }
            ("tga", t)
        }
        Tex::Tga32([r, g, b], a) => {
            let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 8, 0, 32, 8];
            for _ in 0..64 {
                t.extend([b, g, r, a]);
            }
            ("tga", t)
        }
    }
}

const RED: [u8; 3] = [230, 20, 20];
const GREEN: [u8; 3] = [20, 210, 20];
const BLUE: [u8; 3] = [30, 60, 255];
const YELLOW: [u8; 3] = [240, 230, 20];
const MAGENTA: [u8; 3] = [230, 20, 230];
/// The MTRs' `texture0` (`mg_pink`).
const PINK: [u8; 3] = [255, 150, 200];
/// The flat palettes: Cloth 1's orange (the parts' own PLTs), Leather 1's
/// purple (`mg_pltx`), Hair's cyan (another part's PLT).
const PALETTES: [(&str, [u8; 3]); 3] =
    [("pal_cloth01", [255, 120, 0]), ("pal_leath01", [120, 0, 230]), ("pal_hair01", [0, 220, 220])];
const OWN_PLT: Tex = Tex::Plt(4);
/// A material of the name a mesh names (`mg_nma`): a normal map of 2
/// stripes, no texture0.
const NAMED_MTR: &str = "renderhint NormalAndSpecMapped\ntexture1 mg_stripe2\n";
/// A material of the part's or helmet's own name: a normal map of 6
/// stripes, no texture0.
const OWN_MTR: Tex = Tex::Mtr("renderhint NormalAndSpecMapped\ntexture1 mg_stripe6\n");

/// A row: what a creature's chest and head are, the creature's skeleton
/// (`pmh0`, an elf's `pme0`, phenotype 2's `pmh2`), the parts' number,
/// the textures each part's meshes name (`own`: the part's own name;
/// `other`: another part's, of the same kind, numbered 299 for its TGA
/// and 298 for its PLT) and the files of the part's name that exist
/// (as `pmh0_` of the part's name where `human` is set: found only by
/// the fallback).
struct Row {
    label: &'static str,
    skeleton: &'static str,
    n: u16,
    meshes: &'static [&'static str],
    own: &'static [Tex],
    human: bool,
    armored: bool,
    /// Other files for the override, by name.
    extra: &'static [(&'static str, Tex)],
    /// Textures of the part's name in a hak (`mg_parts`) the module has.
    hak: &'static [Tex],
}

const ROWS: [Row; 70] = [
    Row {
        label: "a: one mesh naming a TGA of no part's name, own PLT (the report)",
        skeleton: "pmh0",
        n: 201,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "b: one mesh naming a PLT of no part's name, own PLT",
        skeleton: "pmh0",
        n: 202,
        meshes: &["mg_pltx"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "c: two meshes, the part's own name and a TGA of no part's (hair, a gem), own PLT",
        skeleton: "pmh0",
        n: 203,
        meshes: &["own", "mg_green"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "d: one mesh naming a TGA of no part's name, nothing of the part's name",
        skeleton: "pmh0",
        n: 204,
        meshes: &["mg_red"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "e: one mesh naming its own name, a TGA and a PLT of it",
        skeleton: "pmh0",
        n: 206,
        meshes: &["own"],
        own: &[Tex::Tga(YELLOW), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "f: one mesh naming another part's TGA, own PLT",
        skeleton: "pmh0",
        n: 207,
        meshes: &["other299"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "g: one mesh naming another part's PLT, own PLT (the known case)",
        skeleton: "pmh0",
        n: 208,
        meshes: &["other298"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "h: one mesh naming a TGA of no part's name, own a TGA alone",
        skeleton: "pmh0",
        n: 210,
        meshes: &["mg_red"],
        own: &[Tex::Tga(MAGENTA)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "i: two meshes naming two TGAs of no part's name, own PLT",
        skeleton: "pmh0",
        n: 211,
        meshes: &["mg_red", "mg_green"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "j: an elf's part naming a TGA of no part's, the PLT the human's alone",
        skeleton: "pme0",
        n: 205,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "k: phenotype 2's part naming a TGA of no part's, the PLT phenotype 0's alone",
        skeleton: "pmh2",
        n: 205,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "l: an elf's two meshes, its own name and a TGA of no part's, the human's PLT",
        skeleton: "pme0",
        n: 209,
        meshes: &["own", "mg_green"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "m: row a's chest worn as an armor's torso (its head bare)",
        skeleton: "pmh0",
        n: 201,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: true,
        extra: &[],
        hak: &[],
    },
    // (Stock parts, no meshes authored: a TGA and a PLT of one name in one
    // layer of the game's, set against row e's two in the override. The
    // creatures' skin is color 28.)
    Row {
        label: "n: the game's dwarf, head 1 (pmd0_head001: a gray TGA and a PLT)",
        skeleton: "pmd0",
        n: 1,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "o: the game's human, chest, head and hands 3 (pmh0_handl003: a TGA and a PLT)",
        skeleton: "pmh0",
        n: 3,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    // (The same names, a texture of each in the override over the
    // game's.)
    Row {
        label: "p: the game's dwarf, head 1, a PLT of its name in the override (Cloth 1)",
        skeleton: "pmd0",
        n: 1,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[("pmd0_head001", OWN_PLT)],
        hak: &[],
    },
    Row {
        label: "q: the game's human, chest 3 (naming chest 4), a yellow TGA of its name in the override",
        skeleton: "pmh0",
        n: 3,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[("pmh0_chest003", Tex::Tga(YELLOW))],
        hak: &[],
    },
    Row {
        label: "r: one mesh naming a TGA of no part's name, own a TGA and a PLT",
        skeleton: "pmh0",
        n: 212,
        meshes: &["mg_red"],
        own: &[Tex::Tga(YELLOW), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    // (In a hak: the module's `mg_parts`.)
    Row {
        label: "s: one mesh naming its own name, a TGA and a PLT of it in a hak",
        skeleton: "pmh0",
        n: 213,
        meshes: &["own"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[Tex::Tga(YELLOW), OWN_PLT],
    },
    Row {
        label: "t: one mesh naming a TGA of no part's name, own a TGA and a PLT in a hak",
        skeleton: "pmh0",
        n: 214,
        meshes: &["mg_red"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[Tex::Tga(YELLOW), OWN_PLT],
    },
    Row {
        label: "u: one mesh naming a TGA of no part's name, own PLT in the override, TGA in a hak",
        skeleton: "pmh0",
        n: 215,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[Tex::Tga(YELLOW)],
    },
    Row {
        label: "v: one mesh naming a TGA of no part's name, own TGA in the override, PLT in a hak",
        skeleton: "pmh0",
        n: 216,
        meshes: &["mg_red"],
        own: &[Tex::Tga(YELLOW)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[OWN_PLT],
    },
    Row {
        label: "w: one mesh naming a TGA of no part's name, own PLT in a hak alone",
        skeleton: "pmh0",
        n: 217,
        meshes: &["mg_red"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[OWN_PLT],
    },
    Row {
        label: "x: one mesh naming a TGA of no part's name, own a DDS (yellow) and a PLT",
        skeleton: "pmh0",
        n: 218,
        meshes: &["mg_red"],
        own: &[Tex::Dds(YELLOW), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "y: a woman's part naming a TGA of no part's, the PLT the man's alone",
        skeleton: "pfh0",
        n: 219,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "z: a woman's part naming its own name, the PLT the man's alone",
        skeleton: "pfh0",
        n: 220,
        meshes: &["own"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "A: one mesh naming a TGA of no part's name, own a gray TGA (as the game's) and a PLT",
        skeleton: "pmh0",
        n: 221,
        meshes: &["mg_red"],
        own: &[Tex::Gray(128), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "B: row A, the gray TGA's header saying no alpha bits",
        skeleton: "pmh0",
        n: 222,
        meshes: &["mg_red"],
        own: &[Tex::GrayNoAlpha(128), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "C: one mesh naming another part's TGA, own a TGA alone",
        skeleton: "pmh0",
        n: 223,
        meshes: &["other299"],
        own: &[Tex::Tga(MAGENTA)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "D: an elf's part naming a TGA of no part's, own a TGA alone, the human's a PLT",
        skeleton: "pme0",
        n: 224,
        meshes: &["mg_red"],
        own: &[Tex::Tga(MAGENTA)],
        human: false,
        armored: false,
        extra: &[("pmh0_chest224", OWN_PLT), ("pmh0_head224", OWN_PLT)],
        hak: &[],
    },
    // (An item: a helmet, `helm_250`, worn by the game's human.)
    Row {
        label: "E: a helmet whose mesh names its own name, a TGA and a PLT of it",
        skeleton: "helm",
        n: 250,
        meshes: &["own"],
        own: &[Tex::Tga(YELLOW), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "F: one mesh naming no texture (bitmap NULL), own PLT (CEP's heads)",
        skeleton: "pmh0",
        n: 225,
        meshes: &["NULL"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    // (Materials: `mg_mtrb` names `mg_pink` its texture0; `mg_mtrn` the
    // same normal-mapped, `mg_nrm` its normal map. A mesh's `a|b` is a
    // `bitmap a` and a `materialname b`.)
    Row {
        label: "G: one mesh's bitmap naming an MTR (texture0 pink), own PLT",
        skeleton: "pmh0",
        n: 226,
        meshes: &["mg_mtrb"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[("mg_mtrb", Tex::Mtr("texture0 mg_pink\n"))],
        hak: &[],
    },
    Row {
        label: "H: one mesh's bitmap naming a normal-mapped MTR, own PLT",
        skeleton: "pmh0",
        n: 227,
        meshes: &["mg_mtrn"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[(
            "mg_mtrn",
            Tex::Mtr("renderhint NormalAndSpecMapped\ntexture0 mg_pink\ntexture1 mg_nrm\n"),
        )],
        hak: &[],
    },
    Row {
        label: "I: one mesh's bitmap a TGA of no part's name, its materialname an MTR, own PLT",
        skeleton: "pmh0",
        n: 228,
        meshes: &["mg_red|mg_mtrb"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[("mg_mtrb", Tex::Mtr("texture0 mg_pink\n"))],
        hak: &[],
    },
    Row {
        label: "J: one mesh naming a TGA of no part's name, own PLT and an MTR of the part's name",
        skeleton: "pmh0",
        n: 229,
        meshes: &["mg_red"],
        own: &[OWN_PLT, Tex::Mtr("texture0 mg_pink\n")],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "K: one mesh naming its own name, own PLT and an MTR of the part's name",
        skeleton: "pmh0",
        n: 230,
        meshes: &["own"],
        own: &[OWN_PLT, Tex::Mtr("texture0 mg_pink\n")],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "L: one mesh's bitmap naming an MTR, nothing of the part's name",
        skeleton: "pmh0",
        n: 231,
        meshes: &["mg_mtrb"],
        own: &[],
        human: false,
        armored: false,
        extra: &[("mg_mtrb", Tex::Mtr("texture0 mg_pink\n"))],
        hak: &[],
    },
    // (Robes, worn: `p<g><r><pheno>_robeNNN`, a row of `parts_robe.2da`
    // added that hides nothing.)
    Row {
        label: "M: a robe naming a TGA of no part's name, own PLT",
        skeleton: "pmh0robe",
        n: 232,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "N: an elf's robe naming a TGA of no part's name, the human's PLT alone",
        skeleton: "pme0robe",
        n: 233,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: true,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "P: one mesh naming a TGA of no part's name, own a gray DXT1 DDS (dark) and a PLT",
        skeleton: "pmh0",
        n: 234,
        meshes: &["mg_red"],
        own: &[Tex::DdsGray(60), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "Q: one mesh naming a TGA of no part's name, own a one-channel DDS (dark) and a PLT",
        skeleton: "pmh0",
        n: 235,
        meshes: &["mg_red"],
        own: &[Tex::DdsLuminance(60), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "R: one mesh naming its own name, own a one-channel DDS (dark) alone",
        skeleton: "pmh0",
        n: 236,
        meshes: &["own"],
        own: &[Tex::DdsLuminance(60)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "S: one mesh naming its own name, own a one-channel DDS (black) alone",
        skeleton: "pmh0",
        n: 237,
        meshes: &["own"],
        own: &[Tex::DdsLuminance(0)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "T: one mesh naming its own name, own a one-channel DDS (mid gray) alone",
        skeleton: "pmh0",
        n: 238,
        meshes: &["own"],
        own: &[Tex::DdsLuminance(128)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "U: one mesh naming its own name, own a one-channel DDS (white) alone",
        skeleton: "pmh0",
        n: 239,
        meshes: &["own"],
        own: &[Tex::DdsLuminance(255)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "V: one mesh's bitmap naming an MTR without texture0 (a normal map), own PLT",
        skeleton: "pmh0",
        n: 240,
        meshes: &["mg_mtrm"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[("mg_mtrm", Tex::Mtr("renderhint NormalAndSpecMapped\ntexture1 mg_nrm\n"))],
        hak: &[],
    },
    // (Helmets, `helm_<n>` worn by the game's human, as row E's: GitHub
    // issue 45, the game's hood `helm_033`, whose mesh names `helm`, a
    // texture that is nowhere, and `helm_006`, whose mesh names
    // `helm_003`, the PLT of another helmet. Rows 3 and 6 are the game's
    // own helmets, as the game has them.)
    Row {
        label: "W: a helmet whose mesh names a texture that is nowhere, own PLT (the hood)",
        skeleton: "helm",
        n: 251,
        meshes: &["mg_none"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "X: a helmet whose mesh names another helmet's PLT, own PLT",
        skeleton: "helm",
        n: 252,
        meshes: &["otherhelm"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "Y: a helmet whose mesh names a TGA of no part's name, own PLT",
        skeleton: "helm",
        n: 253,
        meshes: &["mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "Z: a helmet whose mesh names a texture that is nowhere, nothing of its name",
        skeleton: "helm",
        n: 254,
        meshes: &["mg_none"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "3: the game's hood, helm_033 (its mesh names helm, nowhere)",
        skeleton: "helm",
        n: 33,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "6: the game's helm_006 (its mesh names helm_003's PLT)",
        skeleton: "helm",
        n: 6,
        meshes: &[],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "4: a helmet whose mesh names its own name, own PLT and MTR (6 stripes)",
        skeleton: "helm",
        n: 235,
        meshes: &["nm:own"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "5: a helmet whose mesh names a name with an MTR (2 stripes), own PLT",
        skeleton: "helm",
        n: 236,
        meshes: &["nm:mg_nma"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "7: a helmet whose mesh names a TGA of no part's name, own PLT and MTR (6 stripes)",
        skeleton: "helm",
        n: 237,
        meshes: &["nm:mg_red"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "8: a helmet whose mesh names a name with an MTR (2 stripes), own PLT and MTR (6)",
        skeleton: "helm",
        n: 238,
        meshes: &["nm:mg_nma"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "9: a helmet whose mesh names a white TGA (24 bits), nothing of its name",
        skeleton: "helm",
        n: 239,
        meshes: &["mg_white"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "@: a helmet whose mesh names a white TGA of alpha 0, nothing of its name",
        skeleton: "helm",
        n: 241,
        meshes: &["mg_white0"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "%: a part whose mesh names its own name, own PLT and MTR (6 stripes)",
        skeleton: "pmh0",
        n: 241,
        meshes: &["nm:own"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "+: a part whose mesh names a name with an MTR (2 stripes), own PLT",
        skeleton: "pmh0",
        n: 242,
        meshes: &["nm:mg_nma"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "#: a part whose mesh names a TGA of no part's name, own PLT and MTR (6 stripes)",
        skeleton: "pmh0",
        n: 243,
        meshes: &["nm:mg_red"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "=: a part whose mesh names a name with an MTR (2 stripes), own PLT and MTR (6)",
        skeleton: "pmh0",
        n: 244,
        meshes: &["nm:mg_nma"],
        own: &[OWN_PLT, OWN_MTR],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "^: a part whose mesh names a white TGA (24 bits), nothing of its name",
        skeleton: "pmh0",
        n: 248,
        meshes: &["mg_white"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "~: a part whose mesh names a texture that is nowhere, nothing of its name",
        skeleton: "pmh0",
        n: 249,
        meshes: &["mg_none"],
        own: &[],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "$: a helmet of two meshes, an MTR (texture0 pink) and a TGA of no part's name, own PLT",
        skeleton: "helm",
        n: 242,
        meshes: &["mg_mtrb", "mg_red"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[("mg_mtrb", Tex::Mtr("texture0 mg_pink\n"))],
        hak: &[],
    },
    Row {
        label: "&: a helmet whose mesh names a TGA of no part's name, own a TGA and a PLT",
        skeleton: "helm",
        n: 243,
        meshes: &["mg_red"],
        own: &[Tex::Tga(YELLOW), OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "0: a helmet whose mesh names no texture (bitmap NULL), own PLT",
        skeleton: "helm",
        n: 245,
        meshes: &["NULL"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
    Row {
        label: "1: a helmet whose mesh's bitmap names an MTR (texture0 pink), own PLT",
        skeleton: "helm",
        n: 246,
        meshes: &["mg_mtrb"],
        own: &[OWN_PLT],
        human: false,
        armored: false,
        extra: &[("mg_mtrb", Tex::Mtr("texture0 mg_pink\n"))],
        hak: &[],
    },
    Row {
        label: "2: a helmet whose mesh names a texture that is nowhere, own a TGA alone",
        skeleton: "helm",
        n: 247,
        meshes: &["mg_none"],
        own: &[Tex::Tga(MAGENTA)],
        human: false,
        armored: false,
        extra: &[],
        hak: &[],
    },
];

/// The groups shot together, by row letter. The normal maps' groups
/// ([`CLOSE`]) from nearer.
const GROUPS: [&str; 17] = [
    "abcde", "fghi", "jklm", "no", "pqrxAB", "stuvw", "yz", "CDEF", "GHIJKL", "MNPQR", "STUV",
    "WXYZ36", "012", "4578", "%+#=", "Z9@^~", "$&",
];

/// The groups shot from 4 m above, not 5 m level (the normal maps'
/// stripes, on the boxes' sunlit tops), in a darker light.
const CLOSE: [&str; 2] = ["4578", "%+#="];

/// An uncompressed 24-bit TGA of one color.
fn flat_tga(w: u16, h: u16, [r, g, b]: [u8; 3]) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    t.extend(w.to_le_bytes());
    t.extend(h.to_le_bytes());
    t.extend([24, 0]);
    for _ in 0..u32::from(w) * u32::from(h) {
        t.extend([b, g, r]);
    }
    t
}

/// An uncompressed 32-bit TGA of one opaque color: a palette. (With a
/// palette of 24 bits the client drew no flat color, the PLTs a gray green
/// and the loincloths dark, where Moonglow drew the flat color: the first
/// reading, why not read.)
fn flat_palette([r, g, b]: [u8; 3]) -> Vec<u8> {
    let mut t = vec![0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    t.extend(256u16.to_le_bytes());
    t.extend(176u16.to_le_bytes());
    t.extend([32, 8]);
    for _ in 0..256 * 176 {
        t.extend([b, g, r, 255]);
    }
    t
}

/// An 8×8 PLT all of `layer`.
fn flat_plt(layer: u8) -> Vec<u8> {
    let mut f = b"PLT V1  ".to_vec();
    for v in [10u32, 0, 8, 8] {
        f.extend(v.to_le_bytes());
    }
    for _ in 0..64 {
        f.extend([200, layer]);
    }
    f
}

/// A box from `lo` to `hi` as a trimesh node of `parent` named `node`,
/// textured `bitmap`, each face both ways (seen from in front or behind).
pub(super) fn box_node(
    parent: &str,
    node: &str,
    bitmap: &str,
    lo: [f32; 3],
    hi: [f32; 3],
) -> String {
    let v: Vec<[f32; 3]> = (0..8)
        .map(|i| {
            [
                if i & 1 == 0 { lo[0] } else { hi[0] },
                if i & 2 == 0 { lo[1] } else { hi[1] },
                if i & 4 == 0 { lo[2] } else { hi[2] },
            ]
        })
        .collect();
    let quads =
        [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
    let mut faces = Vec::new();
    for q in quads {
        for t in [[q[0], q[1], q[2]], [q[0], q[2], q[3]]] {
            faces.push(format!("    {} {} {} 1 0 0 0 0", t[0], t[1], t[2]));
            faces.push(format!("    {} {} {} 1 0 0 0 0", t[0], t[2], t[1]));
        }
    }
    let verts: Vec<String> = v.iter().map(|p| format!("    {} {} {}", p[0], p[1], p[2])).collect();
    format!(
        "node trimesh {node}\n  parent {parent}\n  ambient 1 1 1\n  diffuse 1 1 1\n  \
         specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  render 1\n  shadow 0\n  \
         verts 8\n{}\n  tverts 1\n    0.5 0.5 0\n  faces {}\n{}\nendnode\n",
        verts.join("\n"),
        faces.len(),
        faces.join("\n")
    )
}

/// [`box_node`] with texture coordinates (each face the whole texture,
/// `v` up the sides), each face smoothed apart and `renderhint
/// NormalAndSpecMapped`: for normal maps (a mesh named `nm:<bitmap>` in a
/// row).
fn uv_box_node(parent: &str, node: &str, bitmap: &str, lo: [f32; 3], hi: [f32; 3]) -> String {
    let v: Vec<String> = (0..8)
        .map(|i| {
            let x = if i & 1 == 0 { lo[0] } else { hi[0] };
            let y = if i & 2 == 0 { lo[1] } else { hi[1] };
            let z = if i & 4 == 0 { lo[2] } else { hi[2] };
            format!("    {x} {y} {z}")
        })
        .collect();
    let quads =
        [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
    let mut faces = Vec::new();
    for (k, q) in quads.iter().enumerate() {
        let group = 1 << k;
        for [a, b, c] in [[0, 1, 2], [0, 2, 3], [0, 2, 1], [0, 3, 2]] {
            faces.push(format!("    {} {} {} {group} {a} {b} {c} 0", q[a], q[b], q[c]));
        }
    }
    format!(
        "node trimesh {node}\n  parent {parent}\n  ambient 1 1 1\n  diffuse 1 1 1\n  \
         specular 0 0 0\n  shininess 1\n  bitmap {bitmap}\n  renderhint NormalAndSpecMapped\n  \
         render 1\n  shadow 0\n  verts 8\n{}\n  tverts 4\n    0 0 0\n    1 0 0\n    \
         1 1 0\n    0 1 0\n  faces {}\n{}\nendnode\n",
        v.join("\n"),
        faces.len(),
        faces.join("\n")
    )
}

/// A part model `name`: a box (`head`: a head's, else a chest's), split
/// into one mesh per texture of `bitmaps`, side by side.
fn part_mdl(name: &str, bitmaps: &[String], head: bool) -> String {
    let (lo, hi) = if head {
        ([-0.11, -0.11, -0.04], [0.11, 0.12, 0.22])
    } else {
        ([-0.24, -0.15, 0.0], [0.24, 0.13, 0.5])
    };
    let mut s = format!(
        "newmodel {name}\nsetsupermodel {name} NULL\nclassification character\n\
         setanimationscale 1\nbeginmodelgeom {name}\nnode dummy {name}\n  parent NULL\nendnode\n"
    );
    let k = bitmaps.len() as f32;
    let width = (hi[0] - lo[0]) / k;
    for (i, b) in bitmaps.iter().enumerate() {
        let x0 = lo[0] + width * i as f32;
        let gap = if bitmaps.len() > 1 { 0.01 } else { 0.0 };
        let (at, to) = ([x0 + gap, lo[1], lo[2]], [x0 + width - gap, hi[1], hi[2]]);
        let node = format!("{name}_m{i}");
        s += &match b.strip_prefix("nm:") {
            Some(b) => uv_box_node(name, &node, b, at, to),
            None => box_node(name, &node, b, at, to),
        };
    }
    s + &format!("endmodelgeom {name}\ndonemodel {name}\n")
}

/// The override's files for the rows: their models and textures, the
/// shared textures and the flat palettes.
fn overrides(rows: &[&Row]) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("mg_red.tga".into(), flat_tga(8, 8, RED)),
        ("mg_green.tga".into(), flat_tga(8, 8, GREEN)),
        ("mg_pltx.plt".into(), flat_plt(6)),
        ("mg_pink.tga".into(), flat_tga(8, 8, PINK)),
        ("mg_nrm.tga".into(), flat_tga(8, 8, [128, 128, 255])),
        // (Normal maps of 2 and 6 stripes; `mg_nma`, a red TGA with a
        // material of its name naming the 2 stripes and no texture0.)
        ("mg_stripe2.tga".into(), file_of(Tex::Stripes(2)).1),
        ("mg_stripe6.tga".into(), file_of(Tex::Stripes(6)).1),
        ("mg_nma.tga".into(), flat_tga(8, 8, RED)),
        ("mg_nma.mtr".into(), NAMED_MTR.as_bytes().to_vec()),
        // (White: 24 bits, and 32 with alpha 0.)
        ("mg_white.tga".into(), flat_tga(8, 8, [255, 255, 255])),
        ("mg_white0.tga".into(), file_of(Tex::Tga32([255, 255, 255], 0)).1),
    ];
    for (palette, color) in PALETTES {
        files.push((format!("{palette}.tga"), flat_palette(color)));
    }
    for part in ["chest", "head"] {
        files.push((format!("pmh0_{part}299.tga"), flat_tga(8, 8, BLUE)));
        files.push((format!("pmh0_{part}298.plt"), flat_plt(1)));
    }
    // (Another helmet's PLT, Hair: cyan.)
    files.push(("helm_248.plt".into(), flat_plt(1)));
    for row in rows {
        for (name, t) in row.extra {
            let (ext, data) = file_of(*t);
            files.push((format!("{name}.{ext}"), data));
        }
        if row.meshes.is_empty() {
            continue;
        }
        if row.skeleton == "helm" {
            let name = format!("helm_{}", row.n);
            let bitmaps: Vec<String> = row
                .meshes
                .iter()
                .map(|m| {
                    let (nm, m) = m.strip_prefix("nm:").map_or(("", *m), |m| ("nm:", m));
                    let t = match m {
                        "own" => name.clone(),
                        "otherhelm" => "helm_248".into(),
                        t => t.into(),
                    };
                    format!("{nm}{t}")
                })
                .collect();
            let mdl = part_mdl(&name, &bitmaps, true);
            files.push((format!("{name}.mdl"), mdl.into()));
            for t in row.own {
                let (ext, data) = file_of(*t);
                files.push((format!("{name}.{ext}"), data));
            }
            continue;
        }
        let robe = row.skeleton.strip_suffix("robe");
        let parts: &[&str] = if robe.is_some() { &["robe"] } else { &["chest", "head"] };
        for &part in parts {
            let name = format!("{}_{part}{}", robe.unwrap_or(row.skeleton), row.n);
            let bitmaps: Vec<String> = row
                .meshes
                .iter()
                .map(|m| {
                    let (nm, m) = m.strip_prefix("nm:").map_or(("", *m), |m| ("nm:", m));
                    let t = match m {
                        "own" => name.clone(),
                        "other299" => format!("pmh0_{part}299"),
                        "other298" => format!("pmh0_{part}298"),
                        t => t.replace('|', "\n  materialname "),
                    };
                    format!("{nm}{t}")
                })
                .collect();
            files.push((format!("{name}.mdl"), part_mdl(&name, &bitmaps, part == "head").into()));
            // (Found only by the fallback: the human man's, phenotype 0.)
            let textured = if row.human { format!("pmh0_{part}{}", row.n) } else { name.clone() };
            for t in row.own {
                let (ext, data) = file_of(*t);
                files.push((format!("{textured}.{ext}"), data));
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files.dedup_by(|a, b| a.0 == b.0);
    files
}

/// The hak of the rows' `hak` textures, if any.
fn hak(rows: &[&Row]) -> Option<Vec<u8>> {
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    for row in rows.iter().filter(|r| !r.hak.is_empty()) {
        for part in ["chest", "head"] {
            let name = ResRef::from_str(&format!("{}_{part}{}", row.skeleton, row.n)).unwrap();
            for t in row.hak {
                let (ext, data) = file_of(*t);
                w.add(name, ResType::from_extension(ext).unwrap(), data).unwrap();
            }
        }
    }
    (!w.is_empty()).then(|| w.to_bytes().unwrap())
}

/// The creature of `row` placed `x` meters east in the reading's area,
/// wearing `armor` where the row says.
fn place(r: &mut Reading, row: &Row, x: f32, items: &HashMap<String, Gff>) {
    use mg_module::instances::{Placement, Placing, instance};
    let robe = row.skeleton.strip_suffix("robe");
    let (appearance, race, phenotype) = match robe.unwrap_or(row.skeleton) {
        "helm" => (6u16, 6u8, 0u8),
        "pme0" => (1, 1, 0),
        "pmd0" => (0, 0, 0),
        "pmh2" => (6, 6, 2),
        _ => (6, 6, 0),
    };
    let bp = Gff::read(&r.game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
    let mut c = bp.root;
    let gender = u8::from(row.skeleton.starts_with("pf"));
    let look =
        mg_preview::CreatureLook { phenotype, gender, ..mg_preview::CreatureLook::new(appearance) };
    for f in look.to_utc().fields {
        c.set(&f.label.to_string_lossy(), f.value);
    }
    c.set("Race", Value::Byte(race));
    let n = row.n;
    let mut set = |label: &str| {
        c.set(label, Value::Byte(n as u8));
        c.set(&mg_rules::items::wide_label(label), Value::Word(n));
    };
    let helmet = row.skeleton == "helm";
    if !helmet && robe.is_none() {
        set("Appearance_Head");
    }
    if !row.armored && !helmet && robe.is_none() {
        set("BodyPart_Torso");
    }
    if row.meshes.is_empty() && !helmet {
        set("BodyPart_LHand");
        set("BodyPart_RHand");
        c.set("Color_Skin", Value::Byte(28));
    }
    let mut worn = Vec::new();
    if row.armored {
        let mut e = mg_gff::Struct::new(0x2);
        e.set("EquippedRes", Value::resref(ResRef::from_str("mg_armor").unwrap()));
        worn.push(e);
    }
    if robe.is_some() {
        let mut e = mg_gff::Struct::new(0x2);
        let name = format!("mg_robe{n}");
        e.set("EquippedRes", Value::resref(ResRef::from_str(&name).unwrap()));
        worn.push(e);
    }
    if helmet {
        let mut e = mg_gff::Struct::new(0x1);
        let name = format!("mg_helm{n}");
        e.set("EquippedRes", Value::resref(ResRef::from_str(&name).unwrap()));
        worn.push(e);
    }
    c.set("Equip_ItemList", Value::List(worn));
    c.set("ItemList", Value::List(Vec::new()));
    let scripts: Vec<String> = c
        .fields
        .iter()
        .map(|f| f.label.to_string_lossy())
        .filter(|l| l.starts_with("Script"))
        .collect();
    for label in scripts {
        c.set(&label, Value::resref(ResRef::EMPTY));
    }
    let item = |r: ResRef| items.get(r.as_str()?).map(|g| g.root.clone());
    let placing = Placing { game: &r.game, item: &item };
    // (A normal map's row nearer the player, seen from above.)
    let y = if row.meshes.iter().any(|m| m.starts_with("nm:")) { 21.0 } else { 22.2 };
    let at = Placement { position: [x, y, 0.0], rotation: std::f32::consts::PI };
    let placed = instance(&placing, ResType::UTC, &c, at, &[]).unwrap();
    let mut list = r.git.root.list("Creature List").map(<[_]>::to_vec).unwrap_or_default();
    list.push(placed);
    r.git.root.set("Creature List", Value::List(list));
}

/// An armor whose torso is `torso` and robe `robe` (its other parts the
/// game's clothes'), as the module's `name`.
fn armor(game: &GameData, torso: u16, robe: u16, name: &str) -> Gff {
    let mut a = Gff::read(&game.resman.get_named("nw_cloth001", ResType::UTI).unwrap()).unwrap();
    for (label, n) in [("ArmorPart_Torso", torso), ("ArmorPart_Robe", robe)] {
        a.root.set(label, Value::Byte(n as u8));
        let wide = mg_rules::items::wide_label(label);
        if a.root.get(&wide).is_some() || n > 255 {
            a.root.set(&wide, Value::Word(n));
        }
    }
    a.root.set("PropertiesList", Value::List(Vec::new()));
    a.root.set("TemplateResRef", Value::resref(ResRef::from_str(name).unwrap()));
    a
}

/// A helmet whose model is `n` (`helm_<n>`), as the module's `mg_helm<n>`.
fn helmet(game: &GameData, n: u16) -> Gff {
    let mut keys: Vec<ResKey> = game
        .resman
        .entries()
        .iter()
        .map(|(k, _)| *k)
        .filter(|k| k.restype == ResType::UTI)
        .collect();
    keys.sort_by_key(|k| k.to_string());
    let mut h = keys
        .iter()
        .filter_map(|k| Gff::read(&game.resman.get(k).ok()?).ok())
        .find(|g| g.root.integer("BaseItem") == Some(17))
        .expect("a helmet");
    h.root.set("ModelPart1", Value::Byte(n as u8));
    h.root.set("PropertiesList", Value::List(Vec::new()));
    let name = format!("mg_helm{n}");
    h.root.set("TemplateResRef", Value::resref(ResRef::from_str(&name).unwrap()));
    h
}

/// The x of the `i`th of `k` creatures.
fn x_of(i: usize, k: usize) -> f32 {
    20.0 + (i as f32 - (k as f32 - 1.0) / 2.0) * 0.8
}

#[test]
#[ignore]
fn part_textures_look() {
    let _ = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let kept = mg_testkit::test_output().join("part_textures");
    std::fs::create_dir_all(&kept).unwrap();
    let side = std::env::var("MG_SIDE").unwrap_or_else(|_| "moonglow".into());
    let moonglow_only = std::env::var_os("MG_MOONGLOW_ONLY").is_some();
    let only = std::env::var("MG_GROUPS").ok();
    for g in GROUPS {
        for (i, c) in g.chars().enumerate() {
            let row = ROWS.iter().find(|r| r.label.starts_with(c)).unwrap();
            eprintln!("{} (x {:.1})", row.label, x_of(i, g.len()));
        }
    }
    for group in GROUPS {
        if only.as_deref().is_some_and(|o| !o.split(',').any(|g| g == group)) {
            continue;
        }
        // (The normal maps' groups lit darker around and brighter by the
        // sun, so their stripes show.)
        let close = CLOSE.contains(&group);
        let (ambient, diffuse) = if close { (0x303030, 0xFFFFFF) } else { (0x909090, 0xC0C0C0) };
        let light = Lighting { ambient, diffuse, main_light: 0, only_tile: None };
        let rows: Vec<&Row> =
            group.chars().map(|c| ROWS.iter().find(|r| r.label.starts_with(c)).unwrap()).collect();
        let files = overrides(&rows);
        let refs: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
        let mut r = Reading::new(&format!("part_textures_{group}"), "tic01", light, &refs);
        let mut items = HashMap::new();
        items.insert("mg_armor".to_string(), armor(&r.game, 201, 0, "mg_armor"));
        for row in rows.iter().filter(|r| r.skeleton == "helm") {
            items.insert(format!("mg_helm{}", row.n), helmet(&r.game, row.n));
        }
        let robes: Vec<u16> =
            rows.iter().filter(|r| r.skeleton.ends_with("robe")).map(|r| r.n).collect();
        for &n in &robes {
            let name = format!("mg_robe{n}");
            items.insert(name.clone(), armor(&r.game, 1, n, &name));
        }
        for (name, g) in &items {
            let key = ResKey::new(ResRef::from_str(name).unwrap(), ResType::UTI);
            r.m.set_gff(key, g).unwrap();
        }
        if !robes.is_empty() {
            // (The robes' rows, hiding nothing, in a copy of the game's
            // table: the area view reads it again with the override.)
            let table = r.game.resman.get_named("parts_robe", ResType::TWODA).unwrap();
            let mut text = String::from_utf8_lossy(&table).trim_end().to_string();
            let last: u16 =
                text.lines().last().unwrap().split_whitespace().next().unwrap().parse().unwrap();
            let columns = text.lines().nth(2).unwrap().split_whitespace().count();
            for n in last + 1..=*robes.iter().max().unwrap() {
                let cells: Vec<&str> = (0..columns)
                    .map(|c| match (robes.contains(&n), c) {
                        (false, _) => "****",
                        (true, 1) => "0.00",
                        (true, _) => "0",
                    })
                    .collect();
                text += &format!("\n{n} {}", cells.join(" "));
            }
            std::fs::write(r.dir.join("user/override/parts_robe.2da"), text + "\n").unwrap();
        }
        for (i, row) in rows.iter().enumerate() {
            place(&mut r, row, x_of(i, rows.len()), &items);
        }
        let hak = hak(&rows);
        if let Some(data) = &hak {
            std::fs::create_dir_all(r.dir.join("user/hak")).unwrap();
            std::fs::write(r.dir.join("user/hak/mg_parts.hak"), data).unwrap();
            let mut info = r.m.info().unwrap();
            info.root
                .set("Mod_HakList", mg_module::attach::hak_list(&info.root, &["mg_parts".into()]));
            r.m.set_info(&info).unwrap();
        }
        let dir = r.dir.clone();
        let (r_are, r_git) = (r.are.clone(), r.git.clone());
        let client_png = kept.join(format!("{group}_client.png"));
        // (From above, for the sunlit tops of the normal maps' boxes.)
        let (distance, pitch) = if close { (4.0, 40.0) } else { (5.0, 75.0) };
        let client = if moonglow_only {
            r.save(&enter(INVISIBLE, (90.0, distance, pitch)), SETTINGS);
            common::read_png(&client_png)
        } else {
            let (_, shot) = r.shoot(&enter(INVISIBLE, (90.0, distance, pitch)), SETTINGS);
            print_log(&dir);
            save_png(&shot, &client_png);
            shot
        };
        let haks: Vec<String> = hak.iter().map(|_| "mg_parts".to_string()).collect();
        let camera = if close { "3.68,39,1.0,48" } else { "4.6,74,1.0,48" };
        moonglow(&dir, &r_are, &r_git, &haks, group, &side, &kept, &client, camera);
    }
    eprintln!("pictures: {}", kept.display());
}

/// Moonglow's area view of the group's scene (in `dir`), from about where
/// the client looks, kept as `<group>_<side>.png` and `_on.png`.
#[allow(clippy::too_many_arguments)]
fn moonglow(
    dir: &Path,
    are: &Gff,
    git: &Gff,
    haks: &[String],
    group: &str,
    side: &str,
    kept: &Path,
    client: &Rgba,
    camera: &str,
) {
    mg_testkit::gpu::hold();
    let Some(gpu) = Gpu::headless() else {
        eprintln!("Moonglow's side skipped: no GPU");
        return;
    };
    let install = GameInstall::new(mg_testkit::nwn_root().unwrap(), Some(dir.join("user")), "en");
    let mut game = GameData::open(&install).unwrap();
    game.resman.add_haks(&install, haks).unwrap();
    let model = mg_area::AreaModel::read(
        &game,
        &are.root,
        &git.root,
        mg_area::tileset(&game, ResRef::from_str("tic01").unwrap()).ok().as_ref(),
    );
    for o in &model.objects {
        if o.problem.is_some() {
            eprintln!("object {:?} problem {:?}", o.kind, o.problem);
        }
    }
    let area_scene = mg_area::AreaScene::new(&gpu, &game, &model);
    let scene =
        area_scene.scene(&model, &mg_area::View { fog: false, ..mg_area::View::of(&model) });
    let c: Vec<f32> = std::env::var("MG_CAMERA")
        .unwrap_or_else(|_| camera.into())
        .split(',')
        .map(|v| v.parse().unwrap())
        .collect();
    let camera = fitted_camera(Vec3::new(20.0, 20.0, 0.0), c[3], c[2], c[0], c[1]);
    off_and_on(
        &gpu,
        &game,
        &scene,
        &camera,
        (client.width, client.height),
        Some(client),
        kept,
        &format!("{group}_{side}"),
    );
}

/// The owner's module's parts that the rule of [`part_textures_look`]
/// draws otherwise than Moonglow drew them before it (The Domains of
/// Dread's haks, `MOONGLOW_HAKS`: its build folder, the module beside it
/// as `../tdod.mod`, whose hak list the scene's module takes). Set 1
/// (`MG_TDOD_SET`), west to east: a human woman and a halfling woman in
/// clothes whose biceps are 25 (`p<g><r>0_bicepl025`, whose meshes name
/// `cfh0_bicepl056`, a PLT of no part's name, beside `pfh0_bicepl025.plt`),
/// a human woman of head 146 (`pfh0_head146`, a mesh naming the material
/// `pfh0_head146w`) and a halfling woman in robe 250 (`pfa0_robe250`,
/// naming `pmh0_robe231`, beside `pmh0_robe250.plt`). Set 2: human men in
/// feet 65 (two meshes naming the TGAs `ad_wd_oak_lights` and
/// `ad_wrap_browns`), feet 92 (`band_foot02`, a DDS), neck 22 (naming the
/// material `pmh0_neck028`) and biceps 98 (`qmh0_bicepr056`), and a
/// halfling man in neck 28 (`1cape`, beside the human's PLT and its own
/// DDS). Pictures in `target/test-output/part_textures/tdod<set>_*.png`;
/// `MG_SIDE` and `MG_MOONGLOW_ONLY` as [`part_textures_look`]'s. Skips
/// without the haks.
///
/// Seen (2026-11-10): the client draws each as the rule does (the biceps
/// in their own PLTs' green cloth where Moonglow drew black leather, the
/// robe brown where it drew black, feet 92 black boots where it drew the
/// bandage); head 146 is drawn by neither. Set 3 (GitHub issue 45):
/// human men in helmets `helm_017` (its mesh naming `pmh0_head001`'s
/// PLT), `036` (`m_helmet`, nowhere), `071` (a TGA and two names
/// nowhere), `076` (`black`, a TGA, and another helmet's PLT), `096`
/// (TGAs of other names) and `033` (TDoD's hood, its own name). Seen
/// (2026-11-13): the client draws each in the helmet's own PLT, as the
/// rule does (36 dark where Moonglow drew it shiny gray, 96's horns dark
/// where it drew them blue).
#[test]
#[ignore]
fn part_textures_tdod_look() {
    let Some(dir) =
        mg_testkit::builder_hak_dirs().into_iter().find(|d| d.join("../tdod.mod").is_file())
    else {
        eprintln!("skipped: MOONGLOW_HAKS names no folder beside tdod.mod");
        return;
    };
    let _ = oracle_tool!("nwn_script_comp");
    let kept = mg_testkit::test_output().join("part_textures");
    std::fs::create_dir_all(&kept).unwrap();
    let side = std::env::var("MG_SIDE").unwrap_or_else(|_| "moonglow".into());
    let light = Lighting { ambient: 0x909090, diffuse: 0xC0C0C0, main_light: 0, only_tile: None };
    let mut r = Reading::new("part_textures_tdod", "tic01", light, &[]);
    std::fs::create_dir_all(r.dir.join("user/hak")).unwrap();
    for f in mg_testkit::files_with_ext(&dir, "hak") {
        let to = r.dir.join("user/hak").join(f.file_name().unwrap());
        mg_testkit::link_or_copy(&std::fs::canonicalize(&f).unwrap(), &to).unwrap();
    }
    let tdod = mg_module::Module::open(&dir.join("../tdod.mod")).unwrap();
    let list = tdod.info().unwrap().root.get("Mod_HakList").cloned().unwrap();
    let mut info = r.m.info().unwrap();
    info.root.set("Mod_HakList", list);
    let haks: Vec<String> = info
        .root
        .list("Mod_HakList")
        .unwrap()
        .iter()
        .filter_map(|h| h.string("Mod_Hak"))
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .collect();
    r.m.set_info(&info).unwrap();
    let install = GameInstall::new(mg_testkit::nwn_root().unwrap(), Some(r.dir.join("user")), "en");
    r.game = GameData::open(&install).unwrap();
    r.game.resman.add_haks(&install, &haks).unwrap();
    // The creatures of the set asked for (`MG_TDOD_SET`, 1 by default):
    // appearance, gender, head, and the clothes' parts (none: nothing
    // worn); robe 250 a robe; `Helm`, a helmet of that model worn.
    type Creature = (u16, u8, u16, &'static [(&'static str, u16)]);
    let sets: [&[Creature]; 3] = [
        &[
            (6, 1, 1, &[("LBicep", 25), ("RBicep", 25)]),
            (3, 1, 1, &[("LBicep", 25), ("RBicep", 25)]),
            (6, 1, 146, &[]),
            (3, 1, 1, &[("Robe", 250)]),
        ],
        &[
            (6, 0, 1, &[("LFoot", 65), ("RFoot", 65)]),
            (6, 0, 1, &[("LFoot", 92), ("RFoot", 92)]),
            (6, 0, 1, &[("Neck", 22)]),
            (3, 0, 1, &[("Neck", 28)]),
            (6, 0, 1, &[("LBicep", 98), ("RBicep", 98)]),
        ],
        &[
            (6, 0, 1, &[("Helm", 17)]),
            (6, 0, 1, &[("Helm", 36)]),
            (6, 0, 1, &[("Helm", 71)]),
            (6, 0, 1, &[("Helm", 76)]),
            (6, 0, 1, &[("Helm", 96)]),
            (6, 0, 1, &[("Helm", 33)]),
        ],
    ];
    let set: usize = std::env::var("MG_TDOD_SET").ok().and_then(|v| v.parse().ok()).unwrap_or(1);
    let creatures = sets[set - 1];
    let mut items = HashMap::new();
    for (i, (_, _, _, parts)) in creatures.iter().enumerate() {
        if parts.is_empty() {
            continue;
        }
        let name = format!("mg_worn{i}");
        if let [("Helm", n)] = parts {
            let mut h = helmet(&r.game, *n);
            h.root.set("TemplateResRef", Value::resref(ResRef::from_str(&name).unwrap()));
            items.insert(name, h);
            continue;
        }
        let mut clothes = armor(&r.game, 1, 0, &name);
        for (part, n) in *parts {
            let label = format!("ArmorPart_{part}");
            clothes.root.set(&label, Value::Byte(*n as u8));
            let wide = mg_rules::items::wide_label(&label);
            if clothes.root.get(&wide).is_some() {
                clothes.root.set(&wide, Value::Word(*n));
            }
        }
        items.insert(name, clothes);
    }
    for (name, g) in &items {
        let key = ResKey::new(ResRef::from_str(name).unwrap(), ResType::UTI);
        r.m.set_gff(key, g).unwrap();
    }
    let k = creatures.len();
    for (i, &(appearance, gender, head, parts)) in creatures.iter().enumerate() {
        let worn = if parts.is_empty() { String::new() } else { format!("mg_worn{i}") };
        use mg_module::instances::{Placement, Placing, instance};
        let bp = Gff::read(&r.game.resman.get_named("nw_commale", ResType::UTC).unwrap()).unwrap();
        let mut c = bp.root;
        let look = mg_preview::CreatureLook { gender, ..mg_preview::CreatureLook::new(appearance) };
        for f in look.to_utc().fields {
            c.set(&f.label.to_string_lossy(), f.value);
        }
        c.set("Race", Value::Byte(appearance as u8));
        c.set("Appearance_Head", Value::Byte(head as u8));
        c.set(&mg_rules::items::wide_label("Appearance_Head"), Value::Word(head));
        let slot = if matches!(parts, [("Helm", _)]) { 0x1 } else { 0x2 };
        let mut e = mg_gff::Struct::new(slot);
        e.set("EquippedRes", Value::resref(ResRef::from_str(&worn).unwrap_or(ResRef::EMPTY)));
        c.set("Equip_ItemList", Value::List(if worn.is_empty() { vec![] } else { vec![e] }));
        c.set("ItemList", Value::List(Vec::new()));
        let scripts: Vec<String> = c
            .fields
            .iter()
            .map(|f| f.label.to_string_lossy())
            .filter(|l| l.starts_with("Script"))
            .collect();
        for label in scripts {
            c.set(&label, Value::resref(ResRef::EMPTY));
        }
        let item = |r: ResRef| items.get(r.as_str()?).map(|g| g.root.clone());
        let placing = Placing { game: &r.game, item: &item };
        let at = Placement { position: [x_of(i, k), 22.2, 0.0], rotation: std::f32::consts::PI };
        let placed = instance(&placing, ResType::UTC, &c, at, &[]).unwrap();
        let mut list = r.git.root.list("Creature List").map(<[_]>::to_vec).unwrap_or_default();
        list.push(placed);
        r.git.root.set("Creature List", Value::List(list));
    }
    let rdir = r.dir.clone();
    let (are, git) = (r.are.clone(), r.git.clone());
    let group = format!("tdod{set}");
    let client_png = kept.join(format!("{group}_client.png"));
    let entry = enter(INVISIBLE, (90.0, 4.0, 78.0))
        .replace("DelayCommand(4.0, Log(\"MG_READY\"))", "DelayCommand(25.0, Log(\"MG_READY\"))");
    let client = if std::env::var_os("MG_MOONGLOW_ONLY").is_some() {
        r.save(&entry, SETTINGS);
        common::read_png(&client_png)
    } else {
        let (_, shot) = r.shoot(&entry, SETTINGS);
        print_log(&rdir);
        save_png(&shot, &client_png);
        shot
    };
    moonglow(&rdir, &are, &git, &haks, &group, &side, &kept, &client, "4.6,74,1.0,48");
}
