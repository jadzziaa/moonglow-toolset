//! The game's sounds and music decode (`mg_audio`), and agree with ffmpeg
//! (when installed) on their rate, channels and length.

use std::process::Command;

use mg_core::ResType;
use mg_resman::{GameInstall, ResKey, ResMan};
use mg_testkit::corpus;

/// Every sound and music file of the game: (name, data).
fn sounds(rm: &ResMan) -> Vec<(String, Vec<u8>)> {
    [ResType::WAV, ResType::BMU]
        .into_iter()
        .flat_map(|t| rm.list(t).into_iter().map(move |r| ResKey::new(r, t)))
        .map(|k| (k.to_string(), rm.get(&k).unwrap().into_owned()))
        .collect()
}

#[test]
fn every_game_sound_decodes() {
    let root = corpus!();
    let rm = ResMan::for_game(&GameInstall::new(&root, None, "en")).unwrap();
    let all = sounds(&rm);
    assert!(all.len() > 12_000, "{} sounds", all.len());
    let mut kinds = std::collections::BTreeMap::new();
    let mut failed = Vec::new();
    for (name, data) in &all {
        let kind = mg_audio::container(data).map(|c| c.0);
        *kinds.entry(format!("{kind:?}")).or_insert(0) += 1;
        match mg_audio::decode(data) {
            Ok(pcm) if pcm.frames() > 0 => {}
            Ok(_) => failed.push(format!("{name}: empty")),
            Err(e) => failed.push(format!("{name}: {e}")),
        }
    }
    eprintln!("{kinds:?}");
    assert!(failed.is_empty(), "{} of {} failed:\n{}", failed.len(), all.len(), failed.join("\n"));
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
    let picked: Vec<&(String, Vec<u8>)> = all
        .iter()
        .enumerate()
        .filter(|(i, (n, _))| i % 60 == 0 || (n.ends_with(".bmu") && i % 10 == 0))
        .map(|(_, s)| s)
        .collect();
    let mut wrong = Vec::new();
    for (name, data) in &picked {
        let Some((start, kind)) = mg_audio::container(data).map(|c| (c.1, c.0)) else { continue };
        // ffmpeg is given what the game decodes: the MP3 after the header.
        let ext = if kind == mg_audio::Container::Mp3 { "mp3" } else { "wav" };
        let Some(theirs) = ffmpeg(&data[start..], &dir, &format!("probe.{ext}")) else {
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
