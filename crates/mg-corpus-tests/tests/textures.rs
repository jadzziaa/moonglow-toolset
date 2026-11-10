//! Every texture in the base game decodes (TGA, DDS, PLT) and every TXI and
//! MTR parses; a sample of textures decodes to the same pixels as Pillow's
//! decoders (BioWare DDS given to Pillow as the standard DDS
//! `mg_image::dds::write_standard` writes of it, every level).

use std::io::Write;
use std::process::Command;

use mg_core::ResType;
use mg_image::mtr::Mtr;
use mg_image::plt::Plt;
use mg_image::txi::{Blending, Txi};
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_testkit::{corpus, scratch_dir};
use rayon::prelude::*;

fn textures(rm: &ResMan, t: ResType) -> Vec<ResKey> {
    let mut keys: Vec<ResKey> =
        rm.entries().into_iter().map(|(k, _)| k).filter(|k| k.restype == t).collect();
    keys.sort();
    keys
}

#[test]
fn every_texture_decodes() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    for t in [ResType::TGA, ResType::DDS] {
        let keys = textures(&rm, t);
        assert!(keys.len() > 1000, "too few {t}");
        let keys = mg_testkit::sweep(keys, 10);
        let failed: Vec<String> = keys
            .par_iter()
            .filter_map(|k| {
                let data = rm.get(k).ok()?;
                match mg_image::read(t, &data) {
                    Ok(tex) => {
                        let (w, h) = (tex.width, tex.height);
                        let ok = tex.mips[0].len() == tex.format.level_size(w, h);
                        (!ok).then(|| format!("{k}: level 0 has the wrong size"))
                    }
                    Err(e) => Some(format!("{k}: {e}")),
                }
            })
            .collect();
        eprintln!("{t}: {} files, {} failed", keys.len(), failed.len());
        assert!(failed.is_empty(), "{failed:#?}");
    }
    let keys = textures(&rm, ResType::PLT);
    assert!(keys.len() > 1000);
    let keys = mg_testkit::sweep(keys, 10);
    let failed: Vec<String> = keys
        .par_iter()
        .filter_map(|k| Plt::read(&rm.get(k).ok()?).err().map(|e| format!("{k}: {e}")))
        .collect();
    eprintln!("plt: {} files, {} failed", keys.len(), failed.len());
    assert!(failed.is_empty(), "{failed:#?}");

    // TXI and MTR: known settings.
    let txi = |n: &str| Txi::parse(&rm.get_named(n, ResType::TXI).unwrap());
    assert_eq!(txi("fxpa_flame02").cycle(), Some((4, 4, 32.0)));
    assert_eq!(txi("fxpa_flame02").blending(), Blending::Additive);
    assert!(txi("tts01__env").cube());
    let all_txi = textures(&rm, ResType::TXI);
    let parsed = all_txi.iter().filter(|k| !Txi::parse(&rm.get(k).unwrap()).entries.is_empty());
    eprintln!("txi: {} files, {} with settings", all_txi.len(), parsed.count());
    let mtrs = textures(&rm, ResType::MTR);
    for k in &mtrs {
        let m = Mtr::parse(&rm.get(k).unwrap());
        assert!(m.shader_fs.is_some() || m.textures.iter().any(Option::is_some), "{k}");
    }
    let ice = Mtr::parse(&rm.get_named("tti01_iceclear", ResType::MTR).unwrap());
    eprintln!("mtr: {} files; tti01_iceclear: {ice:?}", mtrs.len());
}

const COMPARE: &str = r#"
import sys
from PIL import Image
bad = 0
for line in open(sys.argv[1]):
    name, src, ours, alpha = line.split()
    im = Image.open(src).convert("RGBA")
    theirs = im.tobytes()
    mine = open(ours, "rb").read()
    if im.size[0] * im.size[1] * 4 != len(mine):
        print(name, "size", im.size); bad += 1; continue
    n = 4 if alpha == "1" else 3
    diff = 0
    worst = 0
    for i in range(0, len(mine), 4):
        for c in range(n):
            d = abs(mine[i + c] - theirs[i + c])
            if d:
                diff += 1
                worst = max(worst, d)
    if diff:
        print(name, "differs", diff, "worst", worst); bad += 1
print("compared", sum(1 for _ in open(sys.argv[1])), "bad", bad)
"#;

#[test]
fn textures_match_pillow() {
    let root = corpus!();
    let python_ok = Command::new("python3")
        .args(["-c", "import PIL"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !python_ok {
        eprintln!("skipped: python3 with Pillow not found");
        return;
    }
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = scratch_dir("textures_pillow");
    let mut list = std::fs::File::create(dir.join("list.txt")).unwrap();
    let mut count = 0;
    for (t, step) in [(ResType::TGA, 70), (ResType::DDS, 25)] {
        // (A sample of the sample on an everyday run.)
        let picked: Vec<_> = textures(&rm, t).into_iter().step_by(step).collect();
        for k in mg_testkit::sweep(picked, 6) {
            let data = rm.get(&k).unwrap();
            let tex = mg_image::read(t, &data).unwrap();
            let src =
                dir.join(format!("{}.{}", k.resref, if t == ResType::DDS { "dds" } else { "tga" }));
            if t == ResType::DDS && tex.alpha_mean.is_some() {
                // (Written by Moonglow's own writer: Pillow reads its header.)
                std::fs::write(&src, mg_image::dds::write_standard(&tex).unwrap()).unwrap();
            } else {
                std::fs::write(&src, &data).unwrap();
            }
            let ours = dir.join(format!("{}.{}.rgba", k.resref, t.extension().unwrap_or("bin")));
            // Pillow gives a TGA top row first, and a DDS in stored order.
            // (It honors a TGA's top-left origin, which the game and
            // Moonglow ignore: such a file's rows are already top first.)
            let stored_top_first = t == ResType::DDS || data.get(17).is_some_and(|d| d & 0x20 != 0);
            let rgba = if stored_top_first { tex.to_rgba() } else { tex.to_rgba().top_down() };
            std::fs::write(&ours, rgba.data).unwrap();
            let alpha = if tex.has_alpha { 1 } else { 0 };
            writeln!(list, "{k} {} {} {alpha}", src.display(), ours.display()).unwrap();
            count += 1;
        }
    }
    drop(list);
    let out =
        Command::new("python3").args(["-c", COMPARE]).arg(dir.join("list.txt")).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    eprintln!("{text}{}", String::from_utf8_lossy(&out.stderr));
    assert!(out.status.success());
    assert!(count > if mg_testkit::full_corpus() { 300 } else { 50 });
    assert!(text.contains(&format!("compared {count} bad 0")), "{text}");
}

#[test]
fn coloured_parts_take_the_plt_over_a_grey_tga_of_the_same_name() {
    use mg_render::{Assets, colored_name};
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    // A dwarf's head is both a PLT and a plain grey TGA (or DDS).
    let head = mg_core::ResRef::from_str("pmd0_head001").unwrap();
    assert!(rm.get(&ResKey::new(head, ResType::PLT)).is_ok());
    let (t, data) = rm.texture(head).expect("the plain texture is there too");
    let grey = mg_image::read(t, &data).unwrap().to_rgba();
    let skin = |c: u8| {
        let mut colors = [0u8; 10];
        colors[0] = c;
        let t = Assets::texture(&rm, &colored_name("pmd0_head001", colors)).unwrap();
        t.texture.to_rgba()
    };
    // In colours: the PLT, so the skin colour changes the head.
    assert_ne!(skin(0).data, skin(10).data);
    assert_ne!(skin(0).data, grey.data);
    // Without colours, the plain texture as before.
    let plain = Assets::texture(&rm, "pmd0_head001").unwrap().texture.to_rgba();
    assert_eq!(plain.data, grey.data);
}
