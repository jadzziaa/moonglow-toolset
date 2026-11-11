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
