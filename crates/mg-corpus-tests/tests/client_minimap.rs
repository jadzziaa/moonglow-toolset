//! The minimap Moonglow exports (`mg_module::minimap`) laid out as the game
//! client's map draws it: which way each tile's picture turns with the tile,
//! and which row of tiles is at the top.
//!
//! The area is 4 tiles wide and 3 high, each tile turned `index % 4`
//! quarter turns. Every tile picture of the tileset is replaced (in the
//! scratch user directory's `override`) by one in four colored quarters,
//! bottom left red, bottom right green, top left blue, top right yellow; the
//! first tile's by one with white instead of red, to find it. The client
//! explores the area and opens its map (`PopUpGUIPanel`), and is
//! screenshotted; the map is the largest patch of those colors. Each
//! quarter's color there must be the color of the same quarter in
//! Moonglow's minimap (but for the quarter or two the player's arrow
//! covers). Run once with TGA pictures and once with DDS (DXT1), whose rows
//! are stored the other way up, and once with TGA pictures that declare a
//! top-left origin (as Krita writes them).
//!
//! Needs the game, a GPU and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --test client_minimap -- --ignored --nocapture`.

use mg_core::{ResRef, ResType};
use mg_gff::Value;
use mg_image::Rgba;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{SETTINGS, client_screenshot, compile, save_png};

const WIDTH: u32 = 4;
const HEIGHT: u32 = 3;
const TILESET: &str = "tic01";

const ENTER: &str = r#"
void Log(string s) { WriteTimestampedLogEntry(s); }
void Show(object pc)
{
    ExploreAreaForPlayer(GetArea(pc), pc, TRUE);
    PopUpGUIPanel(pc, GUI_PANEL_MINIMAP);
}
void main()
{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    DelayCommand(3.0, Show(pc));
    DelayCommand(6.0, Log("MG_READY"));
}
"#;

/// The colors, by name.
const COLORS: [(char, [u8; 3]); 5] = [
    ('R', [255, 0, 0]),
    ('G', [0, 255, 0]),
    ('B', [0, 0, 255]),
    ('Y', [255, 255, 0]),
    ('W', [255, 255, 255]),
];

/// A picture's quarters: bottom left, bottom right, top left, top right.
fn quarters(marked: bool) -> [[u8; 3]; 4] {
    let bl = if marked { COLORS[4].1 } else { COLORS[0].1 };
    [bl, COLORS[1].1, COLORS[2].1, COLORS[3].1]
}

/// The color of a quarter of a 16×16 picture at (x, y), y counted from the
/// bottom.
fn color_at(q: &[[u8; 3]; 4], x: u32, y: u32) -> [u8; 3] {
    q[usize::from(y >= 8) * 2 + usize::from(x >= 8)]
}

/// An uncompressed 24-bit TGA, rows stored bottom first (the usual way).
fn tga(q: &[[u8; 3]; 4]) -> Vec<u8> {
    let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 16, 0, 24, 0];
    for y in 0..16 {
        for x in 0..16 {
            let [r, g, b] = color_at(q, x, y);
            out.extend([b, g, r]);
        }
    }
    out
}

/// The same picture as a TGA that declares a top-left origin (bit 5 of its
/// descriptor), its rows stored top first: what Krita writes.
fn tga_top_left(q: &[[u8; 3]; 4]) -> Vec<u8> {
    let mut out = vec![0u8, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 16, 0, 24, 0x20];
    for y in (0..16).rev() {
        for x in 0..16 {
            let [r, g, b] = color_at(q, x, y);
            out.extend([b, g, r]);
        }
    }
    out
}

/// A standard DDS, DXT1, its 4×4 blocks stored top row first.
fn dds(q: &[[u8; 3]; 4]) -> Vec<u8> {
    let le = |v: u32| v.to_le_bytes();
    let mut out = b"DDS ".to_vec();
    let header: [u32; 31] = [
        124,
        0x0008_1007,
        16,
        16,
        128,
        0,
        1,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0, // ..reserved
        32,
        0x4,
        u32::from_le_bytes(*b"DXT1"),
        0,
        0,
        0,
        0,
        0,
        0x1000,
        0,
        0,
        0,
        0,
    ];
    for h in header {
        out.extend(le(h));
    }
    for by in 0..4 {
        for bx in 0..4 {
            // The block's rows from the top: picture rows from the top.
            let y_from_bottom = 15 - by * 4;
            let [r, g, b] = color_at(q, bx * 4, y_from_bottom);
            let c = (u16::from(r) >> 3) << 11 | (u16::from(g) >> 2) << 5 | u16::from(b) >> 3;
            out.extend(c.to_le_bytes());
            out.extend(c.to_le_bytes());
            out.extend([0u8; 4]);
        }
    }
    out
}

/// The nearest of [`COLORS`], if near one.
fn classify(p: [u8; 4]) -> Option<char> {
    let [r, g, b, _] = p.map(i32::from);
    let hi = |v: i32| v > 120;
    let lo = |v: i32| v < 90;
    match (hi(r), hi(g), hi(b)) {
        (true, true, true) if r > 180 && g > 180 && b > 180 => Some('W'),
        (true, true, false) if lo(b) => Some('Y'),
        (true, false, false) if lo(g) && lo(b) => Some('R'),
        (false, true, false) if lo(r) && lo(b) => Some('G'),
        (false, false, true) if lo(r) && lo(g) => Some('B'),
        _ => None,
    }
}

/// Each tile's quarters' colors, top row of tiles first, read from the
/// part of `img` (rows top first) inside `rect` (x0, y0, x1, y1).
fn read_grid(img: &Rgba, rect: (u32, u32, u32, u32)) -> Vec<String> {
    let (x0, y0, x1, y1) = rect;
    let (tw, th) = (f64::from(x1 - x0) / f64::from(WIDTH), f64::from(y1 - y0) / f64::from(HEIGHT));
    let mut rows = Vec::new();
    for ty in 0..HEIGHT {
        let mut row = String::new();
        for tx in 0..WIDTH {
            // Quarters: top left, top right, bottom left, bottom right.
            for (qx, qy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let mut votes = std::collections::HashMap::new();
                for dy in -2..=2 {
                    for dx in -2..=2 {
                        let x = f64::from(x0) + (f64::from(tx) + qx) * tw + f64::from(dx);
                        let y = f64::from(y0) + (f64::from(ty) + qy) * th + f64::from(dy);
                        let p = img.pixel(x as u32, y as u32);
                        *votes.entry(classify(p).unwrap_or('?')).or_insert(0) += 1;
                    }
                }
                let best = votes.into_iter().max_by_key(|(_, n)| *n).map_or('?', |(c, _)| c);
                row.push(best);
            }
            row.push(' ');
        }
        rows.push(row.trim_end().to_string());
    }
    rows
}

/// The bounding box of the largest connected patch of [`COLORS`] (not
/// white, which the GUI has too).
fn map_rect(img: &Rgba) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = (img.width as usize, img.height as usize);
    let colored: Vec<bool> = (0..w * h)
        .map(|i| classify(img.pixel((i % w) as u32, (i / w) as u32)).is_some_and(|c| c != 'W'))
        .collect();
    let mut seen = vec![false; w * h];
    let mut best: Option<(usize, (u32, u32, u32, u32))> = None;
    for start in 0..w * h {
        if !colored[start] || seen[start] {
            continue;
        }
        let (mut n, mut bb) = (0usize, (u32::MAX, u32::MAX, 0u32, 0u32));
        let mut stack = vec![start];
        seen[start] = true;
        while let Some(i) = stack.pop() {
            n += 1;
            let (x, y) = ((i % w) as u32, (i / w) as u32);
            bb = (bb.0.min(x), bb.1.min(y), bb.2.max(x + 1), bb.3.max(y + 1));
            // Neighbors, bridging the white quarters and lines between
            // tiles: up to 3 pixels away.
            for (dx, dy) in
                [(1i64, 0i64), (-1, 0), (0, 1), (0, -1), (3, 0), (-3, 0), (0, 3), (0, -3)]
            {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if colored[j] && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        if best.is_none_or(|(m, _)| n > m) {
            best = Some((n, bb));
        }
    }
    best.map(|(_, bb)| bb)
}

#[test]
#[ignore]
fn minimaps_are_laid_out_as_the_client_draws_them() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let mut failures = Vec::new();
    for (format, restype) in
        [("tga", ResType::TGA), ("tga_top_left", ResType::TGA), ("dds", ResType::DDS)]
    {
        let dir = scratch_dir(&format!("client_minimap_{format}"));
        let user = dir.join("user");
        std::fs::create_dir_all(user.join("override")).unwrap();
        std::fs::write(user.join("settings.tml"), SETTINGS).unwrap();

        // The area: tiles turned 0 to 3 quarter turns; the first tile one
        // whose picture differs from the others'.
        let mut rng = fastrand::Rng::with_seed(5);
        let mut m = new_module(&base, "MgMap", &mut rng).unwrap();
        let spec = AreaSpec {
            name: "Map".into(),
            tileset: ResRef::from_str(TILESET).unwrap(),
            width: WIDTH,
            height: HEIGHT,
        };
        let area = add_area(&mut m, &base, &spec, &mut rng).unwrap();
        let are_key = ResKey::new(area, ResType::ARE);
        let mut are = m.gff(&are_key).unwrap().unwrap();
        let set = mg_module::minimap::tileset(&base.resman, &are.root).unwrap();
        let picture = |id: i64| set.tiles[id as usize].image_map_2d.clone().unwrap_or_default();
        let tiles = are.root.list_mut("Tile_List").unwrap();
        let common = tiles[1].integer("Tile_ID").unwrap();
        let marker = (0..set.tiles.len() as i64)
            .find(|&i| {
                let p = picture(i);
                !p.is_empty() && p != picture(common) && !p.eq_ignore_ascii_case("(null)")
            })
            .unwrap();
        for (i, t) in tiles.iter_mut().enumerate() {
            t.set("Tile_ID", Value::Int(if i == 0 { marker as i32 } else { common as i32 }));
            t.set("Tile_Orientation", Value::Int((i % 4) as i32));
        }
        m.set_gff(are_key, &are).unwrap();
        let enter = compile(&dir, "mg_enter", ENTER);
        m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), enter);
        let mut info = m.info().unwrap();
        info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
        m.set_info(&info).unwrap();
        m.save_as(&ModuleLocation::Archive(user.join("modules/MgMap.mod"))).unwrap();

        // Every picture replaced.
        let ext = restype.extension().unwrap();
        for t in &set.tiles {
            let Some(name) = t.image_map_2d.as_ref().filter(|n| !n.is_empty()) else { continue };
            let marked = name.eq_ignore_ascii_case(&picture(marker));
            let q = quarters(marked);
            let data = match format {
                "tga" => tga(&q),
                "tga_top_left" => tga_top_left(&q),
                _ => dds(&q),
            };
            std::fs::write(user.join(format!("override/{}.{ext}", name.to_lowercase())), data)
                .unwrap();
        }

        // Moonglow's minimap, with the same pictures.
        let game = GameData::open(&GameInstall::new(&root, Some(user.clone()), "en")).unwrap();
        let ours = mg_module::minimap::minimap(&game.resman, &are.root, &set, Some(16)).unwrap();
        let ours = ours.top_down();
        save_png(&ours, &dir.join("moonglow.png"));
        let want = read_grid(&ours, (0, 0, ours.width, ours.height));

        let Some(client) = client_screenshot(&dir, "MgMap") else {
            panic!("{format}: the client did not reach the area (see {})", dir.display());
        };
        let Some(rect) = map_rect(&client) else {
            failures
                .push(format!("{format}: no map found in {}", dir.join("client.png").display()));
            continue;
        };
        let got = read_grid(&client, rect);
        eprintln!("{format}: map at {rect:?}\n  client   {got:?}\n  moonglow {want:?}");
        // The player's arrow covers a quarter or two.
        let same = |a: &String, b: &String| {
            a.len() == b.len() && a.chars().zip(b.chars()).all(|(x, y)| x == y || x == '?')
        };
        let covered = got.iter().flat_map(|r| r.chars()).filter(|&c| c == '?').count();
        if covered > 2 || !got.iter().zip(&want).all(|(a, b)| same(a, b)) {
            failures.push(format!("{format}: client {got:?}, Moonglow {want:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// Map notes marked as the client's map marks them (2026-11-05, for El
/// Grillo's ask: minimaps exported with their notes marked). The same
/// area, every tile picture one even gray, with four waypoints: two with
/// an enabled map note (west and east), one whose note is disabled and one
/// with none. The client's map (`client.png`) marks the two enabled ones
/// alone, each by `ctl_map_pin`'s diamond centered on the waypoint, 12
/// pixels where a tile is 16 (the text shows only under the pointer).
/// Moonglow's minimap at 16 pixels a tile, marked (`moonglow.png`), must
/// have its marked pixels where the client's are: each note's within two
/// pixels of the client's middle, and most of them the same pixels. (The
/// client places the marker at whole pixels and spreads the area's meters
/// over the map less 2 pixels: read with fourteen notes on this area and
/// on one 8 by 6, each marker the client's pixel for pixel once placed so,
/// at most 2 pixels from where Moonglow marks the waypoint itself.)
#[test]
#[ignore]
fn map_notes_are_marked_as_the_client_marks_them() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    mg_testkit::gpu::hold();
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = scratch_dir("client_minimap_notes");
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("override")).unwrap();
    std::fs::write(user.join("settings.tml"), SETTINGS).unwrap();
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(&base, "MgMap", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Map".into(),
        tileset: ResRef::from_str(TILESET).unwrap(),
        width: WIDTH,
        height: HEIGHT,
    };
    let area = add_area(&mut m, &base, &spec, &mut rng).unwrap();
    let are = m.gff(&ResKey::new(area, ResType::ARE)).unwrap().unwrap();
    let set = mg_module::minimap::tileset(&base.resman, &are.root).unwrap();
    let gray = [[96, 96, 96]; 4];
    for t in &set.tiles {
        let Some(name) = t.image_map_2d.as_ref().filter(|n| !n.is_empty()) else { continue };
        std::fs::write(user.join(format!("override/{}.tga", name.to_lowercase())), tga(&gray))
            .unwrap();
    }
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let notes = [
        ("MG_ALPHA", [5.0, 25.0], Some((true, "Alpha note"))),
        ("MG_BETA", [25.0, 25.0], Some((false, "Beta note"))),
        ("MG_GAMMA", [35.0, 5.0], None),
        ("MG_DELTA", [32.0, 12.0], Some((true, "Delta note"))),
    ];
    let mut list = Vec::new();
    for (tag, [x, y], note) in notes {
        let mut w = mg_module::instances::walk_waypoint(tag.as_bytes(), [x, y, 0.0]);
        if let Some((enabled, text)) = note {
            w.set("HasMapNote", Value::Byte(1));
            w.set("MapNoteEnabled", Value::Byte(u8::from(enabled)));
            let mut l = mg_core::LocString::default();
            l.set(mg_core::Language::ENGLISH, mg_core::Gender::Male, text);
            w.set("MapNote", Value::LocString(l));
        }
        list.push(w);
    }
    git.root.set("WaypointList", Value::List(list));
    m.set_gff(git_key, &git).unwrap();
    // (Where the server has each waypoint, logged: `MG_AT tag x y`.)
    let enter = ENTER.replace(
        "    DelayCommand(6.0, Log(\"MG_READY\"));",
        "    string t; object w; int i;\n    for (i = 0; i < 4; i++) {\n        t = \
         i == 0 ? \"MG_ALPHA\" : i == 1 ? \"MG_BETA\" : i == 2 ? \"MG_GAMMA\" : \"MG_DELTA\";\n        \
         w = GetWaypointByTag(t);\n        Log(\"MG_AT \" + t + \" \" + FloatToString(GetPosition(w).x, 0, 3) + \
         \" \" + FloatToString(GetPosition(w).y, 0, 3));\n    }\n    DelayCommand(6.0, Log(\"MG_READY\"));",
    );
    assert!(enter.contains("MG_AT"));
    let enter = compile(&dir, "mg_enter", &enter);
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), enter);
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/MgMap.mod"))).unwrap();

    // Moonglow's, marked.
    let game = GameData::open(&GameInstall::new(&root, Some(user.clone()), "en")).unwrap();
    let mut ours = mg_module::minimap::minimap(&game.resman, &are.root, &set, Some(16)).unwrap();
    let marked = mg_module::minimap::map_notes(
        &git.root,
        mg_core::Language::ENGLISH,
        mg_core::Codepage::default(),
    );
    assert_eq!(marked.len(), 2, "the enabled notes alone: {marked:?}");
    let pin = mg_module::minimap::pin(&game.resman).unwrap();
    eprintln!(
        "pin: {}x{} texels, {:?} pixels, offset {:?}",
        pin.image.width, pin.image.height, pin.size, pin.offset
    );
    mg_module::minimap::mark(&mut ours, 16, &marked, &pin);
    let ours = ours.top_down();
    save_png(&ours, &dir.join("moonglow.png"));

    let client = client_screenshot(&dir, "MgMap").expect("the client reached the area");
    let log = std::fs::read_to_string(dir.join("user/logs/nwclientLog1.txt")).unwrap_or_default();
    for line in log.lines().filter(|l| l.contains("MG_AT")) {
        eprintln!("{line}");
    }
    // The map: the box of the gray the game draws the tiles in (the most
    // common color of the top left corner's quarter of the window but black).
    let mut counts = std::collections::HashMap::new();
    for y in 0..client.height / 4 {
        for x in 0..client.width / 4 {
            let p = client.pixel(x, y);
            if p[0] == p[1] && p[1] == p[2] && (40..120).contains(&p[0]) {
                *counts.entry(p[0]).or_insert(0) += 1;
            }
        }
    }
    let gray = counts.into_iter().max_by_key(|(_, n)| *n).map(|(g, _)| g).unwrap();
    let is_gray = |p: [u8; 4]| p[..3] == [gray; 3];
    let rows: Vec<u32> = (0..client.height / 4)
        .filter(|&y| (0..client.width / 4).filter(|&x| is_gray(client.pixel(x, y))).count() > 30)
        .collect();
    let (y0, y1) = (rows[0], rows[rows.len() - 1] + 1);
    let cols: Vec<u32> = (0..client.width / 4)
        .filter(|&x| (y0..y1).filter(|&y| is_gray(client.pixel(x, y))).count() > 20)
        .collect();
    let (x0, x1) = (cols[0], cols[cols.len() - 1] + 1);
    eprintln!("map at ({x0}, {y0}) to ({x1}, {y1}), gray {gray}, in {}", dir.display());
    assert_eq!((x1 - x0, y1 - y0), (WIDTH * 16, HEIGHT * 16), "16 pixels a tile");
    // Marked: not the map's gray, by more than a little.
    let off = |p: [u8; 4], g: u8| p[..3].iter().any(|&c| c.abs_diff(g) > 12);
    let theirs = |x: u32, y: u32| off(client.pixel(x0 + x, y0 + y), gray);
    let mine = |x: u32, y: u32| off(ours.pixel(x, y), 96);
    let (w, h) = (WIDTH * 16, HEIGHT * 16);
    let mut failures = Vec::new();
    for note in &marked {
        // The note's square: three quarters of a tile about it.
        let (cx, cy) = (note.position[0] * 1.6, h as f32 - note.position[1] * 1.6);
        let near =
            |x: u32, y: u32| (x as f32 + 0.5 - cx).abs() < 9.0 && (y as f32 + 0.5 - cy).abs() < 9.0;
        let cells: Vec<(u32, u32)> =
            (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).filter(|&(x, y)| near(x, y)).collect();
        let middle = |is: &dyn Fn(u32, u32) -> bool| {
            let hit: Vec<_> = cells.iter().filter(|&&(x, y)| is(x, y)).collect();
            let n = hit.len().max(1) as f32;
            let sx: f32 = hit.iter().map(|(x, _)| *x as f32 + 0.5).sum();
            let sy: f32 = hit.iter().map(|(_, y)| *y as f32 + 0.5).sum();
            (hit.len(), [sx / n, sy / n])
        };
        let (nt, mt) = middle(&theirs);
        let (nm, mm) = middle(&mine);
        let both = cells.iter().filter(|&&(x, y)| theirs(x, y) && mine(x, y)).count();
        let either = cells.iter().filter(|&&(x, y)| theirs(x, y) || mine(x, y)).count();
        let share = both as f32 / either.max(1) as f32;
        eprintln!(
            "{}: client {nt} marked about {mt:?}, Moonglow {nm} about {mm:?}, {both} of {either} the same",
            note.tag
        );
        if nt == 0 || (mt[0] - mm[0]).abs() > 2.0 || (mt[1] - mm[1]).abs() > 2.0 || share < 0.6 {
            failures.push(note.tag.clone());
        }
    }
    // Nothing else marked: the disabled note and the plain waypoint (the
    // player's arrow aside, at the area's middle).
    let others = |is: &dyn Fn(u32, u32) -> bool| {
        (0..h)
            .flat_map(|y| (0..w).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let mx = x as f32 + 0.5;
                let my = y as f32 + 0.5;
                let by_note = marked.iter().any(|n| {
                    (mx - n.position[0] * 1.6).abs() < 9.0
                        && (my - (h as f32 - n.position[1] * 1.6)).abs() < 9.0
                });
                let arrow =
                    (mx - w as f32 / 2.0).abs() < 10.0 && (my - h as f32 / 2.0).abs() < 10.0;
                !by_note && !arrow && is(x, y)
            })
            .count()
    };
    assert_eq!((others(&theirs), others(&mine)), (0, 0), "nothing else marked");
    assert!(failures.is_empty(), "{failures:?} (see {})", dir.display());
}
