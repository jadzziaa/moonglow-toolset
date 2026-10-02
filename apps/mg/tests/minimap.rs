//! `mg minimap`: Neverwinter Chess's board as a PNG, a tile's pictures'
//! size or the size asked for, a tile.

use std::process::Command;

/// A PNG's width and height, from its header.
fn size(png: &[u8]) -> (u32, u32) {
    assert_eq!(&png[1..4], b"PNG");
    let be = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
    (be(16), be(20))
}

#[test]
fn an_area_s_minimap_as_a_png() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-minimap");
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let out = dir.join("chess.png");
    let mg = |extra: &[&str]| {
        let o = Command::new(env!("CARGO_BIN_EXE_mg"))
            .args(["--root", root.to_str().unwrap(), "--no-user-dir", "minimap"])
            .arg(&chess)
            .args(["chess", out.to_str().unwrap()])
            .args(extra)
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        size(&std::fs::read(&out).unwrap())
    };
    let (w, h) = mg(&[]);
    assert!(w == h && w % 6 == 0 && w >= 6 * 16, "{w}×{h}");
    assert_eq!(mg(&["--size", "8"]), (48, 48));
}
