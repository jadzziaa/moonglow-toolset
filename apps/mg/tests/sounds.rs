//! A builder's custom music and sounds, as a PCM WAVE kept under the name
//! `.bmu` (renamed so the game reads it) or `.wav`: `mg verify` finds
//! nothing to say of them (it reads no sound's bytes, and no music row), and
//! `mg ls` lists a hak holding them.

use mg_core::{ResRef, ResType};
use mg_gff::{Gff, Struct, Value};
use mg_module::{Module, ModuleLocation};
use mg_resman::{GameInstall, ResKey, ResMan};

/// A PCM WAVE: 16-bit stereo, 44.1 kHz, a tenth of a second of a tone.
fn pcm_wave() -> Vec<u8> {
    let mut data = Vec::new();
    for i in 0..4410u32 {
        let v = ((i as f32 / 44100.0 * 440.0 * std::f32::consts::TAU).sin() * 16000.0) as i16;
        data.extend(v.to_le_bytes());
        data.extend((-v).to_le_bytes());
    }
    let mut out = b"RIFF".to_vec();
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    for n in [1u16, 2] {
        out.extend(n.to_le_bytes());
    }
    for n in [44100u32, 44100 * 4] {
        out.extend(n.to_le_bytes());
    }
    for n in [4u16, 16] {
        out.extend(n.to_le_bytes());
    }
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

fn verify(root: &std::path::Path, module: &std::path::Path) -> serde_json::Value {
    let out = crate::mg_on(root, None).args(["verify", "--json"]).arg(module).output().unwrap();
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), {
        String::from_utf8_lossy(&out.stderr)
    });
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn verify_says_nothing_of_pcm_music_and_sounds_kept_as_bmu_or_wav() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-pcm-sounds");
    let before = verify(&root, &chess);

    let mut m = Module::open(&chess).unwrap();
    // ambientmusic.2da with a row naming mg_track, which the area plays.
    let game = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let table = game.get(&ResKey::parse("ambientmusic", ResType::TWODA).unwrap()).unwrap();
    let t = mg_2da::TwoDa::parse(&table, mg_core::Codepage::default()).unwrap();
    let cell = |r: usize, c: usize| {
        let cell = t.cell(r, c).unwrap_or("****");
        if cell.contains(' ') { format!("\"{cell}\"") } else { cell.to_string() }
    };
    let columns = t.columns().len();
    let mut text = format!("2DA V2.0\n\n   {}\n", t.columns().join(" "));
    for r in 0..t.len() {
        let cells: Vec<String> = (0..columns).map(|c| cell(r, c)).collect();
        text += &format!("{r} {}\n", cells.join(" "));
    }
    let mut cells: Vec<String> = (0..columns).map(|c| cell(1, c)).collect();
    cells[t.column("Resource").unwrap()] = "mg_track".into();
    text += &format!("{} {}\n", t.len(), cells.join(" "));
    m.set(ResKey::parse("ambientmusic", ResType::TWODA).unwrap(), text.into_bytes());
    // The track as .bmu, a sound as .wav, and a sound blueprint and a placed sound naming it.
    m.set(ResKey::parse("mg_track", ResType::BMU).unwrap(), pcm_wave());
    m.set(ResKey::parse("mg_snd", ResType::WAV).unwrap(), pcm_wave());
    let mut uts = Gff::new(*b"UTS ");
    let sound = |name: &str| {
        let mut s = Struct::new(0);
        s.set("Sound", Value::resref(ResRef::from_str(name).unwrap()));
        Value::List(vec![s])
    };
    uts.root.set("Sounds", sound("mg_snd"));
    uts.root.set("Tag", Value::String(b"MG_SND".to_vec()));
    uts.root.set("TemplateResRef", Value::resref(ResRef::from_str("mg_snd_bp").unwrap()));
    m.set_gff(ResKey::parse("mg_snd_bp", ResType::UTS).unwrap(), &uts).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut placed = Struct::new(6);
    placed.set("Sounds", sound("mg_snd"));
    placed.set("TemplateResRef", Value::resref(ResRef::from_str("mg_snd_bp").unwrap()));
    for (label, v) in [("XPosition", 5.0), ("YPosition", 5.0), ("ZPosition", 0.0)] {
        placed.set(label, Value::Float(v));
    }
    g.root.set("SoundList", Value::List(vec![placed]));
    let props = g.root.child_mut("AreaProperties").unwrap();
    props.set("MusicDay", Value::Int(t.len() as i32));
    props.set("MusicNight", Value::Int(t.len() as i32));
    m.set_gff(git, &g).unwrap();
    let path = dir.join("pcm-sounds.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();

    let after = verify(&root, &path);
    // Nothing more than the module had: the same findings, none of the sounds.
    assert_eq!(before["errors"], after["errors"], "{after}");
    assert_eq!(before["findings"], after["findings"], "{after}");
    // (Nor a name it could not find: `missing`.)
    assert_eq!(before["missing"], after["missing"], "{after}");
    assert_eq!(before["warnings"], after["warnings"], "{after}");
    let all = after.to_string();
    assert!(!all.contains("mg_track") && !all.contains("mg_snd"), "{all}");
}

#[test]
fn ls_lists_a_pcm_wave_kept_as_bmu_in_a_hak() {
    let dir = mg_testkit::scratch_dir("mg-ls-bmu");
    let wave = pcm_wave();
    let mut w = mg_erf::ErfWriter::new(*b"HAK ");
    for (name, restype) in [("mg_track", ResType::BMU), ("mg_snd", ResType::WAV)] {
        w.add(ResRef::from_str(name).unwrap(), restype, wave.clone()).unwrap();
    }
    let hak = dir.join("mg_sounds.hak");
    std::fs::write(&hak, w.to_bytes().unwrap()).unwrap();
    let (ok, json) = crate::mg_json(&["ls", hak.to_str().unwrap()]);
    assert!(ok, "{json}");
    let size = wave.len();
    assert_eq!(
        json["resources"],
        serde_json::json!([
            { "name": "mg_track.bmu", "size": size },
            { "name": "mg_snd.wav", "size": size }
        ])
    );
    let text = String::from_utf8(crate::mg_output(&["ls", hak.to_str().unwrap()]).stdout).unwrap();
    assert!(text.contains("mg_track.bmu") && text.contains("mg_snd.wav"), "{text}");
}
