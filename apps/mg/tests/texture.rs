//! `mg texture convert`: textures between PNG, TGA and both DDS headers,
//! the way up kept; a DDS kept in its format copied; resizing, alpha
//! bleed, and what is refused before anything is written; and
//! `mg texture info`, their facts; PLTs split into PNGs and joined.

use std::path::Path;

use mg_image::Rgba;

/// A picture of 8×8, as a PNG shows it (top row first): the top half
/// red, the bottom blue (a 4×4 block of one color each), the left half's
/// alpha 255 and the right
/// half's `right_alpha`.
fn png(path: &Path, right_alpha: u8) {
    let mut data = Vec::new();
    for y in 0..8 {
        for x in 0..8 {
            let c = match y {
                0..4 => [255, 0, 0],
                _ => [0, 0, 255],
            };
            data.extend_from_slice(&[c[0], c[1], c[2], if x < 4 { 255 } else { right_alpha }]);
        }
    }
    std::fs::write(path, mg_image::png::write_rgba(8, 8, &data).unwrap()).unwrap();
}

/// A PNG's pixels, top row first.
fn read_png(path: &Path) -> Rgba {
    mg_image::png::read_picture(&std::fs::read(path).unwrap(), 1 << 20).unwrap()
}

fn convert(args: &[&str]) -> (bool, serde_json::Value) {
    let mut all = vec!["texture", "convert"];
    all.extend(args);
    crate::json_out(crate::mg_alone().arg("--json").args(&all))
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn textures_convert_between_png_tga_and_both_dds_headers() {
    let dir = mg_testkit::scratch_dir("mg-texture-convert");
    let (cut, solid) = (dir.join("cut.png"), dir.join("solid.png"));
    png(&cut, 0);
    png(&solid, 255);
    // Alpha gives DXT5, none DXT1; every level down to 1×1.
    let (ok, json) = convert(&[s(&cut), s(&solid), "--to", s(&dir.join("bio"))]);
    assert!(ok, "{json}");
    let files = json["files"].as_array().unwrap();
    assert_eq!(
        (files[0]["format"].as_str(), files[1]["format"].as_str()),
        (Some("dxt5"), Some("dxt1"))
    );
    assert_eq!(files[0]["container"], "bioware");
    assert_eq!((files[0]["mips"].as_u64(), files[0]["has_alpha"].as_bool()), (Some(4), Some(true)));
    assert_eq!(files[1]["has_alpha"], false);
    assert_eq!(files[0]["source"]["type"], "png");
    let bio = std::fs::read(dir.join("bio/cut.dds")).unwrap();
    assert_eq!(&bio[..12], &[8, 0, 0, 0, 8, 0, 0, 0, 4, 0, 0, 0], "BioWare's header");
    // The standard header, and a TGA.
    let (ok, json) = convert(&[s(&cut), "--to", s(&dir.join("std")), "--standard"]);
    assert!(ok && json["files"][0]["container"] == "standard", "{json}");
    assert!(std::fs::read(dir.join("std/cut.dds")).unwrap().starts_with(b"DDS "));
    let (ok, json) = convert(&[s(&cut), "-o", s(&dir.join("cut.tga"))]);
    assert!(ok && json["files"][0]["format"] == "rgba8", "{json}");
    // Each back to a PNG: the same way up (red on top), alpha kept; the
    // TGA exactly, the DDS within DXT's error (blocks of one color).
    let source = read_png(&cut);
    for (from, exact) in [("bio/cut.dds", false), ("std/cut.dds", false), ("cut.tga", true)] {
        let back = dir.join(format!("back-{}.png", from.replace('/', "-")));
        let (ok, json) = convert(&[s(&dir.join(from)), "-o", s(&back)]);
        assert!(ok, "{from}: {json}");
        let got = read_png(&back);
        assert_eq!((got.width, got.height), (8, 8));
        if exact {
            assert_eq!(got, source, "{from}");
        }
        for (i, (p, q)) in
            source.data.as_chunks::<4>().0.iter().zip(got.data.as_chunks::<4>().0).enumerate()
        {
            for k in 0..4 {
                assert!(
                    (i32::from(p[k]) - i32::from(q[k])).abs() <= 8,
                    "{from} texel {i}: {p:?} {q:?}"
                );
            }
        }
    }
    // The game's row order: a TGA's first row is the picture's bottom.
    let tga = mg_image::tga::read(&std::fs::read(dir.join("cut.tga")).unwrap()).unwrap();
    assert_eq!(tga.stored_rgba().pixel(0, 0), [0, 0, 255, 255], "bottom row blue");
}

#[test]
fn a_dds_kept_in_its_format_is_copied_and_its_missing_levels_made() {
    let dir = mg_testkit::scratch_dir("mg-texture-copy");
    let cut = dir.join("cut.png");
    png(&cut, 0);
    let one = dir.join("one.dds");
    let out = crate::mg_alone()
        .args(["texture", "convert", s(&cut), "-o", s(&one), "--no-mipmaps"])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("reads past"), "the warning");
    let level0 = mg_image::dds::read(&std::fs::read(&one).unwrap()).unwrap();
    assert_eq!(level0.mips.len(), 1);
    // To the standard header: copied, the levels made below the same
    // largest level.
    let std = dir.join("std.dds");
    let (ok, json) = convert(&[s(&one), "-o", s(&std), "--standard"]);
    assert!(ok && json["files"][0]["reencoded"] == false, "{json}");
    let t = mg_image::dds::read(&std::fs::read(&std).unwrap()).unwrap();
    assert_eq!(t.mips.len(), 4);
    assert_eq!(t.mips[0], level0.mips[0]);
    // And back: BioWare's again, byte for byte with a whole chain.
    let (bio, again) = (dir.join("bio.dds"), dir.join("again.dds"));
    assert!(convert(&[s(&std), "-o", s(&bio)]).0);
    assert!(convert(&[s(&bio), "-o", s(&again)]).0);
    assert_eq!(std::fs::read(&bio).unwrap(), std::fs::read(&again).unwrap());
    // Another format asked: encoded again.
    let (ok, json) = convert(&[s(&bio), "-o", s(&dir.join("dxt1.dds")), "--format", "dxt1"]);
    assert!(
        ok && json["files"][0]["reencoded"] == true && json["files"][0]["format"] == "dxt1",
        "{json}"
    );
}

#[test]
fn textures_are_resized_bled_and_refused_before_anything_is_written() {
    let dir = mg_testkit::scratch_dir("mg-texture-adjust");
    let cut = dir.join("cut.png");
    png(&cut, 0);
    // A picture of 8×4: the left half seen, the right hidden, magenta.
    let mut hidden = Vec::new();
    for _ in 0..4 {
        for x in 0..8 {
            hidden.extend_from_slice(&if x < 4 { [10, 200, 30, 255] } else { [255, 0, 255, 0] });
        }
    }
    let src = dir.join("hidden.png");
    std::fs::write(&src, mg_image::png::write_rgba(8, 4, &hidden).unwrap()).unwrap();
    // Sizes: the larger side within --max-size, powers of two by ratio.
    let size = |args: &[&str]| {
        let mut all = vec![s(&src), "-o"];
        let out = dir.join("sized.png");
        all.push(s(&out));
        all.push("--force");
        all.extend(args);
        let (ok, json) = convert(&all);
        assert!(ok, "{json}");
        (json["files"][0]["width"].as_u64().unwrap(), json["files"][0]["height"].as_u64().unwrap())
    };
    assert_eq!(size(&["--max-size", "4"]), (4, 2));
    assert_eq!(size(&["--max-size", "6"]), (6, 3));
    assert_eq!(size(&["--pow2", "--max-size", "2"]), (2, 1));
    assert_eq!(size(&[]), (8, 4));
    // Bleeding: the hidden half's color from the visible half's.
    let (plain, bled) = (dir.join("plain.tga"), dir.join("bled.tga"));
    let magenta_under = |p: &Path| {
        let px = mg_image::tga::read(&std::fs::read(p).unwrap()).unwrap().stored_rgba();
        px.pixel(7, 1)
    };
    assert!(convert(&[s(&src), "-o", s(&plain)]).0);
    assert!(convert(&[s(&src), "-o", s(&bled), "--bleed"]).0);
    assert_eq!(magenta_under(&plain), [255, 0, 255, 0]);
    assert_eq!(magenta_under(&bled), [10, 200, 30, 0]);
    // A file there already, without --force; two textures to one name;
    // -o with two.
    let (ok, json) = convert(&[s(&src), "-o", s(&plain)]);
    assert!(!ok && json["error"].as_str().unwrap().contains("--force"), "{json}");
    let other = dir.join("other");
    std::fs::create_dir_all(&other).unwrap();
    std::fs::copy(&src, other.join("hidden.png")).unwrap();
    let (ok, json) =
        convert(&[s(&src), s(&other.join("hidden.png")), "--to", s(&dir.join("both"))]);
    assert!(!ok && json["error"].as_str().unwrap().contains("one file"), "{json}");
    assert!(!dir.join("both").exists(), "nothing written");
    let (ok, json) = convert(&[s(&src), s(&cut), "-o", s(&dir.join("x.dds"))]);
    assert!(!ok && json["error"].as_str().unwrap().contains("--to"), "{json}");
    // One that can't be read fails the run, after the rest are written.
    let bad = dir.join("bad.tga");
    std::fs::write(&bad, b"not a texture").unwrap();
    let (ok, json) = convert(&[s(&bad), s(&cut), "--to", s(&dir.join("some"))]);
    assert!(!ok, "{json}");
    assert_eq!(json["failed"][0]["input"], s(&bad));
    assert!(dir.join("some/cut.dds").is_file());
}

#[test]
fn textures_are_converted_by_name_from_the_load_order() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-texture-names");
    let run = |args: &[&str]| crate::json_out(crate::mg_on(&root, None).arg("--json").args(args));
    // The game's own, as the game finds it (a DDS over a TGA): copied.
    let (ok, json) = run(&["texture", "convert", "c_wolf", "--to", s(&dir)]);
    assert!(ok, "{json}");
    let f = &json["files"][0];
    assert!(f["from"].as_str().unwrap().starts_with("key:"), "{json}");
    assert_eq!(
        (f["reencoded"].as_bool(), f["source"]["type"].as_str()),
        (Some(false), Some("dds"))
    );
    let cat = crate::mg_on(&root, None).args(["cat", "c_wolf.dds"]).output().unwrap();
    assert_eq!(std::fs::read(dir.join("c_wolf.dds")).unwrap(), cat.stdout);
    // One of a folder --hak gives, over the game's.
    let hak = dir.join("hak");
    std::fs::create_dir_all(&hak).unwrap();
    png(&dir.join("mine.png"), 0);
    let (ok, _) =
        run(&["texture", "convert", s(&dir.join("mine.png")), "-o", s(&hak.join("c_wolf.tga"))]);
    assert!(ok);
    let (ok, json) =
        run(&["texture", "convert", "c_wolf.tga", "-o", s(&dir.join("w.png")), "--hak", s(&hak)]);
    assert!(ok && json["files"][0]["from"] == format!("folder:{}", hak.display()), "{json}");
    // Facts by name: the game's DXT5, its alpha used between clear and
    // opaque, and a PLT where there is no DDS or TGA of the name.
    let (ok, json) = run(&["texture", "info", "c_azer", "pmh0_head001"]);
    assert!(ok, "{json}");
    let t = &json["textures"];
    assert_eq!((t[0]["format"].as_str(), t[0]["alpha"].as_str()), (Some("dxt5"), Some("blended")));
    assert!(t[0]["alpha_mean"].as_f64().is_some_and(|m| m > 0.5 && m < 0.7), "{json}");
    assert_eq!((t[1]["type"].as_str(), t[1]["layers"][0].as_str()), (Some("plt"), Some("skin")));
    // A game PLT by name, split and joined: the same bytes as the game's.
    let (ok, json) = run(&["texture", "plt-split", "pmh0_chest001", "--to", s(&dir)]);
    assert!(ok, "{json}");
    let back = dir.join("chest.plt");
    let (shades, layers) =
        (dir.join("pmh0_chest001.shade.png"), dir.join("pmh0_chest001.layers.png"));
    assert!(run(&["texture", "plt-join", s(&shades), s(&layers), "-o", s(&back)]).0);
    let cat = crate::mg_on(&root, None).args(["cat", "pmh0_chest001.plt"]).output().unwrap();
    assert_eq!(std::fs::read(&back).unwrap(), cat.stdout);
    let (ok, json) = run(&["texture", "convert", "mg_zz_nowhere", "--to", s(&dir)]);
    assert!(
        !ok && json["failed"][0]["error"].as_str().unwrap().contains("found nowhere"),
        "{json}"
    );
}

#[test]
fn info_gives_each_texture_s_size_format_levels_and_alpha() {
    let dir = mg_testkit::scratch_dir("mg-texture-info");
    let (cut, solid) = (dir.join("cut.png"), dir.join("solid.png"));
    png(&cut, 0);
    png(&solid, 255);
    for (from, to, extra) in [
        (&cut, "cut.dds", None),
        (&cut, "cut_std.dds", Some("--standard")),
        (&solid, "solid.tga", None),
    ] {
        let mut args = vec![s(from), "-o"];
        let out = dir.join(to);
        args.push(s(&out));
        args.extend(extra);
        assert!(convert(&args).0);
    }
    // A PLT of two layers (skin and cloth 1), by hand.
    let mut plt = b"PLT V1  ".to_vec();
    for v in [10u32, 0, 2, 1] {
        plt.extend_from_slice(&v.to_le_bytes());
    }
    plt.extend_from_slice(&[100, 0, 200, 4]);
    std::fs::write(dir.join("part.plt"), plt).unwrap();
    std::fs::write(dir.join("bad.dds"), b"DDS not one").unwrap();
    let names = ["cut.png", "cut.dds", "cut_std.dds", "solid.tga", "part.plt", "bad.dds"];
    let paths: Vec<String> = names.iter().map(|n| s(&dir.join(n)).to_string()).collect();
    let mut args = vec!["--json", "texture", "info"];
    args.extend(paths.iter().map(String::as_str));
    let (ok, json) = crate::json_out(crate::mg_alone().args(&args));
    assert!(!ok, "the bad one fails the run: {json}");
    let t = json["textures"].as_array().unwrap();
    assert_eq!(t.len(), 5, "{json}");
    let fact = |i: usize, k: &str| t[i][k].clone();
    assert_eq!(
        (fact(0, "type"), fact(0, "format"), fact(0, "has_alpha")),
        ("png".into(), "rgba8".into(), true.into())
    );
    assert_eq!(fact(0, "alpha"), "binary");
    assert_eq!(
        (fact(1, "container"), fact(1, "format"), fact(1, "mips")),
        ("bioware".into(), "dxt5".into(), 4.into())
    );
    assert_eq!(fact(1, "alpha_mean"), 0.5);
    assert_eq!((fact(2, "container"), fact(2, "format")), ("standard".into(), "dxt5".into()));
    assert_eq!(
        (fact(3, "format"), fact(3, "has_alpha"), fact(3, "alpha")),
        ("rgb8".into(), false.into(), "opaque".into())
    );
    assert_eq!((fact(3, "width"), fact(3, "height")), (8.into(), 8.into()));
    assert_eq!(fact(4, "layers"), serde_json::json!(["skin", "cloth1"]));
    assert_eq!((fact(4, "width"), fact(4, "height")), (2.into(), 1.into()));
    assert_eq!(json["failed"][0]["input"], paths[5]);
    // Headers only: the texels' alpha not looked at.
    let (ok, json) = crate::json_out(crate::mg_alone().args([
        "--json",
        "texture",
        "info",
        "--headers",
        &paths[0],
        &paths[1],
    ]));
    assert!(ok, "{json}");
    assert!(json["textures"].as_array().unwrap().iter().all(|t| t["alpha"].is_null()), "{json}");
    assert_eq!(json["textures"][0]["has_alpha"], true);
}

/// A PLT of 16×20: every shade and every layer, its header word 6.
fn plt_file() -> Vec<u8> {
    let mut f = b"PLT V1  ".to_vec();
    for v in [6u32, 0, 16, 20] {
        f.extend_from_slice(&v.to_le_bytes());
    }
    for i in 0..16 * 20u32 {
        f.extend_from_slice(&[(i % 256) as u8, (i / 32 % 10) as u8]);
    }
    f
}

#[test]
fn a_plt_split_into_pngs_joins_back_to_the_same_bytes() {
    let dir = mg_testkit::scratch_dir("mg-texture-plt");
    let plt = dir.join("part.plt");
    std::fs::write(&plt, plt_file()).unwrap();
    let (ok, json) = crate::json_out(crate::mg_alone().args([
        "--json",
        "texture",
        "plt-split",
        s(&plt),
        "--to",
        s(&dir.join("out")),
    ]));
    assert!(ok, "{json}");
    let f = &json["files"][0];
    assert_eq!(f["layers_used"].as_array().unwrap().len(), 10);
    let (shades, layers) = (dir.join("out/part.shade.png"), dir.join("out/part.layers.png"));
    // Shown the right way up: the PNG's top row is the file's last.
    let pic = read_png(&layers);
    assert_eq!((pic.width, pic.height), (16, 20));
    assert_eq!(pic.pixel(0, 0), [9, 9, 9, 255], "the top row: the last stored, layer 9");
    assert_eq!(
        read_png(&shades).pixel(0, 19),
        [0, 0, 0, 255],
        "the first stored texel at the bottom"
    );
    let back = dir.join("back.plt");
    let (ok, json) = crate::json_out(crate::mg_alone().args([
        "--json",
        "texture",
        "plt-join",
        s(&shades),
        s(&layers),
        "-o",
        s(&back),
    ]));
    assert!(ok && json["width"] == 16, "{json}");
    assert_eq!(std::fs::read(&back).unwrap(), plt_file(), "the same bytes, header word too");
    // A texel of a color, a layer past 9, two sizes: refused.
    let mut colored = read_png(&layers);
    colored.data[0] = 200;
    let bad = dir.join("colored.png");
    std::fs::write(&bad, mg_image::png::write_rgba(16, 20, &colored.data).unwrap()).unwrap();
    let join = |l: &Path| {
        crate::json_out(crate::mg_alone().args([
            "--json",
            "texture",
            "plt-join",
            s(&shades),
            s(l),
            "-o",
            s(&dir.join("x.plt")),
        ]))
    };
    let (ok, json) = join(&bad);
    assert!(!ok && json["error"].as_str().unwrap().contains("not gray"), "{json}");
    let ten = dir.join("ten.png");
    std::fs::write(&ten, mg_image::png::write_gray(16, 20, &[10; 320], &[]).unwrap()).unwrap();
    let (ok, json) = join(&ten);
    assert!(!ok && json["error"].as_str().unwrap().contains("0 to 9"), "{json}");
    let small = dir.join("small.png");
    std::fs::write(&small, mg_image::png::write_gray(2, 2, &[0; 4], &[]).unwrap()).unwrap();
    let (ok, json) = join(&small);
    assert!(!ok && json["error"].as_str().unwrap().contains("one size"), "{json}");
    assert!(!dir.join("x.plt").exists());
}
