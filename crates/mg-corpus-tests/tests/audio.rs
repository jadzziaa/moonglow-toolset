//! The game's sounds and music decode (`mg_audio`), and agree with ffmpeg
//! (when installed) on their rate, channels and length.

use std::process::Command;

use mg_core::ResType;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_testkit::{corpus, sweep};
use rayon::prelude::*;

/// Every sound and music file of the game.
fn sounds(rm: &ResMan) -> Vec<ResKey> {
    [ResType::WAV, ResType::BMU]
        .into_iter()
        .flat_map(|t| rm.list(t).into_iter().map(move |r| ResKey::new(r, t)))
        .collect()
}

#[test]
fn every_game_sound_decodes() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let all = sounds(&rm);
    assert!(all.len() > 12_000, "{} sounds", all.len());
    let all = sweep(all, 10);
    // (kind, what went wrong) of each, on every core.
    let results: Vec<(String, Option<String>)> = all
        .par_iter()
        .map(|key| {
            let data = rm.get(key).unwrap();
            let kind = format!("{:?}", mg_audio::container(&data).map(|c| c.0));
            let failure = match mg_audio::decode(&data) {
                Ok(pcm) if pcm.frames() > 0 => None,
                Ok(_) => Some(format!("{key}: empty")),
                Err(e) => Some(format!("{key}: {e}")),
            };
            (kind, failure)
        })
        .collect();
    let mut kinds = std::collections::BTreeMap::new();
    for (kind, _) in &results {
        *kinds.entry(kind.as_str()).or_insert(0) += 1;
    }
    let failed: Vec<String> = results.iter().filter_map(|r| r.1.clone()).collect();
    eprintln!("{kinds:?}");
    assert!(failed.is_empty(), "{} of {} failed:\n{}", failed.len(), all.len(), failed.join("\n"));
}

/// Every row of the game's own `ambientmusic.2da`, as music, and of its
/// `ambientsound.2da`, as an ambient sound, names a file the game plays
/// there by the role rules (`mg_audio::role`) that Verify's `sound-silent`
/// holds a module's areas to (`mg_module::sounds`), but one: row 200 of
/// `ambientsound.2da`, `al_mus_throne2.wav` (`BMU V1.0` + stereo MP3 under
/// `.wav`), which the client plays nothing of (read on 2026-11-11,
/// `client_audio` with `MG_AUDIO_FORMS=stock`: the stock row by its number,
/// a new row naming the game's file and its bytes under a test name, all
/// silent, a PCM control playing before and after): the shipped row is
/// dead in the game. Another finding means a reading of the client is
/// wrong or incomplete; row 200 no longer flagged, that the rules changed.
/// (Every row, every day: 253 rows take some 30 ms, so the sweep takes
/// them all rather than a sample.) The shipped modules' areas are held to
/// the check in `doctor.rs`, `shipped_modules_have_only_known_findings`.
#[test]
fn the_game_s_own_music_and_ambient_rows_are_heard() {
    use mg_audio::Role;
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let mut rows = Vec::new();
    for (table, role) in [("ambientmusic", Role::Music), ("ambientsound", Role::Ambient)] {
        let data = rm.get(&ResKey::parse(table, ResType::TWODA).unwrap()).unwrap();
        let t = mg_2da::TwoDa::parse(&data, mg_core::Codepage::default()).unwrap();
        for row in 0..t.len() {
            let Some(cell) = t.get(row, "Resource").map(str::trim).filter(|c| !c.is_empty()) else {
                continue;
            };
            let name = mg_core::ResRef::from_str(cell).unwrap();
            rows.push((table, row, role, name));
        }
    }
    let all = rows.len();
    assert!(all > 250, "{all} rows name a sound");
    // (Every one of them: a sweep of one in one.)
    let picked = sweep(rows, 1);
    assert_eq!(picked.len(), all);
    let judged = mg_module::sounds::Judged::default();
    let unheard: Vec<String> = picked
        .par_iter()
        .filter_map(|&(table, row, role, name)| {
            let read = |k: &ResKey| judged.form(&rm, k);
            let u = mg_module::sounds::unheard_by(&rm, role, name, false, &read)?;
            Some(format!("{table}.2da row {row}: {}", u.message(role, name)))
        })
        .collect();
    eprintln!("{all} rows judged");
    assert_eq!(
        unheard,
        ["ambientsound.2da row 200: Ambient sound al_mus_throne2.wav holds MP3, which the game \
             does not play under .wav (it plays a WAVE there, or MP3 as al_mus_throne2.bmu)"],
        "the game's rows the rules say are silent"
    );
}

/// Every sound of the game's own sound blueprints (each `.uts` of the
/// install: its `Sounds`, judged as a placed sound's, positional where the
/// blueprint is), by the rules Verify's `sound-silent` holds a module's
/// placed sounds and blueprints to (a `.wav` holding a WAVE or MP3 behind
/// the `BMU V1.0` header, mono where the sound is positional), is a file the
/// game plays there but two: `lavalakeloop.uts` and `sludgelake.uts`,
/// positional, of the stereo IMA ADPCM `al_na_lavalake1.wav` and
/// `al_na_sludglake1.wav`. Both are silent as shipped and play made
/// area-wide (read in the client on 2026-11-11, `client_audio` with
/// `MG_AUDIO_FORMS=stereo`: every field of each copied into a placed
/// object). Another finding means a reading of the client is wrong or
/// incomplete; these two no longer flagged, that the rules changed. (Every
/// blueprint, every day: the 301 take some 10 ms alone, so the sweep takes them
/// all. Sounds the game's blueprints name and the game hasn't are a missing
/// reference's to say, and counted.)
#[test]
fn the_game_s_own_sound_blueprints_name_only_two_sounds_it_does_not_play() {
    use mg_audio::Role;
    use mg_module::sounds::{Unheard, unheard_by};
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let all: Vec<ResKey> =
        rm.list(ResType::UTS).into_iter().map(|r| ResKey::new(r, ResType::UTS)).collect();
    let blueprints = all.len();
    assert!(blueprints > 250, "{blueprints} sound blueprints");
    // (Every one of them: a sweep of one in one.)
    let picked = sweep(all, 1);
    assert_eq!(picked.len(), blueprints);
    let started = std::time::Instant::now();
    let judged = mg_module::sounds::Judged::default();
    // (blueprint, positional, sound, what is wrong) of each sound.
    let found: Vec<(ResKey, bool, mg_core::ResRef, Option<Unheard>)> = picked
        .par_iter()
        .flat_map_iter(|key| {
            let gff = mg_gff::Gff::read(&rm.get(key).unwrap()).unwrap();
            let positional = gff.root.integer("Positional").unwrap_or(0) != 0;
            let names: Vec<mg_core::ResRef> = (gff.root.list("Sounds").unwrap_or_default().iter())
                .filter_map(|e| e.resref("Sound").filter(|n| !n.is_empty()))
                .collect();
            let read = |k: &ResKey| judged.form(&rm, k);
            let told: Vec<_> = names
                .into_iter()
                .map(|n| (*key, positional, n, unheard_by(&rm, Role::Placed, n, positional, &read)))
                .collect();
            told
        })
        .collect();
    let sounds = found.len();
    let positional = found.iter().filter(|f| f.1).count();
    let missing = found.iter().filter(|f| matches!(f.3, Some(Unheard::Missing))).count();
    let wrong: Vec<String> = (found.iter())
        .filter(|f| f.3.is_some() && !matches!(f.3, Some(Unheard::Missing)))
        .map(|(k, p, n, u)| {
            let place = if *p { "positional" } else { "area-wide" };
            format!("{k} ({place}): {}", u.as_ref().unwrap().message(Role::Placed, *n))
        })
        .collect();
    eprintln!(
        "{blueprints} blueprints, {sounds} sounds ({positional} positional), {missing} found \
         nowhere, in {:?}",
        started.elapsed()
    );
    let mut wrong = wrong;
    wrong.sort();
    let stereo = |bp: &str, wav: &str| {
        format!(
            "{bp}.uts (positional): Sound {wav}.wav is stereo, which the game does not play from \
             a positional sound (make it mono, or the sound area-wide)"
        )
    };
    assert_eq!(
        wrong,
        [stereo("lavalakeloop", "al_na_lavalake1"), stereo("sludgelake", "al_na_sludglake1")],
        "the game's sounds the rules say are silent"
    );
}

/// What ffmpeg decodes: rate, channels and frames.
fn ffmpeg(data: &[u8], dir: &std::path::Path, name: &str) -> Option<(u32, u16, usize)> {
    let path = dir.join(name);
    std::fs::write(&path, data).ok()?;
    let probe = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "a:0", "-show_entries"])
        .args(["stream=sample_rate,channels", "-of", "csv=p=0"])
        .arg(&path)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&probe.stdout);
    let mut it = text.trim().split(',');
    let rate: u32 = it.next()?.parse().ok()?;
    let channels: u16 = it.next()?.parse().ok()?;
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&path)
        .args(["-f", "f32le", "-"])
        .output()
        .ok()?;
    Some((rate, channels, out.stdout.len() / 4 / usize::from(channels)))
}

#[test]
fn sounds_agree_with_ffmpeg() {
    let root = corpus!();
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("skipped: no ffmpeg");
        return;
    }
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let dir = mg_testkit::scratch_dir("audio-ffmpeg");
    let all = sounds(&rm);
    // Every 60th sound and every music track's first few: each container.
    let picked: Vec<(String, Vec<u8>)> = all
        .iter()
        .enumerate()
        .filter(|(i, k)| i % 60 == 0 || (k.restype == ResType::BMU && i % 10 == 0))
        .map(|(_, k)| (k.to_string(), rm.get(k).unwrap().into_owned()))
        .collect();
    // (A sample of the sample on an everyday run.)
    let picked = sweep(picked, 6);
    let mut wrong = Vec::new();
    for (name, data) in &picked {
        let Some((kind, range)) = mg_audio::container(data) else { continue };
        // ffmpeg is given what the game decodes: the MP3 after the header.
        let ext = if kind == mg_audio::Container::Mp3 { "mp3" } else { "wav" };
        let Some(theirs) = ffmpeg(&data[range], &dir, &format!("probe.{ext}")) else {
            wrong.push(format!("{name}: ffmpeg failed"));
            continue;
        };
        let ours = mg_audio::decode(data).unwrap();
        // MP3 encoder delay and padding may be trimmed differently: two
        // frames (2 × 1152 samples) of slack.
        let slack = if kind == mg_audio::Container::Mp3 { 2304 } else { 0 };
        if (ours.rate, ours.channels) != (theirs.0, theirs.1)
            || ours.frames().abs_diff(theirs.2) > slack
        {
            wrong.push(format!(
                "{name}: {}/{}/{} vs ffmpeg {theirs:?}",
                ours.rate,
                ours.channels,
                ours.frames()
            ));
        }
    }
    eprintln!("{} sounds compared", picked.len());
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
