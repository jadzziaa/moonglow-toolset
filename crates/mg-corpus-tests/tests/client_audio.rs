//! Which music and sound files the game client plays: each form of file
//! (a WAV of each encoding, MP3 with and without the `BMU V1.0` header,
//! with tags or junk in front, MP3 inside a RIFF WAVE, WAVs whose header
//! sizes are wrong, a file under the other extension) played by the client,
//! and what it played captured, so that `mg_audio` takes what the game
//! takes.
//!
//! The client's OpenAL Soft writes everything it mixes to a WAV file
//! (`MG_CLIENT_AUDIO_OUT` of `tools/nwclient/run-client.sh`: the `wave`
//! backend, 44100 Hz, stereo, 16-bit, in real time). The files are in a
//! hak (`mg_audio.hak`, with `ambientmusic.2da` and `ambientsound.2da`, the
//! game's tables each with a row added for every form). The module's
//! OnClientEnter script steps through the forms: for each it sets the
//! area's day and night music (`MusicBackgroundChangeDay`, `...Night`,
//! `MusicBackgroundPlay`) or ambient sound (`AmbientSoundChangeDay`...) to
//! the form's row, logs `MG_BEGIN i`, and after [`PLAY`] seconds stops it
//! and logs `MG_END i`, a silent [`GAP`] between forms. The harness reads
//! the capture's length at the moment each marker appears in the log, so
//! that each form's stretch of the capture is known without clocks.
//!
//! The authored WAVs are each a pure tone of its own frequency, found in
//! the form's stretch with a Goertzel filter (so that "played" is never
//! other game audio, or an earlier form's); the MP3 forms carry a game
//! track's MP3 (read from the install at test time, never committed),
//! found by its level against the gaps' silence. A form whose file is
//! missing (a row with no file) is the silent control, the game's own
//! track the loud one. A form during which the client ends is reported as
//! such, and the forms after it are played in a new launch.
//!
//! `MG_AUDIO_FORMS` (names, separated by commas) plays only those forms
//! (the controls are always played). `MG_AUDIO_BY_AREA=1` plays the first
//! music form by the area's own `MusicDay` and `MusicNight` instead of by
//! script. `MG_AUDIO_FOLDER` (`override`, `music`, `ambient`) puts the
//! forms' files loose in that folder of the user directory instead of the
//! hak (the tables stay in the hak).
//!
//! Needs the game and the off-screen display; run by hand:
//! `DISPLAY=$D cargo test -p mg-corpus-tests --release --test client_audio -- --ignored --nocapture`.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use mg_core::{Codepage, ResRef, ResType};
use mg_erf::ErfWriter;
use mg_gff::Value;
use mg_module::ModuleLocation;
use mg_module::new::{AreaSpec, add_area, new_module};
use mg_resman::{GameInstall, ResKey};
use mg_rules::GameData;
use mg_schema::{StructExt, ifo};
use mg_testkit::{corpus, oracle_tool, scratch_dir};

mod common;
use common::{SETTINGS, compile, repo};

/// Seconds a form plays.
const PLAY: f64 = 9.0;
/// Seconds of silence after each form.
const GAP: f64 = 6.0;
/// Seconds after the area is entered before the first form.
const LEAD: f64 = 3.0;
/// The capture's rate and channels (pinned in run-client.sh's alsoft.conf).
const RATE: u32 = 44100;
/// Seconds of each authored tone (longer than [`PLAY`]: no loop inside it).
const TONE_SECONDS: f32 = 14.0;
/// The authored tones' amplitude.
const AMPLITUDE: f32 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Music,
    Ambient,
    /// A sound the PC plays (`PlaySound`), once.
    Sound,
}

/// A form of file the client is given.
struct Form {
    /// The form's name, and its files' resource name.
    name: &'static str,
    /// The resource its row or `PlaySound` names (the name, or a game
    /// sound's).
    res: String,
    kind: Kind,
    /// What the form is.
    what: String,
    /// The files under that name (none, with `res` the name: the silent
    /// control).
    files: Vec<(ResType, Vec<u8>)>,
    /// The tone's frequency, for the authored WAVs.
    tone: Option<f32>,
    /// Whether the client plays it, where that is settled (asserted).
    expect: Option<bool>,
    /// Played in every launch (the controls and checks); a launch in which
    /// one with an `expect` does otherwise is played again.
    control: bool,
}

impl Form {
    /// Whether nothing of its name exists (the silent controls).
    fn missing(&self) -> bool {
        self.files.is_empty() && self.res == self.name
    }
}

/// What was measured in a form's stretch of the capture.
#[derive(Debug, Clone)]
struct Measure {
    /// The level of the whole stretch, dBFS (RMS).
    rms: f64,
    /// The peak, dBFS.
    peak: f64,
    /// The level of the form's tone in it, dBFS (RMS of a sine).
    tone: Option<f64>,
    /// Seconds from `MG_BEGIN` to the first 20 ms over -60 dBFS.
    onset: Option<f64>,
    /// The level of the gap after it, dBFS.
    gap: f64,
}

/// How a launch went for a form.
#[derive(Debug, Clone)]
enum Outcome {
    Measured(Measure),
    /// The client ended (crashed) while it played.
    Ended,
    /// Not reached (the launch's time ran out).
    NotReached,
}

// ---------------------------------------------------------------- fixtures

/// A tone of `freq` Hz, interleaved over `channels`, in [-1, 1].
fn tone(freq: f32, rate: u32, channels: u16) -> Vec<f32> {
    tone_of(TONE_SECONDS, freq, rate, channels)
}

/// [`tone`] lasting `seconds`.
fn tone_of(seconds: f32, freq: f32, rate: u32, channels: u16) -> Vec<f32> {
    let n = (seconds * rate as f32) as usize;
    let mut out = Vec::with_capacity(n * usize::from(channels));
    for i in 0..n {
        let v = AMPLITUDE * (std::f32::consts::TAU * freq * i as f32 / rate as f32).sin();
        for _ in 0..channels {
            out.push(v);
        }
    }
    out
}

#[derive(Clone, Copy)]
enum Pcm {
    U8,
    S16,
    S24,
    F32,
}

/// A `fmt ` chunk's body for plain PCM or float.
fn fmt_pcm(enc: Pcm, rate: u32, channels: u16) -> Vec<u8> {
    let (tag, bits) = match enc {
        Pcm::U8 => (1u16, 8u16),
        Pcm::S16 => (1, 16),
        Pcm::S24 => (1, 24),
        Pcm::F32 => (3, 32),
    };
    let align = channels * bits / 8;
    let mut f = Vec::new();
    f.extend(tag.to_le_bytes());
    f.extend(channels.to_le_bytes());
    f.extend(rate.to_le_bytes());
    f.extend((rate * u32::from(align)).to_le_bytes());
    f.extend(align.to_le_bytes());
    f.extend(bits.to_le_bytes());
    f
}

fn pcm_bytes(enc: Pcm, samples: &[f32]) -> Vec<u8> {
    let mut d = Vec::new();
    for &s in samples {
        match enc {
            Pcm::U8 => d.push((s * 127.0 + 128.0).round() as u8),
            Pcm::S16 => d.extend(((s * 32767.0).round() as i16).to_le_bytes()),
            Pcm::S24 => d.extend(&((s * 8_388_607.0).round() as i32).to_le_bytes()[..3]),
            Pcm::F32 => d.extend(s.to_le_bytes()),
        }
    }
    d
}

/// The sizes a RIFF WAVE file declares, for the forms that get them wrong.
#[derive(Clone, Copy, Default)]
struct Sizes {
    riff: Option<u32>,
    data: Option<u32>,
}

/// A RIFF WAVE file of `chunks` (id, body) and then `data`.
fn riff(chunks: &[(&[u8; 4], &[u8])], data: &[u8], sizes: Sizes) -> Vec<u8> {
    let mut body = b"WAVE".to_vec();
    for (id, c) in chunks {
        body.extend(*id);
        body.extend((c.len() as u32).to_le_bytes());
        body.extend(*c);
        if c.len() % 2 == 1 {
            body.push(0);
        }
    }
    body.extend(b"data");
    body.extend(sizes.data.unwrap_or(data.len() as u32).to_le_bytes());
    body.extend(data);
    let mut out = b"RIFF".to_vec();
    out.extend(sizes.riff.unwrap_or(body.len() as u32).to_le_bytes());
    out.extend(body);
    out
}

/// A PCM WAV of a tone.
fn wav(enc: Pcm, rate: u32, channels: u16, freq: f32, sizes: Sizes) -> Vec<u8> {
    let fmt = fmt_pcm(enc, rate, channels);
    riff(&[(b"fmt ", &fmt)], &pcm_bytes(enc, &tone(freq, rate, channels)), sizes)
}

/// The same 16-bit PCM as an RF64 file (EBU Tech 3306): `ds64` holding the
/// sizes, the RIFF and `data` sizes 0xFFFFFFFF.
fn rf64(rate: u32, channels: u16, freq: f32) -> Vec<u8> {
    let fmt = fmt_pcm(Pcm::S16, rate, channels);
    let data = pcm_bytes(Pcm::S16, &tone(freq, rate, channels));
    let frames = (data.len() / usize::from(channels * 2)) as u64;
    let total = 4 + (8 + 28) + (8 + fmt.len()) + 8 + data.len();
    let mut ds64 = Vec::new();
    ds64.extend((total as u64).to_le_bytes());
    ds64.extend((data.len() as u64).to_le_bytes());
    ds64.extend(frames.to_le_bytes());
    ds64.extend(0u32.to_le_bytes());
    let mut out = riff(
        &[(b"ds64", &ds64), (b"fmt ", &fmt)],
        &data,
        Sizes { riff: Some(u32::MAX), data: Some(u32::MAX) },
    );
    out[..4].copy_from_slice(b"RF64");
    out
}

const IMA_STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];
const IMA_INDEX: [i32; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

/// A Microsoft IMA ADPCM WAV (tag 0x11) of a tone, 1024-byte blocks per
/// channel pair (512 per channel), as the game's ambient sounds are.
fn ima_adpcm(rate: u32, channels: u16, freq: f32) -> Vec<u8> {
    let ch = usize::from(channels);
    let align = 512 * ch;
    let per_block = (align - 4 * ch) * 2 / ch + 1;
    let samples: Vec<i32> =
        tone(freq, rate, channels).iter().map(|s| (s * 32767.0).round() as i32).collect();
    let frames = samples.len() / ch;
    let mut state = vec![(0i32, 0i32); ch];
    let mut data = Vec::new();
    let mut f = 0;
    while f < frames {
        let at = |i: usize, c: usize| samples.get((f + i) * ch + c).copied().unwrap_or(0);
        for (c, st) in state.iter_mut().enumerate() {
            st.0 = at(0, c);
            data.extend((st.0 as i16).to_le_bytes());
            data.push(st.1 as u8);
            data.push(0);
        }
        // Then 8 samples a channel at a time, channel by channel.
        let mut i = 1;
        while i < per_block {
            for (c, st) in state.iter_mut().enumerate() {
                let mut word = [0u8; 4];
                for k in 0..8 {
                    let n = encode_ima(at(i + k, c), st);
                    word[k / 2] |= n << ((k % 2) * 4);
                }
                data.extend(word);
            }
            i += 8;
        }
        f += per_block;
    }
    let mut fmt = Vec::new();
    fmt.extend(0x11u16.to_le_bytes());
    fmt.extend(channels.to_le_bytes());
    fmt.extend(rate.to_le_bytes());
    fmt.extend(((rate as usize * align / per_block) as u32).to_le_bytes());
    fmt.extend((align as u16).to_le_bytes());
    fmt.extend(4u16.to_le_bytes());
    fmt.extend(2u16.to_le_bytes());
    fmt.extend((per_block as u16).to_le_bytes());
    let fact = (frames as u32).to_le_bytes();
    riff(&[(b"fmt ", &fmt), (b"fact", &fact)], &data, Sizes::default())
}

fn encode_ima(sample: i32, (pred, index): &mut (i32, i32)) -> u8 {
    let mut step = IMA_STEPS[*index as usize];
    let mut diff = sample - *pred;
    let mut nib = 0u8;
    if diff < 0 {
        nib = 8;
        diff = -diff;
    }
    let mut delta = step >> 3;
    for bit in [4u8, 2, 1] {
        if diff >= step {
            nib |= bit;
            diff -= step;
            delta += step;
        }
        step >>= 1;
    }
    *pred = if nib & 8 != 0 { *pred - delta } else { *pred + delta }.clamp(-32768, 32767);
    *index = (*index + IMA_INDEX[usize::from(nib)]).clamp(0, 88);
    nib
}

/// An MP3's first frame: (sample rate, channels, bit rate in bits/s).
fn mp3_frame(mp3: &[u8]) -> Option<(u32, u16, u32)> {
    let h = u32::from_be_bytes(mp3.get(..4)?.try_into().ok()?);
    if h >> 21 != 0x7FF || (h >> 17) & 3 != 1 {
        return None; // (not a sync, or not layer III)
    }
    let version = (h >> 19) & 3; // 3: MPEG-1, 2: MPEG-2, 0: MPEG-2.5
    let rates = match version {
        3 => [44100, 48000, 32000],
        2 => [22050, 24000, 16000],
        0 => [11025, 12000, 8000],
        _ => return None,
    };
    let rate = *rates.get(((h >> 10) & 3) as usize)?;
    let kbps_v1 = [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320];
    let kbps_v2 = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
    let idx = ((h >> 12) & 15) as usize;
    let kbps = if version == 3 { kbps_v1.get(idx)? } else { kbps_v2.get(idx)? };
    let channels = if (h >> 6) & 3 == 3 { 1 } else { 2 };
    Some((rate, channels, kbps * 1000))
}

/// The MP3 inside a RIFF WAVE: `fmt ` MPEGLAYER3WAVEFORMAT (tag 0x55, 30
/// bytes), as ffmpeg's `-c:a copy -f wav` writes it.
fn mp3_in_riff(mp3: &[u8]) -> Vec<u8> {
    let (rate, channels, bps) = mp3_frame(mp3).expect("an MP3 frame first");
    let block = (144 * bps / rate) as u16;
    let mut fmt = Vec::new();
    fmt.extend(0x55u16.to_le_bytes());
    fmt.extend(channels.to_le_bytes());
    fmt.extend(rate.to_le_bytes());
    fmt.extend((bps / 8).to_le_bytes());
    fmt.extend(1u16.to_le_bytes()); // block align
    fmt.extend(0u16.to_le_bytes()); // bits per sample
    fmt.extend(12u16.to_le_bytes()); // extra
    fmt.extend(1u16.to_le_bytes()); // MPEGLAYER3_ID_MPEG
    fmt.extend(0u32.to_le_bytes()); // flags: padding as encoded
    fmt.extend(block.to_le_bytes());
    fmt.extend(1u16.to_le_bytes()); // frames per block
    fmt.extend(1393u16.to_le_bytes()); // codec delay
    assert_eq!(fmt.len(), 30);
    riff(&[(b"fmt ", &fmt)], mp3, Sizes::default())
}

/// An ID3v2.3 tag: a title frame, then `padding` zero bytes.
fn id3(padding: usize) -> Vec<u8> {
    let mut frames = b"TIT2".to_vec();
    let text = b"\0Moonglow test";
    frames.extend((text.len() as u32).to_be_bytes());
    frames.extend([0, 0]);
    frames.extend(text);
    frames.resize(frames.len() + padding, 0);
    let n = frames.len() as u32;
    let mut out = b"ID3\x03\x00\x00".to_vec();
    out.extend([
        (n >> 21) as u8 & 0x7F,
        (n >> 14) as u8 & 0x7F,
        (n >> 7) as u8 & 0x7F,
        n as u8 & 0x7F,
    ]);
    out.extend(frames);
    out
}

fn bmu(rest: &[u8]) -> Vec<u8> {
    [&mg_audio::BMU_HEADER[..], rest].concat()
}

/// Every form, in order: each kind's controls first, and after each group
/// of forms that may be silent a known-good one again (a check: a form
/// that stopped the music or sounds for good would silence it).
fn forms(mp3: &[u8], game_track: &[u8], shipped: &str, shipped_data: &[u8]) -> Vec<Form> {
    use Kind::{Ambient, Music, Sound};
    let no = Sizes::default();
    let mut tones = (0..).map(|i| 400.0 + 90.0 * i as f32);
    let mut next = || tones.next().unwrap();
    let mut out = Vec::new();
    // (name, kind, what, files, tone, expect, control)
    let mut add = |name: &'static str,
                   kind,
                   what: &str,
                   files: Vec<(ResType, Vec<u8>)>,
                   tone: Option<f32>,
                   expect: Option<bool>,
                   control: bool| {
        out.push(Form {
            name,
            res: name.to_string(),
            kind,
            what: what.into(),
            files,
            tone,
            expect,
            control,
        });
    };
    let b = ResType::BMU;
    let w = ResType::WAV;
    let pcm = |f: f32, sizes: Sizes| wav(Pcm::S16, 44100, 2, f, sizes);
    let game = || vec![(b, game_track.to_vec())];
    let big = id3(2 << 20);
    let small = id3(0);
    let zeros = vec![0u8; 1024];
    // Junk with a false frame sync at its start.
    let mut junk = b"\xFF\xFB".to_vec();
    while junk.len() < 1024 {
        junk.extend(b" Moonglow junk, no frame here.");
    }
    junk.truncate(1024);
    let riff_small = Sizes { riff: Some(36), data: None };

    // Music.
    add("mg_m_none", Music, "row with no file (silent control)", vec![], None, Some(false), true);
    add(
        "mg_m_game",
        Music,
        "game track as shipped (BMU V1.0 + ID3 + MP3)",
        game(),
        None,
        Some(true),
        true,
    );
    let f = next();
    add(
        "mg_m_s16s44",
        Music,
        "PCM s16 stereo 44100 WAV as .bmu",
        vec![(b, pcm(f, no))],
        Some(f),
        Some(false),
        false,
    );
    let f = next();
    add(
        "mg_m_wavonly",
        Music,
        "PCM s16 stereo 44100 WAV as .wav only",
        vec![(w, pcm(f, no))],
        Some(f),
        None,
        false,
    );
    add("mg_m_wbmu", Music, "BMU V1.0 + MP3 as .wav only", vec![(w, bmu(mp3))], None, None, false);
    add("mg_m_wbare", Music, "bare MP3 as .wav only", vec![(w, mp3.to_vec())], None, None, false);
    add("mg_m_chk1", Music, "game track (check)", game(), None, Some(true), true);
    add(
        "mg_m_bare",
        Music,
        "bare MP3 (no header)",
        vec![(b, mp3.to_vec())],
        None,
        Some(true),
        false,
    );
    add(
        "mg_m_mp3riff",
        Music,
        "MP3 in RIFF WAVE (tag 0x55) as .bmu",
        vec![(b, mp3_in_riff(mp3))],
        None,
        Some(true),
        false,
    );
    add(
        "mg_m_id3big",
        Music,
        "bare MP3, 2 MiB ID3v2 tag in front",
        vec![(b, [&big[..], mp3].concat())],
        None,
        Some(true),
        false,
    );
    add(
        "mg_m_id3small",
        Music,
        "bare MP3, small ID3v2 tag in front",
        vec![(b, [&small[..], mp3].concat())],
        None,
        Some(true),
        false,
    );
    add(
        "mg_m_zeros",
        Music,
        "bare MP3, 1 KiB of zeros in front",
        vec![(b, [&zeros[..], mp3].concat())],
        None,
        None,
        false,
    );
    add(
        "mg_m_junk",
        Music,
        "bare MP3, 1 KiB of junk with a false sync in front",
        vec![(b, [&junk[..], mp3].concat())],
        None,
        None,
        false,
    );
    add(
        "mg_m_bid3big",
        Music,
        "BMU V1.0 + 2 MiB ID3v2 tag + MP3",
        vec![(b, bmu(&[&big[..], mp3].concat()))],
        None,
        None,
        false,
    );
    add(
        "mg_m_bid3small",
        Music,
        "BMU V1.0 + small ID3v2 tag + MP3",
        vec![(b, bmu(&[&small[..], mp3].concat()))],
        None,
        None,
        false,
    );
    add(
        "mg_m_bzeros",
        Music,
        "BMU V1.0 + 1 KiB of zeros + MP3",
        vec![(b, bmu(&[&zeros[..], mp3].concat()))],
        None,
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_riff0",
        Music,
        "PCM s16 WAV, RIFF size 0, as .bmu",
        vec![(b, pcm(f, Sizes { riff: Some(0), data: None }))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_riffsmall",
        Music,
        "PCM s16 WAV, RIFF size 36 (too small), as .bmu",
        vec![(b, pcm(f, riff_small))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_dataffff",
        Music,
        "PCM s16 WAV, data size 0xFFFFFFFF, as .bmu",
        vec![(b, pcm(f, Sizes { riff: None, data: Some(u32::MAX) }))],
        Some(f),
        None,
        false,
    );
    add("mg_m_chk2", Music, "game track (check)", game(), None, Some(true), true);
    let f = next();
    add(
        "mg_m_data0",
        Music,
        "PCM s16 WAV, data size 0 with data after, as .bmu",
        vec![(b, pcm(f, Sizes { riff: None, data: Some(0) }))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_rf64",
        Music,
        "PCM s16 RF64 as .bmu",
        vec![(b, rf64(44100, 2, f))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_s16m22",
        Music,
        "PCM s16 mono 22050 WAV as .bmu",
        vec![(b, wav(Pcm::S16, 22050, 1, f, no))],
        Some(f),
        None,
        false,
    );
    add("mg_m_chk3", Music, "game track (check)", game(), None, Some(true), true);
    let f = next();
    add(
        "mg_m_u8",
        Music,
        "PCM u8 stereo 44100 WAV as .bmu",
        vec![(b, wav(Pcm::U8, 44100, 2, f, no))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_s24",
        Music,
        "PCM s24 stereo 44100 WAV as .bmu",
        vec![(b, wav(Pcm::S24, 44100, 2, f, no))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_f32",
        Music,
        "float32 stereo 44100 WAV as .bmu",
        vec![(b, wav(Pcm::F32, 44100, 2, f, no))],
        Some(f),
        None,
        false,
    );
    add("mg_m_chk4", Music, "game track (check)", game(), None, Some(true), true);
    let f = next();
    add(
        "mg_m_adpcm",
        Music,
        "IMA ADPCM stereo 44100 WAV as .bmu",
        vec![(b, ima_adpcm(44100, 2, f))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_m_bmupcm",
        Music,
        "BMU V1.0 + PCM s16 WAV as .bmu",
        vec![(b, bmu(&pcm(f, no)))],
        Some(f),
        None,
        false,
    );
    add("mg_m_game2", Music, "game track (closing check)", game(), None, Some(true), true);

    // Ambient sounds.
    add("mg_a_none", Ambient, "row with no file (silent control)", vec![], None, Some(false), true);
    let f = next();
    add(
        "mg_a_s16",
        Ambient,
        "PCM s16 stereo 44100 WAV as .wav (control)",
        vec![(w, pcm(f, no))],
        Some(f),
        Some(true),
        true,
    );
    let f = next();
    add(
        "mg_a_adpcm",
        Ambient,
        "IMA ADPCM stereo 44100 WAV as .wav",
        vec![(w, ima_adpcm(44100, 2, f))],
        Some(f),
        Some(true),
        false,
    );
    let f = next();
    add(
        "mg_a_riff0",
        Ambient,
        "PCM s16 WAV, RIFF size 0, as .wav",
        vec![(w, pcm(f, Sizes { riff: Some(0), data: None }))],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_a_riffsmall",
        Ambient,
        "PCM s16 WAV, RIFF size 36 (too small), as .wav",
        vec![(w, pcm(f, riff_small))],
        Some(f),
        None,
        false,
    );
    let f = next();
    let mut big_data = pcm(f, no);
    let n = u32::from_le_bytes(big_data[40..44].try_into().unwrap());
    big_data[40..44].copy_from_slice(&(n + 1_000_000).to_le_bytes());
    add(
        "mg_a_databig",
        Ambient,
        "PCM s16 WAV, data size 1 MB past the file's end, as .wav",
        vec![(w, big_data)],
        Some(f),
        None,
        false,
    );
    let f = next();
    add(
        "mg_a_chk1",
        Ambient,
        "PCM s16 WAV as .wav (check)",
        vec![(w, pcm(f, no))],
        Some(f),
        Some(true),
        true,
    );
    add("mg_a_bmu", Ambient, "BMU V1.0 + MP3 as .wav", vec![(w, bmu(mp3))], None, None, false);
    add("mg_a_bare", Ambient, "bare MP3 as .wav", vec![(w, mp3.to_vec())], None, None, false);
    add(
        "mg_a_mp3riff",
        Ambient,
        "MP3 in RIFF WAVE (tag 0x55) as .wav",
        vec![(w, mp3_in_riff(mp3))],
        None,
        None,
        false,
    );
    let f = next();
    add(
        "mg_a_bmuonly",
        Ambient,
        "PCM s16 WAV as .bmu only",
        vec![(b, pcm(f, no))],
        Some(f),
        None,
        false,
    );
    add(
        "mg_a_bmuonly3",
        Ambient,
        "BMU V1.0 + MP3 as .bmu only",
        vec![(b, bmu(mp3))],
        None,
        None,
        false,
    );
    let what = format!("shipped {shipped}.wav (BMU V1.0 + MP3), by its own name");
    add("mg_a_shipped", Ambient, &what, vec![], None, None, false);
    let what = format!("shipped {shipped}.wav (BMU V1.0 + MP3), copied as .wav");
    add("mg_a_shipcopy", Ambient, &what, vec![(w, shipped_data.to_vec())], None, None, false);
    let f = next();
    add(
        "mg_a_chk2",
        Ambient,
        "PCM s16 WAV as .wav (closing check)",
        vec![(w, pcm(f, no))],
        Some(f),
        Some(true),
        true,
    );

    // Sounds the PC plays (PlaySound), once: 8 seconds, so as to end in
    // the window, and mono, as the game's positional sounds are.
    let short = |f: f32| {
        let fmt = fmt_pcm(Pcm::S16, 44100, 1);
        riff(&[(b"fmt ", &fmt)], &pcm_bytes(Pcm::S16, &tone_of(8.0, f, 44100, 1)), no)
    };
    // The track's first 8 seconds of frames (128 kbit/s; a decoder finds
    // the next frame after the cut).
    let mp3_short = &mp3[..mp3.len().min(8 * 16_000)];
    add("mg_s_none", Sound, "no file (silent control)", vec![], None, Some(false), true);
    let f = next();
    add(
        "mg_s_s16",
        Sound,
        "PCM s16 mono 44100 WAV as .wav (control)",
        vec![(w, short(f))],
        Some(f),
        None,
        true,
    );
    let what = format!("shipped {shipped}.wav (BMU V1.0 + MP3), by its own name");
    add("mg_s_shipped", Sound, &what, vec![], None, None, false);
    let what = format!("shipped {shipped}.wav (BMU V1.0 + MP3), copied as .wav");
    add("mg_s_shipcopy", Sound, &what, vec![(w, shipped_data.to_vec())], None, None, false);
    let f = next();
    add(
        "mg_s_bmuonly",
        Sound,
        "PCM s16 mono 44100 WAV as .bmu only",
        vec![(b, short(f))],
        Some(f),
        None,
        false,
    );
    add(
        "mg_s_bmu",
        Sound,
        "BMU V1.0 + MP3 (8 s) as .wav",
        vec![(w, bmu(mp3_short))],
        None,
        None,
        false,
    );
    add(
        "mg_s_bare",
        Sound,
        "bare MP3 (8 s) as .wav",
        vec![(w, mp3_short.to_vec())],
        None,
        None,
        false,
    );
    add(
        "mg_s_bmuonly3",
        Sound,
        "BMU V1.0 + MP3 (8 s) as .bmu only",
        vec![(b, bmu(mp3_short))],
        None,
        None,
        false,
    );
    // The shipped sound's own MP3 (mono), with no header as .wav, and
    // whole as .bmu: the stereo MP3s above are not positional sounds.
    let ship_bare = shipped_data[mg_audio::BMU_HEADER.len()..].to_vec();
    add(
        "mg_s_shipbare",
        Sound,
        "shipped sound's MP3 (mono), no header, as .wav",
        vec![(w, ship_bare)],
        None,
        None,
        false,
    );
    add(
        "mg_s_shipbmu",
        Sound,
        "shipped sound (BMU V1.0 + mono MP3) as .bmu only",
        vec![(b, shipped_data.to_vec())],
        None,
        None,
        false,
    );
    let f = next();
    add(
        "mg_s_chk",
        Sound,
        "PCM s16 mono 44100 WAV as .wav (closing check)",
        vec![(w, short(f))],
        Some(f),
        None,
        true,
    );
    for f in &mut out {
        if f.name.ends_with("_shipped") {
            f.res = shipped.to_string();
        }
    }
    out
}

// ---------------------------------------------------------------- analysis

/// The capture's samples (mono: the channels' mean), read past the header
/// whatever its sizes say (a killed client leaves them unset).
fn capture(path: &Path) -> Option<(usize, Vec<f32>)> {
    let d = std::fs::read(path).ok()?;
    if d.get(..4)? != b"RIFF" || d.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut format = None;
    while at + 8 <= d.len() {
        let id = &d[at..at + 4];
        let len = u32::from_le_bytes(d[at + 4..at + 8].try_into().ok()?) as usize;
        if id == b"fmt " {
            let f = d.get(at + 8..at + 8 + 16)?;
            let tag = u16::from_le_bytes([f[0], f[1]]);
            let channels = u16::from_le_bytes([f[2], f[3]]);
            let rate = u32::from_le_bytes(f[4..8].try_into().ok()?);
            let bits = u16::from_le_bytes([f[14], f[15]]);
            format = Some((tag, channels, rate, bits));
        }
        if id == b"data" {
            let (tag, channels, rate, bits) = format?;
            assert!(
                (tag == 1 || tag == 0xFFFE) && bits == 16 && rate == RATE,
                "the capture is {tag:#x}, {channels} channels, {rate} Hz, {bits} bits"
            );
            let ch = usize::from(channels);
            let body = &d[at + 8..];
            let samples = body
                .chunks_exact(2 * ch)
                .map(|fr| {
                    fr.as_chunks::<2>()
                        .0
                        .iter()
                        .map(|s| f32::from(i16::from_le_bytes(*s)))
                        .sum::<f32>()
                        / (32768.0 * ch as f32)
                })
                .collect();
            return Some((at + 8, samples));
        }
        at += 8 + len + (len & 1);
    }
    None
}

fn db(x: f64) -> f64 {
    20.0 * x.max(1e-10).log10()
}

fn rms(s: &[f32]) -> f64 {
    (s.iter().map(|&v| f64::from(v) * f64::from(v)).sum::<f64>() / s.len().max(1) as f64).sqrt()
}

/// The RMS of the `freq` component of `s` (Goertzel).
fn goertzel(s: &[f32], freq: f64) -> f64 {
    let w = std::f64::consts::TAU * freq / f64::from(RATE);
    let c = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0, 0.0);
    for &v in s {
        let s0 = f64::from(v) + c * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let power = s1 * s1 + s2 * s2 - c * s1 * s2;
    // Amplitude 2|X|/N; a sine's RMS is that over √2.
    2.0 * power.sqrt() / s.len().max(1) as f64 / std::f64::consts::SQRT_2
}

/// Each form's stretch measured: `marks` are (marker, capture length when
/// it appeared).
fn measure(
    samples: &[f32],
    data_at: usize,
    marks: &[(String, u64)],
    forms: &[&Form],
) -> Vec<Outcome> {
    let frame = |len: u64| (len as usize).saturating_sub(data_at) / 4;
    let find = |m: &str| marks.iter().find(|(k, _)| k == m).map(|(_, n)| frame(*n));
    let sec = RATE as usize;
    forms
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let (Some(b), Some(e)) = (find(&format!("MG_BEGIN {i}")), find(&format!("MG_END {i}")))
            else {
                return if find(&format!("MG_BEGIN {i}")).is_some() {
                    Outcome::Ended
                } else {
                    Outcome::NotReached
                };
            };
            let e = e.min(samples.len());
            // From a second in to the end (a sound played once: all of
            // it), the onset over all of it.
            let all = &samples[b.min(e)..e];
            let body = if f.kind == Kind::Sound { all } else { &samples[(b + sec).min(e)..e] };
            let onset =
                all.chunks(sec / 50).position(|c| db(rms(c)) > -60.0).map(|k| k as f64 / 50.0);
            let gap_end = find(&format!("MG_BEGIN {}", i + 1))
                .or_else(|| find("MG_DONE"))
                .unwrap_or(e + 2 * sec)
                .min(samples.len());
            // (The last two seconds of it: a sound stopped fades out.)
            let gap = &samples[gap_end.saturating_sub(2 * sec).max(e).min(gap_end)..gap_end];
            Outcome::Measured(Measure {
                rms: db(rms(body)),
                peak: db(body.iter().fold(0f64, |m, &v| m.max(f64::from(v.abs())))),
                tone: f.tone.map(|t| db(goertzel(body, f64::from(t)))),
                onset,
                gap: db(rms(gap)),
            })
        })
        .collect()
}

// ---------------------------------------------------------------- the client

/// A game client running without a `-userdirectory` argument: the owner's
/// own game (a client with one is a test's, this one's or another
/// session's, and is left alone).
fn owners_game() -> Option<String> {
    let out = Command::new("ps").args(["-o", "pid=,args=", "-C", "nwmain-linux"]).output().ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| !l.split_whitespace().any(|a| a == "-userdirectory"))
        .map(|l| l.trim().to_string())
}

/// The PIDs of clients whose arguments name `dir`.
fn our_clients(dir: &Path) -> Vec<String> {
    let Ok(out) = Command::new("ps").args(["-o", "pid=,args=", "-C", "nwmain-linux"]).output()
    else {
        return Vec::new();
    };
    let dir = dir.canonicalize().unwrap_or_else(|_| dir.to_path_buf());
    let dir = dir.to_string_lossy().to_string();
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| l.contains(&format!("-userdirectory {dir}/user")))
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect()
}

/// Stops the client started (its sandbox, then any client of `dir` left).
fn stop(child: &mut Child, dir: &Path) {
    let _ = child.kill();
    let _ = child.wait();
    for _ in 0..20 {
        let left = our_clients(dir);
        if left.is_empty() {
            return;
        }
        for pid in &left {
            let _ = Command::new("kill").args(["-9", pid]).status();
        }
        std::thread::sleep(Duration::from_millis(250));
    }
    panic!("the test client of {} did not stop: {:?}", dir.display(), our_clients(dir));
}

/// Builds the module and hak for `forms` in `dir/user` and runs the client
/// on it: the markers with the capture's length at each, and the capture.
fn launch(
    base: &GameData,
    dir: &Path,
    forms: &[&Form],
    by_area: bool,
) -> (Vec<(String, u64)>, PathBuf) {
    let user = dir.join("user");
    std::fs::create_dir_all(user.join("hak")).unwrap();
    std::fs::write(user.join("settings.tml"), SETTINGS).unwrap();

    // The tables: the game's, a row added for each form.
    let mut rows = Vec::new();
    let mut hak = ErfWriter::new(*b"HAK ");
    let mut data: Vec<(ResRef, ResType, Vec<u8>)> = Vec::new();
    for (table, kind) in [("ambientmusic", Kind::Music), ("ambientsound", Kind::Ambient)] {
        let key = ResKey::parse(table, ResType::TWODA).unwrap();
        let mut t =
            mg_2da::TwoDa::parse(&base.resman.get(&key).unwrap(), Codepage::default()).unwrap();
        let res = t.column("Resource").unwrap();
        let desc = t.column("Description").unwrap();
        let strref = t.cell(1, desc).map(str::to_string);
        for f in forms.iter().filter(|f| f.kind == kind) {
            let r = t.add_row();
            t.set(r, res, Some(f.res.clone())).unwrap();
            t.set(r, desc, strref.clone()).unwrap();
            rows.push((f.name, r));
        }
        data.push((
            ResRef::from_str(table).unwrap(),
            ResType::TWODA,
            t.to_bytes(Codepage::default()).unwrap(),
        ));
    }
    // The forms' files in the hak, or with `MG_AUDIO_FOLDER` loose in that
    // folder of the user directory: `music` the music forms', `ambient`
    // the ambient sounds', `override` all (the controls' stay in the hak).
    let folder = std::env::var("MG_AUDIO_FOLDER").ok().filter(|f| !f.is_empty());
    for f in forms {
        let loose = folder.as_ref().filter(|d| {
            !f.control
                && match d.as_str() {
                    "music" => f.kind == Kind::Music,
                    "ambient" => f.kind == Kind::Ambient,
                    _ => true,
                }
        });
        for (ty, bytes) in &f.files {
            match loose {
                Some(folder) => {
                    let ext = ty.extension().unwrap();
                    std::fs::create_dir_all(user.join(folder)).unwrap();
                    std::fs::write(user.join(folder).join(format!("{}.{ext}", f.name)), bytes)
                        .unwrap();
                }
                None => data.push((ResRef::from_str(f.name).unwrap(), *ty, bytes.clone())),
            }
        }
    }
    for (r, t, d) in &data {
        hak.add(*r, *t, &d[..]).unwrap();
    }
    std::fs::write(user.join("hak/mg_audio.hak"), hak.to_bytes().unwrap()).unwrap();
    let row = |name: &str| rows.iter().find(|(n, _)| *n == name).unwrap().1;

    // The script: each form in turn.
    let mut steps = String::new();
    let mut t = LEAD;
    for (i, f) in forms.iter().enumerate() {
        // A stopped ambient sound fades for longer than a gap, and only
        // the next ambient row cuts it: a longer gap after the last.
        if i > 0 && forms[i - 1].kind == Kind::Ambient && f.kind != Kind::Ambient {
            t += 2.0 * GAP;
        }
        let kind = match f.kind {
            Kind::Music => 1,
            Kind::Ambient => 0,
            Kind::Sound => 2,
        };
        let script = i32::from(!(by_area && i == 0));
        let r = if f.kind == Kind::Sound { 0 } else { row(f.name) };
        steps.push_str(&format!(
            "    DelayCommand({t:.1}, Begin({i}, {kind}, {r}, {script}, \"{}\"));\n    \
             DelayCommand({:.1}, End({i}));\n",
            f.res,
            t + PLAY
        ));
        t += PLAY + GAP;
    }
    steps.push_str(&format!("    DelayCommand({t:.1}, Log(\"MG_DONE\"));\n"));
    let enter = format!(
        r#"
void Log(string s) {{ WriteTimestampedLogEntry(s); }}
void Begin(int i, int kind, int row, int script, string res)
{{
    object a = GetArea(GetFirstPC());
    if (kind == 2) {{
        AssignCommand(GetFirstPC(), PlaySound(res));
    }} else if (script && kind == 1) {{
        MusicBackgroundChangeDay(a, row);
        MusicBackgroundChangeNight(a, row);
        MusicBackgroundPlay(a);
    }} else if (script) {{
        AmbientSoundChangeDay(a, row);
        AmbientSoundChangeNight(a, row);
        AmbientSoundSetDayVolume(a, 100);
        AmbientSoundSetNightVolume(a, 100);
        AmbientSoundPlay(a);
    }}
    Log("MG_BEGIN " + IntToString(i));
}}
void End(int i)
{{
    object a = GetArea(GetFirstPC());
    // (Stopped alone, the music starts again a second later: the track
    // emptied first.)
    MusicBackgroundChangeDay(a, 0);
    MusicBackgroundChangeNight(a, 0);
    MusicBackgroundStop(a);
    AmbientSoundChangeDay(a, 0);
    AmbientSoundChangeNight(a, 0);
    AmbientSoundStop(a);
    Log("MG_END " + IntToString(i));
}}
void main()
{{
    object pc = GetEnteringObject();
    if (!GetIsPC(pc)) return;
    Log("MG_READY");
{steps}}}
"#
    );
    let mut rng = fastrand::Rng::with_seed(5);
    let mut m = new_module(base, "MgAudio", &mut rng).unwrap();
    let spec = AreaSpec {
        name: "Audio".into(),
        tileset: ResRef::from_str("tic01").unwrap(),
        width: 2,
        height: 2,
    };
    let area = add_area(&mut m, base, &spec, &mut rng).unwrap();
    let git_key = ResKey::new(area, ResType::GIT);
    let mut git = m.gff(&git_key).unwrap().unwrap();
    let props = git.root.child_mut("AreaProperties").expect("AreaProperties");
    let first =
        forms.first().filter(|f| by_area && f.kind == Kind::Music).map_or(0, |f| row(f.name));
    for (label, v) in [
        ("MusicDay", first),
        ("MusicNight", first),
        ("MusicBattle", 0),
        ("MusicDelay", 0),
        ("AmbientSndDay", 0),
        ("AmbientSndNight", 0),
        ("AmbientSndDayVol", 100),
        ("AmbientSndNitVol", 100),
    ] {
        props.set(label, Value::Int(v as i32));
    }
    m.set_gff(git_key, &git).unwrap();
    m.set(ResKey::parse("mg_enter", ResType::NCS).unwrap(), compile(dir, "mg_enter", &enter));
    let mut info = m.info().unwrap();
    info.root.write(&ifo::MOD_ON_CLIENT_ENTR, ResRef::from_str("mg_enter").unwrap());
    let list = mg_module::attach::hak_list(&info.root, &["mg_audio".to_string()]);
    info.root.set("Mod_HakList", list);
    m.set_info(&info).unwrap();
    m.save_as(&ModuleLocation::Archive(user.join("modules/MgAudio.mod"))).unwrap();

    // The client.
    if let Some(game) = owners_game() {
        let pid = game.split_whitespace().next().unwrap_or("?");
        panic!("the owner's game is running (PID {pid}: {game}): not launching");
    }
    let out = dir.join("capture.wav");
    let _ = std::fs::remove_file(&out);
    let log = user.join("logs/nwclientLog1.txt");
    let _ = std::fs::remove_file(&log);
    let mut child = Command::new(repo().join("tools/nwclient/run-client.sh"))
        .arg(dir)
        .args(["+TestNewModule", "MgAudio"])
        .env("MG_CLIENT_AUDIO_OUT", &out)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("run-client.sh");
    let budget = Duration::from_secs(180) + Duration::from_secs_f64(t + 10.0);
    let start = Instant::now();
    let mut marks: Vec<(String, u64)> = Vec::new();
    while start.elapsed() < budget {
        let len = std::fs::metadata(&out).map_or(0, |m| m.len());
        let text = std::fs::read_to_string(&log).unwrap_or_default();
        for l in text.lines() {
            let Some(k) = l.find("MG_") else { continue };
            let mark = l[k..].trim().to_string();
            if !marks.iter().any(|(m, _)| *m == mark) {
                eprintln!("  {:6.1} s  {mark}", start.elapsed().as_secs_f64());
                marks.push((mark, len));
            }
        }
        if marks.iter().any(|(m, _)| m == "MG_DONE") {
            break;
        }
        if child.try_wait().is_ok_and(|s| s.is_some()) {
            eprintln!("the client ended (see {})", dir.display());
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    stop(&mut child, dir);
    (marks, out)
}

/// What the client did with a form: its tone (the authored WAVs: most of
/// what was heard, at the frequency written), or a level well over the
/// silent control's (the MP3s).
fn verdict(f: &Form, m: &Measure, silent: f64) -> String {
    let loud = m.rms > -65.0 && m.rms > silent + 20.0;
    match m.tone {
        Some(t) if t > -50.0 && t >= m.rms - 6.0 => "played".into(),
        Some(_) if loud => "sound, not the tone".into(),
        Some(_) if m.rms > silent + 6.0 => "weak (not the tone)".into(),
        Some(_) => "silent".into(),
        None if f.missing() && m.rms > -70.0 => "NOT silent (control)".into(),
        None if f.missing() => "silent (control)".into(),
        None if loud => "played".into(),
        None if m.rms > silent + 6.0 => "weak".into(),
        None => "silent".into(),
    }
}

/// Whether the client played `f` (its verdict is "played").
fn played(f: &Form, m: &Measure, silent: f64) -> bool {
    verdict(f, m, silent) == "played"
}

/// The silent control's level for each kind in `results` (its stretch or
/// gap, the louder).
fn silent_level(results: &[(&Form, Outcome)], kind: Kind) -> f64 {
    results
        .iter()
        .find(|(f, _)| f.kind == kind && f.missing())
        .and_then(|(_, o)| match o {
            Outcome::Measured(m) => Some(m.rms.max(m.gap)),
            _ => None,
        })
        .unwrap_or(-100.0)
}

/// A shipped sound that is `BMU V1.0` + MP3 under a `.wav` name, 2.5 to 8
/// seconds long: its name and bytes.
fn shipped_bmu_wav(rm: &mg_resman::ResMan) -> (String, Vec<u8>) {
    let mut names: Vec<_> = rm.list(ResType::WAV).into_iter().collect();
    names.sort_by_key(|r| r.to_string());
    for r in names {
        let key = ResKey::new(r, ResType::WAV);
        let Ok(d) = rm.get(&key) else { continue };
        if !d.starts_with(mg_audio::BMU_HEADER)
            || !mg_audio::container(&d).is_some_and(|c| c.0 == mg_audio::Container::Mp3)
        {
            continue;
        }
        if mg_audio::decode(&d).is_ok_and(|p| (2.5..8.0).contains(&p.seconds())) {
            return (r.to_string(), d.to_vec());
        }
    }
    panic!("no shipped BMU V1.0 + MP3 .wav of 2.5 to 8 seconds");
}

#[test]
#[ignore]
fn which_audio_files_the_client_plays() {
    let root = corpus!();
    let _ = oracle_tool!("nwn_script_comp");
    let base = GameData::open(&GameInstall::new(&root, None, "en")).unwrap();

    // A game track: ambientmusic.2da's first.
    let t = mg_2da::TwoDa::parse(
        &base.resman.get(&ResKey::parse("ambientmusic", ResType::TWODA).unwrap()).unwrap(),
        Codepage::default(),
    )
    .unwrap();
    let name = t.get(1, "Resource").unwrap().to_string();
    let game_track =
        base.resman.get(&ResKey::parse(&name, ResType::BMU).unwrap()).unwrap().to_vec();
    assert!(game_track.starts_with(mg_audio::BMU_HEADER), "{name} has the BMU header");
    // The MP3 frames alone: the game's tracks carry an ID3v2 tag of their
    // own after the header (about a kilobyte of padding).
    let mut mp3 = &game_track[8..];
    if mp3.starts_with(b"ID3") && mp3.len() > 10 {
        let n = mp3[6..10].iter().fold(0usize, |n, &b| n << 7 | usize::from(b & 0x7F));
        eprintln!("{name}: an ID3v2 tag of {} bytes after the header", n + 10);
        mp3 = &mp3[10 + n..];
    }
    let mp3 = mp3.to_vec();
    let info = mp3_frame(&mp3).expect("the MP3 starts with a frame");
    eprintln!("game track {name}: {} bytes of MP3, {info:?}", mp3.len());
    let (shipped, shipped_data) = shipped_bmu_wav(&base.resman);
    eprintln!("shipped sound: {shipped}.wav, {} bytes", shipped_data.len());

    let all = forms(&mp3, &game_track, &shipped, &shipped_data);
    // What mg_audio makes of each form (and that the authored WAVs decode
    // to their tones: the fixtures are right).
    let mut ours = std::collections::HashMap::new();
    for f in &all {
        let data = match f.files.first() {
            Some((_, d)) => d.as_slice(),
            None if !f.missing() => shipped_data.as_slice(),
            None => continue,
        };
        let r = match mg_audio::decode(data) {
            Ok(p) => {
                if let Some(t) = f.tone {
                    let mono: Vec<f32> =
                        p.samples.chunks(usize::from(p.channels)).map(|c| c[0]).collect();
                    let level =
                        if p.rate == RATE { db(goertzel(&mono, f64::from(t))) } else { f64::NAN };
                    format!(
                        "decodes ({} Hz, {} ch, {:.1} s, tone {level:.1} dB)",
                        p.rate,
                        p.channels,
                        p.seconds()
                    )
                } else {
                    format!("decodes ({} Hz, {} ch, {:.1} s)", p.rate, p.channels, p.seconds())
                }
            }
            Err(e) => format!("refused: {e}"),
        };
        ours.insert(f.name, r);
    }

    let only: Option<Vec<String>> = std::env::var("MG_AUDIO_FORMS")
        .ok()
        .map(|s| s.split(',').map(|n| n.trim().to_string()).collect());
    let named = |f: &Form| only.as_ref().is_some_and(|o| o.iter().any(|n| n == f.name));
    let by_area = std::env::var_os("MG_AUDIO_BY_AREA").is_some();
    let folder = std::env::var_os("MG_AUDIO_FOLDER").is_some_and(|f| !f.is_empty());
    let mut pending: Vec<&Form> =
        all.iter().filter(|f| only.is_none() || f.control || named(f)).collect();
    // The first music form named (or the report's own) by the area's
    // properties.
    let by_area_form = by_area
        .then(|| {
            pending.iter().position(|f| {
                f.kind == Kind::Music
                    && !f.missing()
                    && if only.is_some() { named(f) } else { f.name == "mg_m_s16s44" }
            })
        })
        .flatten()
        .map(|p| {
            let f = pending.remove(p);
            pending.insert(0, f);
            f.name
        });
    let mut results: Vec<(&Form, Outcome)> = Vec::new();
    let (mut run, mut disturbed) = (0, 0);
    while !pending.is_empty() && run < 8 {
        let dir = scratch_dir(&format!("client_audio_{run}"));
        eprintln!("launch {run}: {} forms in {}", pending.len(), dir.display());
        let first = by_area_form.is_some() && pending.first().map(|f| f.name) == by_area_form;
        let (marks, out) = launch(&base, &dir, &pending, first);
        run += 1;
        if marks.iter().all(|(m, _)| m != "MG_READY") {
            panic!("the client did not reach the area: see {}", dir.display());
        }
        let Some((data_at, samples)) = capture(&out) else {
            panic!("no capture written ({}): see {}", out.display(), dir.display());
        };
        eprintln!("  capture: {:.1} s", samples.len() as f64 / f64::from(RATE));
        let outcomes = measure(&samples, data_at, &marks, &pending);
        let these: Vec<(&Form, Outcome)> = pending
            .iter()
            .copied()
            .zip(outcomes)
            .filter(|(_, o)| !matches!(o, Outcome::NotReached))
            .collect();
        // The controls as they must be, or the launch is played again
        // (another client on the display, a busy machine).
        let failed: Vec<&str> = these
            .iter()
            .filter(|(f, o)| {
                f.control
                    && f.expect.is_some_and(|e| match o {
                        Outcome::Measured(m) => played(f, m, silent_level(&these, f.kind)) != e,
                        _ => true,
                    })
            })
            .map(|(f, _)| f.name)
            .collect();
        if !failed.is_empty() && disturbed < 2 {
            disturbed += 1;
            eprintln!("  controls failed ({failed:?}): launching again");
            continue;
        }
        let reached: Vec<&str> = these.iter().map(|(f, _)| f.name).collect();
        pending.retain(|f| !reached.contains(&f.name));
        results.extend(these);
    }

    let silent = |kind: Kind| silent_level(&results, kind);
    // Whether a kind's loud controls and checks played: without them, a
    // silent form says nothing.
    let heard = |kind: Kind| {
        results.iter().any(|(f, o)| {
            f.kind == kind
                && f.control
                && !f.missing()
                && matches!(o, Outcome::Measured(m) if played(f, m, silent(kind)))
        })
    };
    eprintln!(
        "\n{run} launches, {disturbed} of them again for failed controls\n\n\
         | form | file | client | level dBFS | tone dBFS | peak | onset s | gap dBFS | mg_audio |"
    );
    eprintln!("|---|---|---|---|---|---|---|---|---|");
    let mut wrong = Vec::new();
    for (f, o) in &results {
        let file = match f.files.first() {
            Some((t, _)) => format!("{}.{}", f.name, t.extension().unwrap_or("?")),
            None if !f.missing() => format!("{}.wav (game)", f.res),
            None => "(none)".to_string(),
        };
        let ours = ours.get(f.name).cloned().unwrap_or_default();
        let area = if by_area_form == Some(f.name) { ", by the area" } else { "" };
        match o {
            Outcome::Measured(m) => {
                let v = verdict(f, m, silent(f.kind));
                eprintln!(
                    "| {} ({:?}{area}) | {file} | {} | {:.1} | {} | {:.1} | {} | {:.1} | {ours} |",
                    f.what,
                    f.kind,
                    if heard(f.kind) || f.missing() {
                        v.clone()
                    } else {
                        format!("undetermined (control failed: {v})")
                    },
                    m.rms,
                    m.tone.map_or("-".into(), |t| format!("{t:.1}")),
                    m.peak,
                    m.onset.map_or("-".into(), |t| format!("{t:.2}")),
                    m.gap,
                );
                // The settled rows, as the hak and the script give them.
                let asserted = !folder && area.is_empty();
                if let Some(e) = f.expect.filter(|_| asserted)
                    && (v == "played") != e
                {
                    wrong.push(format!(
                        "{}: {v}, expected {}",
                        f.name,
                        if e { "played" } else { "silent" }
                    ));
                }
            }
            o => {
                eprintln!(
                    "| {} ({:?}{area}) | {file} | {o:?} | | | | | | {ours} |",
                    f.what, f.kind
                );
                if f.expect.is_some() {
                    wrong.push(format!("{}: {o:?}", f.name));
                }
            }
        }
    }
    let unrun: Vec<_> = pending.iter().map(|f| f.name).collect();
    assert!(unrun.is_empty(), "not run: {unrun:?}");
    assert!(wrong.is_empty(), "{wrong:#?}");
}
