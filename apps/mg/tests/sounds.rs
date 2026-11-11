//! A builder's custom music and sounds, as `mg verify` judges them by the
//! role each plays in (`sound-silent`: an area's music and ambient sounds
//! the game plays nothing of; a placed sound kept only as `.bmu`, a missing
//! reference that says so), and `mg ls` lists a hak holding them.

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
    wave(1, 2, 44100 * 4, 4, 16, &data)
}

/// A RIFF WAVE of format `tag` (1: PCM, 0x55: MP3) holding `data`.
fn wave(
    tag: u16,
    channels: u16,
    bytes_per_second: u32,
    align: u16,
    bits: u16,
    data: &[u8],
) -> Vec<u8> {
    let mut out = b"RIFF".to_vec();
    out.extend((36 + data.len() as u32).to_le_bytes());
    out.extend(b"WAVEfmt ");
    out.extend(16u32.to_le_bytes());
    for n in [tag, channels] {
        out.extend(n.to_le_bytes());
    }
    for n in [44100u32, bytes_per_second] {
        out.extend(n.to_le_bytes());
    }
    for n in [align, bits] {
        out.extend(n.to_le_bytes());
    }
    out.extend(b"data");
    out.extend((data.len() as u32).to_le_bytes());
    out.extend(data);
    out
}

/// Silent MP3 (MPEG-1 layer III frames of 417 bytes, mono, 44.1 kHz): no
/// encoder needed.
fn mp3() -> Vec<u8> {
    let mut frame = vec![0xFF, 0xFB, 0x90, 0xC4];
    frame.resize(417, 0);
    frame.repeat(8)
}

/// MP3 behind the `BMU V1.0` header, as the game ships it.
fn bmu_mp3() -> Vec<u8> {
    [b"BMU V1.0".as_slice(), &mp3()].concat()
}

/// MP3 inside a RIFF WAVE (`fmt ` tag 0x55): a `.wav` some tools write,
/// renamed `.bmu`.
fn mp3_in_wave() -> Vec<u8> {
    wave(0x55, 1, 16000, 1, 0, &mp3())
}

fn verify_with(
    root: &std::path::Path,
    module: &std::path::Path,
    more: &[&str],
) -> serde_json::Value {
    let out = crate::mg_on(root, None).args(["verify", "--json"]).args(more).arg(module).output();
    let out = out.unwrap();
    // (Warnings all: the exit is a success.)
    assert!(out.status.success(), "{}{}", String::from_utf8_lossy(&out.stdout), {
        String::from_utf8_lossy(&out.stderr)
    });
    serde_json::from_slice(&out.stdout).unwrap()
}

fn verify(root: &std::path::Path, module: &std::path::Path) -> serde_json::Value {
    verify_with(root, module, &[])
}

/// The game's table `name` with rows added whose `Resource` are
/// `resources`, as 2DA text; and the first added row.
fn extended(game: &ResMan, name: &str, resources: &[&str]) -> (Vec<u8>, usize) {
    let table = game.get(&ResKey::parse(name, ResType::TWODA).unwrap()).unwrap();
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
    for (i, resource) in resources.iter().enumerate() {
        let mut cells: Vec<String> = (0..columns).map(|c| cell(1, c)).collect();
        cells[t.column("Resource").unwrap()] = resource.to_string();
        text += &format!("{} {}\n", t.len() + i, cells.join(" "));
    }
    (text.into_bytes(), t.len())
}

/// Neverwinter Chess with music and ambient rows of its own (the hak's
/// tables the module carries), its one area's five fields set to them
/// (`fields`: a field and which of `music` or `ambient` it names), the
/// files `files`, and a sound blueprint and a placed sound naming
/// `mg_snd` (a .wav) and `mg_snd_b` (only a .bmu). Saved in `dir`.
fn chess_with(
    root: &std::path::Path,
    dir: &std::path::Path,
    music: &[&str],
    ambient: &[&str],
    fields: &[(&str, usize)],
    files: &[(&str, ResType, Vec<u8>)],
) -> std::path::PathBuf {
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let mut m = Module::open(&chess).unwrap();
    let game = ResMan::for_game(&GameInstall::new(root, None, "en")).unwrap();
    let (music_table, first_music) = extended(&game, "ambientmusic", music);
    let (ambient_table, first_ambient) = extended(&game, "ambientsound", ambient);
    m.set(ResKey::parse("ambientmusic", ResType::TWODA).unwrap(), music_table);
    m.set(ResKey::parse("ambientsound", ResType::TWODA).unwrap(), ambient_table);
    for (name, t, data) in files {
        m.set(ResKey::new(ResRef::from_str(name).unwrap(), *t), data.clone());
    }
    m.set(ResKey::parse("mg_snd", ResType::WAV).unwrap(), pcm_wave());
    m.set(ResKey::parse("mg_snd_b", ResType::BMU).unwrap(), bmu_mp3());
    let mut uts = Gff::new(*b"UTS ");
    let sounds = |names: &[&str]| {
        let list = names.iter().map(|name| {
            let mut s = Struct::new(0);
            s.set("Sound", Value::resref(ResRef::from_str(name).unwrap()));
            s
        });
        Value::List(list.collect())
    };
    uts.root.set("Sounds", sounds(&["mg_snd", "mg_snd_b"]));
    uts.root.set("Tag", Value::String(b"MG_SND".to_vec()));
    uts.root.set("TemplateResRef", Value::resref(ResRef::from_str("mg_snd_bp").unwrap()));
    m.set_gff(ResKey::parse("mg_snd_bp", ResType::UTS).unwrap(), &uts).unwrap();
    let git = ResKey::new(ResRef::from_str("chess").unwrap(), ResType::GIT);
    let mut g = m.gff(&git).unwrap().unwrap();
    let mut placed = Struct::new(6);
    placed.set("Sounds", sounds(&["mg_snd"]));
    placed.set("TemplateResRef", Value::resref(ResRef::from_str("mg_snd_bp").unwrap()));
    for (label, v) in [("XPosition", 5.0), ("YPosition", 5.0), ("ZPosition", 0.0)] {
        placed.set(label, Value::Float(v));
    }
    let mut list = vec![placed];
    // A positional sound of the files' mg_wind and mg_bare, where they
    // are given.
    if files.iter().any(|f| f.0 == "mg_wind") {
        let mut wind = Struct::new(6);
        wind.set("Sounds", sounds(&["mg_wind", "mg_bare"]));
        wind.set("Positional", Value::Byte(1));
        for (label, v) in [("XPosition", 9.0), ("YPosition", 9.0), ("ZPosition", 0.0)] {
            wind.set(label, Value::Float(v));
        }
        list.push(wind);
    }
    g.root.set("SoundList", Value::List(list));
    let props = g.root.child_mut("AreaProperties").unwrap();
    for (field, i) in fields {
        let row = if field.starts_with("Music") { first_music } else { first_ambient } + i;
        props.set(field, Value::Int(row as i32));
    }
    m.set_gff(git, &g).unwrap();
    let path = dir.join("sounds.mod");
    m.save_as(&ModuleLocation::Archive(path.clone())).unwrap();
    path
}

/// The module's `sound-silent` findings: (at, message), each a warning of
/// the module's chess.git.
fn silent(json: &serde_json::Value) -> Vec<(String, String)> {
    let findings = json["findings"].as_array().unwrap();
    let ours = findings.iter().filter(|f| f["check"] == "sound-silent");
    ours.map(|f| {
        let place = serde_json::json!([f["severity"], f["source"], f["resource"]]);
        assert_eq!(place, serde_json::json!(["warning", "module", "chess.git"]), "{f}");
        (f["at"].as_str().unwrap().to_string(), f["message"].as_str().unwrap().to_string())
    })
    .collect()
}

/// The rows of the five fields: two music tracks and a battle music of
/// `ambientmusic.2da`, two ambient sounds of `ambientsound.2da`.
fn at(field: &str, table: &str, row: usize) -> String {
    format!("AreaProperties/{field}, {table}.2da row {row}")
}

#[test]
fn verify_says_which_music_and_sounds_the_game_plays_nothing_of() {
    let root = mg_testkit::corpus!();
    let chess = root.join("data/mod/Neverwinter Chess.mod");
    let dir = mg_testkit::scratch_dir("mg-verify-silent-sounds");
    let before = verify(&root, &chess);
    let path = chess_with(
        &root,
        &dir,
        &["mg_in_wave", "mg_pcm", "mg_only_wav"],
        &["mg_amb_mp3", "mg_amb_pcm"],
        &[
            ("MusicDay", 0),
            ("MusicNight", 1),
            ("MusicBattle", 2),
            ("AmbientSndDay", 0),
            ("AmbientSndNight", 1),
        ],
        &[
            // MP3 in a WAVE, as .bmu: music the game plays.
            ("mg_in_wave", ResType::BMU, mp3_in_wave()),
            // A PCM WAVE as .bmu: music the game is silent on.
            ("mg_pcm", ResType::BMU, pcm_wave()),
            // Music only as .wav: never looked for.
            ("mg_only_wav", ResType::WAV, bmu_mp3()),
            // MP3 under .wav, as an ambient sound: silent.
            ("mg_amb_mp3", ResType::WAV, bmu_mp3()),
            // A PCM WAVE as .wav, as an ambient sound: it plays.
            ("mg_amb_pcm", ResType::WAV, pcm_wave()),
        ],
    );
    let game = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let rows = |name: &str| {
        let t = game.get(&ResKey::parse(name, ResType::TWODA).unwrap()).unwrap();
        mg_2da::TwoDa::parse(&t, mg_core::Codepage::default()).unwrap().len()
    };
    let (music, ambient) = (rows("ambientmusic"), rows("ambientsound"));

    let after = verify(&root, &path);
    assert_eq!(
        silent(&after),
        [
            (
                at("MusicNight", "ambientmusic", music + 1),
                "Music mg_pcm.bmu is a WAVE, which the game does not play as music (it must hold \
                 MP3)"
                    .to_string()
            ),
            (
                at("MusicBattle", "ambientmusic", music + 2),
                "Battle music mg_only_wav exists only as mg_only_wav.wav, which the game does not \
                 look for (music must be a .bmu holding MP3)"
                    .into()
            ),
            (
                at("AmbientSndDay", "ambientsound", ambient),
                "Ambient sound mg_amb_mp3.wav holds MP3, which the game does not play under .wav \
                 (it plays a WAVE there, or MP3 as mg_amb_mp3.bmu)"
                    .into()
            ),
        ],
        "{after:#}"
    );
    // Nothing else changed but the sound kept only as .bmu, which is
    // missing, and says it is there as a .bmu: warnings, the errors and the
    // exit as they were.
    assert_eq!(before["errors"], after["errors"], "{after}");
    let others = |j: &serde_json::Value| -> Vec<serde_json::Value> {
        let all = j["findings"].as_array().unwrap();
        all.iter().filter(|f| f["check"] != "sound-silent").cloned().collect()
    };
    assert_eq!(others(&before), others(&after));
    let missing = after["missing"].as_array().unwrap();
    let ours: Vec<&serde_json::Value> =
        missing.iter().filter(|x| x["target"].as_str().unwrap().starts_with("mg_")).collect();
    assert_eq!(ours.len(), 1, "{ours:#?}");
    let m = ours[0];
    let what = serde_json::json!([m["target"], m["kind"], m["from"], m["found_as"], m["severity"]]);
    assert_eq!(what, serde_json::json!(["mg_snd_b", "sound", "mg_snd_bp.uts", "bmu", "warning"]));
    assert_eq!(
        after["missing"].as_array().unwrap().len(),
        before["missing"].as_array().unwrap().len() + 1
    );
    let warnings = |j: &serde_json::Value| j["warnings"].as_u64().unwrap();
    assert_eq!(warnings(&after), warnings(&before) + 4, "{after}");
    // A finding's fields, and a missing reference's.
    let f = after["findings"].as_array().unwrap().iter().find(|f| f["check"] == "sound-silent");
    let mut keys: Vec<&str> = f.unwrap().as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(keys, ["at", "check", "fix", "message", "resource", "severity", "source"]);
    assert_eq!(f.unwrap()["fix"], serde_json::Value::Null);
    for x in missing {
        assert!(x["found_as"].is_null() || x["found_as"] == "bmu", "{x}");
    }

    // As text: a line a finding, the reference's saying what is there.
    let out = crate::mg_on(&root, None).arg("verify").arg(&path).output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let line = format!(
        "warning\tmodule › chess.git {}: Music mg_pcm.bmu is a WAVE, which the game does not \
         play as music (it must hold MP3)",
        at("MusicNight", "ambientmusic", music + 1)
    );
    assert!(text.lines().any(|l| l == line), "{text}");
    let gone = "sound mg_snd_b missing (only mg_snd_b.bmu is there, which the game does not look \
                for)";
    assert!(
        text.lines().any(|l| l.starts_with("warning\tSounds\tmg_snd_bp.uts") && l.ends_with(gone)),
        "{text}"
    );

    // Left out, it isn't checked: nothing of it, and the JSON says so.
    let skipped = verify_with(&root, &path, &["--skip", "sound-silent"]);
    assert_eq!(silent(&skipped), []);
    assert_eq!(skipped["skipped"], serde_json::json!({ "sound-silent": null }));
    let skipped = verify_with(&root, &path, &["--skip", "objects"]);
    assert_eq!(silent(&skipped), []);
}

#[test]
fn verify_says_a_track_found_nowhere_and_nothing_of_music_the_game_plays() {
    let root = mg_testkit::corpus!();
    let dir = mg_testkit::scratch_dir("mg-verify-missing-track");
    let path = chess_with(
        &root,
        &dir,
        &["mg_gone", "mg_in_wave", "mg_bmu"],
        &["mg_amb_bmu", "mg_amb_gone"],
        &[
            ("MusicDay", 0),
            ("MusicNight", 1),
            ("MusicBattle", 2),
            ("AmbientSndDay", 0),
            ("AmbientSndNight", 1),
        ],
        &[
            ("mg_in_wave", ResType::BMU, mp3_in_wave()),
            ("mg_bmu", ResType::BMU, bmu_mp3()),
            // An ambient sound only as .bmu, holding MP3: it plays.
            ("mg_amb_bmu", ResType::BMU, bmu_mp3()),
            // A positional sound's stereo WAVE, and its MP3 without the
            // header: silent.
            ("mg_wind", ResType::WAV, pcm_wave()),
            ("mg_bare", ResType::WAV, mp3()),
        ],
    );
    let game = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let rows = |name: &str| {
        let t = game.get(&ResKey::parse(name, ResType::TWODA).unwrap()).unwrap();
        mg_2da::TwoDa::parse(&t, mg_core::Codepage::default()).unwrap().len()
    };
    let after = verify(&root, &path);
    assert_eq!(
        silent(&after),
        [
            (
                "SoundList[1], Sounds[0]".to_string(),
                "Sound mg_wind.wav is stereo, which the game does not play from a positional sound \
                 (make it mono, or the sound area-wide)"
                    .to_string()
            ),
            (
                "SoundList[1], Sounds[1]".to_string(),
                "Sound mg_bare.wav is MP3 without the BMU V1.0 header, which the game does not \
                 play (it plays a WAVE there, or MP3 behind the header)"
                    .to_string()
            ),
            (
                at("MusicDay", "ambientmusic", rows("ambientmusic")),
                "Music mg_gone is found nowhere (the game looks for mg_gone.bmu)".to_string()
            ),
            (
                at("AmbientSndNight", "ambientsound", rows("ambientsound") + 1),
                "Ambient sound mg_amb_gone is found nowhere (the game looks for mg_amb_gone.wav, \
                 then mg_amb_gone.bmu)"
                    .into()
            ),
        ],
        "{after:#}"
    );
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
