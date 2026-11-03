//! Item icons as the game client's inventory and examine window draw them
//! (GitHub issue 41): which way up a DDS icon is drawn, which file the
//! game takes when a DDS and a TGA of one name are both there, and what it
//! draws for a DDS alone.
//!
//! Each case is a small miscellaneous item (base item 24, `iit_smlmisc_NNN`,
//! 32×32) whose icon is one picture of four colored quarters, written as
//! one kind of file, in the scratch user directory's `override`, a hak or
//! the module. Every file stores the same rows: the first stored row is
//! the red and green half, the last the blue and yellow one, so Moonglow
//! (first stored row at the bottom) shows every case red and green below.
//! Where a DDS and a TGA of one name are both there, the TGA's picture has
//! other colors (white and magenta below cyan and orange: [`OTHER`]), so
//! that the picture says which file was drawn as well as which way up.
//! Each case is a client run of its own: the player's own items destroyed,
//! the case's item made in the bag (named for its case), the inventory
//! opened (`PopUpGUIPanel`) and the item examined (`ActionExamine`), one
//! picture of both. (One run of every case, examined in turn, gave
//! pictures whose examine window was a case behind.)
//!
//! Moonglow's picture of the same files (as `ResMan::texture_over` finds
//! them with the module, drawn top down) is written beside the client's,
//! `moonglow.png`.
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_render reading_icon -- --ignored --nocapture`.

use super::readings::{Reading, print_log};
use super::*;

/// An icon's side.
const SIDE: u32 = 32;

/// A picture's quarters: the first stored rows' left and right half, then
/// the last stored rows' left and right half.
type Quarters = [[u8; 3]; 4];

/// The cases' picture: red and green first, blue and yellow last.
const FIRST: Quarters = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 255, 0]];

/// The TGA of a pair: white and magenta first, cyan and orange last.
const OTHER: Quarters = [[255, 255, 255], [255, 0, 255], [0, 255, 255], [255, 128, 0]];

/// Where a file of a case is put.
#[derive(Clone, Copy, PartialEq)]
enum At {
    /// The scratch user directory's `override`.
    Override,
    /// The module's first (highest) hak.
    Hak,
    /// The module's second hak, under the first.
    LowerHak,
    /// The module itself.
    Module,
}

/// How a file is written.
#[derive(Clone, Copy)]
enum Kind {
    /// A TGA, its rows stored bottom first (its origin bottom left).
    Tga,
    /// A TGA that declares a top-left origin (bit 5 of its descriptor),
    /// its rows stored in the same order.
    TgaTopLeft,
    /// A BioWare DDS (no magic), DXT1 (3 channels) or DXT5 (4), with its
    /// mip chain.
    BioWare(u32),
    /// A standard `DDS ` with its mip chain, DXT1 or DXT5.
    Standard(&'static [u8; 4]),
    /// A standard `DDS `, uncompressed 32-bit BGRA, one level.
    StandardRgba,
    /// The game's own `iit_smlmisc_015.dds` (a BioWare DDS, a pod of seeds),
    /// as it is shipped; the picture is not used.
    GameDds,
}

/// A case's file: where it is put, how it is written, its picture.
type CaseFile = (At, Kind, Quarters);

/// The cases: the icon's number, what it is, its files.
const CASES: &[(u32, &str, &[CaseFile])] = &[
    (240, "standard DDS DXT1", &[(At::Override, Kind::Standard(b"DXT1"), FIRST)]),
    (241, "standard DDS DXT5", &[(At::Override, Kind::Standard(b"DXT5"), FIRST)]),
    (242, "BioWare DDS DXT1", &[(At::Override, Kind::BioWare(3), FIRST)]),
    (243, "BioWare DDS DXT5", &[(At::Override, Kind::BioWare(4), FIRST)]),
    (244, "TGA", &[(At::Override, Kind::Tga, FIRST)]),
    (245, "TGA declaring top left", &[(At::Override, Kind::TgaTopLeft, FIRST)]),
    (
        246,
        "standard DDS and TGA, override",
        &[(At::Override, Kind::Standard(b"DXT5"), FIRST), (At::Override, Kind::Tga, OTHER)],
    ),
    (
        247,
        "BioWare DDS and TGA, override",
        &[(At::Override, Kind::BioWare(4), FIRST), (At::Override, Kind::Tga, OTHER)],
    ),
    (
        248,
        "BioWare DDS and TGA, one hak",
        &[(At::Hak, Kind::BioWare(4), FIRST), (At::Hak, Kind::Tga, OTHER)],
    ),
    (
        249,
        "TGA in the upper hak, DDS in the lower",
        &[(At::LowerHak, Kind::Standard(b"DXT5"), FIRST), (At::Hak, Kind::Tga, OTHER)],
    ),
    (
        250,
        "DDS in a hak, TGA in override",
        &[(At::Hak, Kind::Standard(b"DXT5"), FIRST), (At::Override, Kind::Tga, OTHER)],
    ),
    (251, "standard DDS DXT5 in a hak", &[(At::Hak, Kind::Standard(b"DXT5"), FIRST)]),
    (252, "standard DDS uncompressed", &[(At::Override, Kind::StandardRgba, FIRST)]),
    (253, "the game's own DDS alone", &[(At::Override, Kind::GameDds, FIRST)]),
    (
        254,
        "DDS in a hak, TGA in the module",
        &[(At::Hak, Kind::Standard(b"DXT5"), FIRST), (At::Module, Kind::Tga, OTHER)],
    ),
    (
        255,
        "TGA in a hak, DDS in the module",
        &[(At::Module, Kind::Standard(b"DXT5"), FIRST), (At::Hak, Kind::Tga, OTHER)],
    ),
    (15, "TGA in a hak over the game's DDS and TGA", &[(At::Hak, Kind::Tga, OTHER)]),
    (238, "no picture at all", &[]),
    (
        239,
        "DDS in the upper hak, TGA in the lower",
        &[(At::Hak, Kind::Standard(b"DXT5"), FIRST), (At::LowerHak, Kind::Tga, OTHER)],
    ),
];

/// The picture of `side`×`side`, its rows in stored order.
fn picture(q: &Quarters, side: u32) -> Rgba {
    picture_of(q, side, side)
}

/// The picture of `w`×`h`, its rows in stored order.
fn picture_of(q: &Quarters, w: u32, h: u32) -> Rgba {
    let mut img = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let c = q[usize::from(y >= h / 2) * 2 + usize::from(x >= w / 2)];
            let i = ((y * w + x) * 4) as usize;
            img.data[i..i + 4].copy_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    img
}

/// `img` halved, each pixel the mean of four (rows kept in their order).
fn halved(img: &Rgba) -> Rgba {
    let (w, h) = ((img.width / 2).max(1), (img.height / 2).max(1));
    let mut out = Rgba::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let mut sum = [0u32; 4];
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let p =
                    img.pixel((x * 2 + dx).min(img.width - 1), (y * 2 + dy).min(img.height - 1));
                for c in 0..4 {
                    sum[c] += u32::from(p[c]);
                }
            }
            let i = ((y * w + x) * 4) as usize;
            for (o, s) in out.data[i..i + 4].iter_mut().zip(sum) {
                *o = (s / 4) as u8;
            }
        }
    }
    out
}

/// One level, block-compressed: each 4×4 block of the rows in stored order
/// one color (its mean), DXT1 (`alpha` false) or DXT5.
fn blocks(img: &Rgba, alpha: bool) -> Vec<u8> {
    let mut out = Vec::new();
    for by in 0..img.height.div_ceil(4) {
        for bx in 0..img.width.div_ceil(4) {
            let mut sum = [0u32; 3];
            let mut n = 0;
            for y in by * 4..(by * 4 + 4).min(img.height) {
                for x in bx * 4..(bx * 4 + 4).min(img.width) {
                    let p = img.pixel(x, y);
                    for c in 0..3 {
                        sum[c] += u32::from(p[c]);
                    }
                    n += 1;
                }
            }
            let [r, g, b] = sum.map(|s| (s / n) as u16);
            let c = (r >> 3) << 11 | (g >> 2) << 5 | b >> 3;
            if alpha {
                out.extend([255u8, 255, 0, 0, 0, 0, 0, 0]);
            }
            // (Both ends the color, every index 0.)
            out.extend(c.to_le_bytes());
            out.extend(c.to_le_bytes());
            out.extend([0u8; 4]);
        }
    }
    out
}

/// Every level of `img` down to 1×1, block-compressed.
fn chain(img: &Rgba, alpha: bool) -> (Vec<u8>, u32) {
    let (mut data, mut level, mut count) = (Vec::new(), img.clone(), 0);
    loop {
        data.extend(blocks(&level, alpha));
        count += 1;
        if level.width == 1 && level.height == 1 {
            return (data, count);
        }
        level = halved(&level);
    }
}

/// A standard DDS header: `flags`, the pitch or linear size, the mip
/// count, the pixel format's flags, FourCC, bit count and masks, caps.
fn standard_header(
    side: u32,
    flags: u32,
    pitch: u32,
    mips: u32,
    format: [u32; 7],
    caps: u32,
) -> Vec<u8> {
    let mut h = vec![124, flags, side, side, pitch, 0, mips];
    h.extend([0; 11]);
    h.push(32);
    h.extend(format);
    h.extend([caps, 0, 0, 0, 0]);
    let mut out = b"DDS ".to_vec();
    for v in h {
        out.extend(v.to_le_bytes());
    }
    out
}

/// The file of `q` written as `kind`.
fn file(kind: Kind, q: &Quarters, game_dds: &[u8]) -> Vec<u8> {
    let img = picture(q, SIDE);
    match kind {
        Kind::GameDds => game_dds.to_vec(),
        Kind::Tga | Kind::TgaTopLeft => {
            let descriptor = if matches!(kind, Kind::TgaTopLeft) { 0x28 } else { 8 };
            let side = (SIDE as u16).to_le_bytes();
            let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
            out.extend(side);
            out.extend(side);
            out.extend([32, descriptor]);
            for p in img.data.as_chunks::<4>().0 {
                out.extend([p[2], p[1], p[0], p[3]]);
            }
            out
        }
        Kind::BioWare(channels) => {
            let (data, _) = chain(&img, channels == 4);
            let top = if channels == 4 { SIDE * SIDE } else { SIDE * SIDE / 2 };
            let mut out = Vec::new();
            for v in [SIDE, SIDE, channels, top] {
                out.extend(v.to_le_bytes());
            }
            out.extend(1.0f32.to_le_bytes());
            out.extend(data);
            out
        }
        Kind::Standard(four_cc) => {
            let alpha = four_cc == b"DXT5";
            let (data, mips) = chain(&img, alpha);
            let top = if alpha { SIDE * SIDE } else { SIDE * SIDE / 2 };
            let format = [0x4, u32::from_le_bytes(*four_cc), 0, 0, 0, 0, 0];
            let mut out = standard_header(SIDE, 0x000A_1007, top, mips, format, 0x0040_1008);
            out.extend(data);
            out
        }
        Kind::StandardRgba => {
            let format = [0x41, 0, 32, 0x00ff_0000, 0x0000_ff00, 0x0000_00ff, 0xff00_0000];
            let mut out = standard_header(SIDE, 0x0000_100f, SIDE * 4, 1, format, 0x1000);
            for p in img.data.as_chunks::<4>().0 {
                out.extend([p[2], p[1], p[0], p[3]]);
            }
            out
        }
    }
}

/// The extension of `kind`'s files.
fn ext(kind: Kind) -> &'static str {
    match kind {
        Kind::Tga | Kind::TgaTopLeft => "tga",
        _ => "dds",
    }
}

/// The names of the module's haks, the higher first.
const HAKS: [&str; 2] = ["mg_icons_a", "mg_icons_b"];

/// The entry script: the player's own items destroyed, the case's item
/// made in the bag (named for its case), the inventory opened and the item
/// examined.
fn entry(n: u32, what: &str) -> String {
    format!(
        r#"
void Log(string s) {{ WriteTimestampedLogEntry(s); }}
void Make(object pc)
{{
    object o = CreateItemOnObject("mg_icon_{n}", pc);
    SetName(o, "{n} {what}");
    Log("MG_MADE " + IntToString(GetIsObjectValid(o)));
}}
void main()
{{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    ApplyEffectToObject(DURATION_TYPE_PERMANENT,
        EffectVisualEffect(VFX_DUR_CUTSCENE_INVISIBILITY), pc);
    object o = GetFirstItemInInventory(pc);
    while (GetIsObjectValid(o)) {{
        DestroyObject(o);
        o = GetNextItemInInventory(pc);
    }}
    DelayCommand(0.5, Make(pc));
    DelayCommand(2.0, PopUpGUIPanel(pc, GUI_PANEL_INVENTORY));
    DelayCommand(2.5, AssignCommand(pc, ActionExamine(GetItemPossessedBy(pc, "MG_ICON_{n}"))));
    DelayCommand(5.0, Log("MG_READY"));
}}
"#
    )
}

/// Moonglow's picture of icon `n` as [`mg_resman::ResMan::texture_over`]
/// finds it with the module `m` (as the window's pictures take it), drawn
/// top down; and what it took.
fn moonglow_icon(game: &GameData, m: &mg_module::Module, n: u32) -> (Option<Rgba>, String) {
    let name = ResRef::from_str(&format!("iit_smlmisc_{n:03}")).unwrap();
    let module = |k: &ResKey| m.get(k).map(std::borrow::Cow::Borrowed);
    let Some((restype, data, label)) = game.resman.texture_over(name, module) else {
        return (None, "none".into());
    };
    let shown = mg_image::read(restype, &data).unwrap().to_rgba().top_down();
    (Some(shown), format!("{restype} from {label}"))
}

/// One case run in the client: the module with the case's files, the
/// item made, the inventory opened and the item examined; Moonglow's
/// picture of the icon (`moonglow.png`) beside the client's
/// (`client.png`). Moonglow's picture and what it took.
fn run_case(
    root: &Path,
    game_dds: &[u8],
    (n, what, files): (u32, &str, &[CaseFile]),
) -> (Option<Rgba>, String) {
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let mut overrides = Vec::new();
    let mut hak_files: [Vec<(ResRef, ResType, Vec<u8>)>; 2] = [Vec::new(), Vec::new()];
    let mut module_files = Vec::new();
    let resref = ResRef::from_str(&format!("iit_smlmisc_{n:03}")).unwrap();
    for &(at, kind, q) in files {
        let restype = if ext(kind) == "dds" { ResType::DDS } else { ResType::TGA };
        let data = file(kind, &q, game_dds);
        match at {
            At::Override => overrides.push((format!("{resref}.{}", ext(kind)), data)),
            At::Hak => hak_files[0].push((resref, restype, data)),
            At::LowerHak => hak_files[1].push((resref, restype, data)),
            At::Module => module_files.push((resref, restype, data)),
        }
    }
    let overrides: Vec<(&str, Vec<u8>)> =
        overrides.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
    let mut r = Reading::new(&format!("icon_{n}"), "tic01", light, &overrides);

    // The haks (those with files: the game does not load a module with an
    // empty one), and the module's own files.
    let hak_dir = r.dir.join("user/hak");
    std::fs::create_dir_all(&hak_dir).unwrap();
    let mut haks = Vec::new();
    for (name, files) in HAKS.iter().zip(&hak_files).filter(|(_, f)| !f.is_empty()) {
        let mut hak = mg_erf::ErfWriter::new(*b"HAK ");
        for (resref, restype, data) in files {
            hak.add(*resref, *restype, data.as_slice()).unwrap();
        }
        std::fs::write(hak_dir.join(format!("{name}.hak")), hak.to_bytes().unwrap()).unwrap();
        haks.push(name.to_string());
    }
    let mut info = r.m.info().unwrap();
    let list = mg_module::attach::hak_list(&info.root, &haks);
    info.root.set("Mod_HakList", list);
    r.m.set_info(&info).unwrap();
    for (resref, restype, data) in module_files {
        r.m.set(ResKey::new(resref, restype), data);
    }

    // The item: a small miscellaneous item of the game's, made the case's
    // appearance.
    let base = r
        .game
        .resman
        .list(ResType::UTI)
        .into_iter()
        .filter_map(|name| {
            let data = r.game.resman.get(&ResKey::new(name, ResType::UTI)).ok()?;
            let gff = Gff::read(&data).ok()?;
            (gff.root.int("BaseItem") == Some(24)).then_some(gff)
        })
        .next()
        .expect("a small miscellaneous item in the game");
    let mut item = base;
    let template = format!("mg_icon_{n}");
    item.root.set("TemplateResRef", Value::ResRef(template.as_bytes().to_vec()));
    item.root.set("Tag", Value::String(format!("MG_ICON_{n}").into_bytes()));
    item.root.set("ModelPart1", Value::Byte(n as u8));
    item.root.set("Identified", Value::Byte(1));
    item.root.set("Plot", Value::Byte(0));
    item.root.set("StackSize", Value::Word(1));
    r.m.set_gff(ResKey::parse(&template, ResType::UTI).unwrap(), &item).unwrap();

    // Moonglow's side: the game as the module opens it, with its haks.
    let install = GameInstall::new(root, Some(r.dir.join("user")), "en");
    let mut game = GameData::open(&install).unwrap();
    game.resman.add_haks(&install, &haks).unwrap();
    let (shown, took) = moonglow_icon(&game, &r.m, n);
    if let Some(shown) = &shown {
        common::save_png(shown, &r.dir.join("moonglow.png"));
    }
    eprintln!("{n} {what}: Moonglow takes {took}");

    let (dir, _) = r.shoot(&entry(n, what), SETTINGS);
    print_log(&dir);
    (shown, took)
}

/// Item icons in the game client's inventory and examine window, each
/// case of [`CASES`] in a client run of its own, beside Moonglow's picture
/// of it. Moonglow's pictures side by side, in [`CASES`]' order:
/// `client_reading_icons/moonglow.png`; the client's in
/// `client_reading_icon_<n>/client.png`. `MG_ICON_CASES` (numbers, by
/// commas) runs those alone.
///
/// Seen 2026-11-02 (89.8193.37), the inventory's slot and the examine
/// window alike:
///
/// - Rows: every icon drawn has its first stored row at the bottom, as
///   Moonglow draws it: a TGA (244), one that declares a top-left origin
///   (245), a standard DDS DXT5 (246, 249, 239, 254, 255) and a BioWare DDS
///   DXT5 (247, 248). (Neither kind's DXT1, nor an uncompressed DDS, was
///   drawn with a TGA beside it.)
/// - Which file: the DDS where both are in one class of layers, the
///   directories' over the ERFs' (`ResMan::texture_over`'s rule): both in
///   `override` (246, 247) or in one hak (248), the DDS in the lower hak
///   and the TGA in the upper (249) or the other way (239), the DDS in the
///   module and the TGA in a hak (255) or the other way (254); a TGA in
///   `override` over a DDS in a hak (250); a TGA in a hak over the game's
///   DDS and TGA (15).
/// - A DDS with no TGA of its name anywhere is not drawn: the base item's
///   `DefaultIcon` (`iinvalid_1x1`, a red cross) is, as with no picture at
///   all (238). So for both kinds of DDS (240 to 243), in a hak (251),
///   uncompressed (252) and the game's own `iit_smlmisc_015.dds` (253).
///
/// See `docs/research/notes_models.md` §E.4.
#[test]
#[ignore]
fn reading_icon_rows() {
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    let base_game = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let game_dds = base_game.resman.get_named("iit_smlmisc_015", ResType::DDS).unwrap();
    let only: Option<Vec<u32>> = std::env::var("MG_ICON_CASES")
        .ok()
        .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect());
    const SCALE: u32 = 4;
    let cell = SIDE * SCALE + 8;
    let mut strip = Rgba::new(cell * CASES.len() as u32, SIDE * SCALE);
    for (i, &(n, what, files)) in CASES.iter().enumerate() {
        if only.as_ref().is_some_and(|o| !o.contains(&n)) {
            continue;
        }
        let (shown, _) = run_case(&root, &game_dds, (n, what, files));
        let Some(shown) = shown else { continue };
        for y in 0..SIDE * SCALE {
            for x in 0..SIDE * SCALE {
                let p = shown.pixel(x / SCALE, y / SCALE);
                let at = ((y * strip.width + i as u32 * cell + x) * 4) as usize;
                strip.data[at..at + 4].copy_from_slice(&p);
            }
        }
    }
    let dir = scratch_dir("client_reading_icons");
    common::save_png(&strip, &dir.join("moonglow.png"));
    eprintln!("Moonglow's: {}", dir.join("moonglow.png").display());
}

/// A portrait whose pictures are only DDS, or only TGA: whether the game
/// needs a TGA for a portrait as it does for an item icon. The player's
/// portrait (`SetPortraitResRef`) is `po_mgtst_`, its five sizes (`h`,
/// `l`, `m`, `s`, `t`: 256×512 down to 16×32) in the scratch `override`,
/// each the quarters of [`FIRST`] (red and green the first stored rows),
/// one run each: `client_reading_portrait_dds/` and
/// `client_reading_portrait_tga/`, Moonglow's `m` picture (`images.rs`'s
/// `portrait`, drawn top down) beside the client's as `moonglow.png`.
///
/// Seen 2026-11-03 (89.8193.37): the party bar's portrait is drawn from
/// the DDS alone as from the TGA alone, the same way up (red and green,
/// the first stored rows, at the bottom), the top 100 of its 128 rows:
/// a portrait needs no TGA, unlike an item icon. Moonglow draws both so.
/// See `docs/research/notes_models.md` §E.4.
#[test]
#[ignore]
fn reading_portrait_dds() {
    let light = Lighting { ambient: 0x404040, diffuse: 0x808080, main_light: 0, only_tile: None };
    let root = mg_testkit::nwn_root().expect("the game (NWN_ROOT)");
    for dds in [true, false] {
        let mut files = Vec::new();
        for (size, w) in [('h', 256u32), ('l', 128), ('m', 64), ('s', 32), ('t', 16)] {
            let img = picture_of(&FIRST, w, w * 2);
            let data = if dds {
                let (data, _) = chain(&img, false);
                let mut out = Vec::new();
                // (DXT1: 8 bytes a 4×4 block, w × 2w.)
                for v in [w, w * 2, 3, w * w] {
                    out.extend(v.to_le_bytes());
                }
                out.extend(1.0f32.to_le_bytes());
                out.extend(data);
                out
            } else {
                let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
                out.extend((w as u16).to_le_bytes());
                out.extend((w as u16 * 2).to_le_bytes());
                out.extend([32, 8]);
                for p in img.data.as_chunks::<4>().0 {
                    out.extend([p[2], p[1], p[0], p[3]]);
                }
                out
            };
            let ext = if dds { "dds" } else { "tga" };
            files.push((format!("po_mgtst_{size}.{ext}"), data));
        }
        let files: Vec<(&str, Vec<u8>)> =
            files.iter().map(|(n, d)| (n.as_str(), d.clone())).collect();
        let name = if dds { "portrait_dds" } else { "portrait_tga" };
        let r = Reading::new(name, "tic01", light, &files);
        let game =
            GameData::open(&GameInstall::new(&root, Some(r.dir.join("user")), "en")).unwrap();
        let took = game.resman.texture(ResRef::from_str("po_mgtst_m").unwrap());
        if let Some((t, data)) = took {
            let shown = mg_image::read(t, &data).unwrap().to_rgba().top_down();
            common::save_png(&shown, &r.dir.join("moonglow.png"));
        }
        let body =
            format!("{}\n    SetPortraitResRef(pc, \"po_mgtst_\");", super::readings::INVISIBLE);
        let (dir, _) = r.shoot(&super::readings::enter(&body, (90.0, 12.0, 45.0)), SETTINGS);
        print_log(&dir);
    }
}
