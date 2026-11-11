//! The game's sounds and music, decoded to PCM for playback. Decoding is
//! Symphonia's; this module decides what a file holds, as the game does, by
//! its bytes and not its name.
//!
//! What the game plays is known from the real client (the hand-run test
//! `client_audio.rs` in `mg-corpus-tests`, which captures what it plays) and
//! from nwn.wiki, "Sounds and Music":
//!
//! - As **music** (an `ambientmusic.2da` row, a `.bmu` file) the game plays
//!   MP3 data in any wrapper: behind the eight-byte `BMU V1.0` header (as
//!   shipped), bare, behind an ID3v2 tag of any size or behind junk (it finds
//!   the first frame), and inside a RIFF WAVE with the `fmt ` tag 0x55
//!   (MPEGLAYER3), which is what a builder gets from renaming such a `.wav` to
//!   `.bmu`. It does not play a PCM WAVE as music. Moonglow does: that is
//!   kept, though the game would stay silent, and the app says so in its log.
//! - As an ambient **sound** (a `.wav` file) it plays PCM and IMA ADPCM
//!   WAVEs, and does not go by a WAVE's RIFF size field (a size of 0 plays).
//!   The base game's 12,800 sounds (counted in `audio.rs`) are MP3 behind
//!   `BMU V1.0` (most), RIFF WAVE with IMA ADPCM or PCM, and a few bare MP3s.
//!
//! So an MP3's first frame is found (not assumed at a fixed offset), a WAVE
//! holding MP3 is read from its `data` chunk, and a WAVE's RIFF size is not
//! believed. RF64 and WAVEs of other codecs are refused by name.

use std::io::Cursor;
use std::sync::Arc;

use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

/// The header of an MP3 stored as a game sound.
pub const BMU_HEADER: &[u8; 8] = b"BMU V1.0";

/// Decoded audio: interleaved samples in [-1, 1].
#[derive(Debug, Clone, PartialEq)]
pub struct Pcm {
    pub rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl Pcm {
    /// Frames (samples per channel).
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }

    /// The length in seconds.
    pub fn seconds(&self) -> f32 {
        self.frames() as f32 / self.rate.max(1) as f32
    }
}

/// What a sound file holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    /// MP3 data (after a `BMU V1.0` header, an ID3v2 tag, junk or a WAVE's
    /// chunks).
    Mp3,
    /// RIFF WAVE.
    Wave,
}

/// Why a sound could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("not a sound the game plays (neither WAVE nor MP3)")]
    Unknown,
    #[error("an RF64 WAVE, which the game does not play")]
    Rf64,
    #[error("a WAVE of a format Moonglow does not play ({})", wave_format_name(*.0))]
    WaveFormat(u16),
    #[error("no audio track")]
    NoTrack,
    #[error("{0}")]
    Decode(String),
}

impl From<SymphoniaError> for AudioError {
    fn from(e: SymphoniaError) -> AudioError {
        AudioError::Decode(e.to_string())
    }
}

/// A WAVE format tag, named when it is a common one.
fn wave_format_name(tag: u16) -> String {
    let name = match tag {
        0x0050 => "MPEG layer 1 or 2",
        0x0160..=0x0163 => "Windows Media Audio",
        0x00FF => "AAC",
        0x674F..=0x6771 => "Ogg Vorbis",
        _ => return format!("format 0x{tag:04X}"),
    };
    format!("format 0x{tag:04X}, {name}")
}

/// The format tags Symphonia's WAVE reader knows (PCM, MS ADPCM, float,
/// A-law, mu-law, IMA ADPCM, extensible); any other it refuses.
const SYMPHONIA_WAVE_TAGS: [u16; 7] = [0x01, 0x02, 0x03, 0x06, 0x07, 0x11, 0xFFFE];

/// The `fmt ` tag of an MP3 in a WAVE (MPEGLAYER3).
const WAVE_MPEGLAYER3: u16 = 0x55;

/// How far past where its search starts an MP3's first frame is looked for.
const MP3_SEARCH: usize = 1 << 20;

/// The length of the MP3 frame a header starts, and what it must share with
/// the next frame's header (version, layer, sample rate). `None` when the
/// four bytes are no valid header.
fn mp3_frame(h: &[u8]) -> Option<(usize, (u8, u8, u8))> {
    let &[b0, b1, b2, _] = h.get(..4)? else { return None };
    if b0 != 0xFF || b1 & 0xE0 != 0xE0 {
        return None;
    }
    let version = (b1 >> 3) & 3; // 0: MPEG 2.5, 1: reserved, 2: MPEG 2, 3: MPEG 1
    let layer = (b1 >> 1) & 3; // 0: reserved, 1: III, 2: II, 3: I
    let bitrate = usize::from(b2 >> 4);
    let rate = usize::from((b2 >> 2) & 3);
    let pad = usize::from((b2 >> 1) & 1);
    if version == 1 || layer == 0 || bitrate == 0 || bitrate == 15 || rate == 3 {
        return None;
    }
    const V1: [[u16; 14]; 3] = [
        [32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448], // layer I
        [32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384],    // layer II
        [32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320],     // layer III
    ];
    const V2_L1: [u16; 14] = [32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256];
    const V2_L23: [u16; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
    let kbps = usize::from(match (version, layer) {
        (3, l) => V1[usize::from(3 - l)][bitrate - 1],
        (_, 3) => V2_L1[bitrate - 1],
        _ => V2_L23[bitrate - 1],
    }) * 1000;
    let hz = usize::from(match version {
        3 => [44100u16, 48000, 32000][rate],
        2 => [22050, 24000, 16000][rate],
        _ => [11025, 12000, 8000][rate],
    });
    let len = match (version, layer) {
        (_, 3) => (12 * kbps / hz + pad) * 4,
        (3, _) | (_, 2) => 144 * kbps / hz + pad,
        _ => 72 * kbps / hz + pad, // MPEG 2/2.5 layer III
    };
    Some((len, (version, layer, rate as u8)))
}

/// Where the ID3v2 tag at `at` ends: `None` when there is none, or when it
/// claims more than the data has.
fn id3v2_end(data: &[u8], at: usize) -> Option<usize> {
    let h = data.get(at..at.checked_add(10)?)?;
    if &h[..3] != b"ID3" || h[3] == 0xFF || h[4] == 0xFF || h[6..10].iter().any(|b| b & 0x80 != 0) {
        return None;
    }
    let size = h[6..10].iter().fold(0usize, |n, b| n << 7 | usize::from(*b));
    let footer = if h[5] & 0x10 != 0 { 10 } else { 0 };
    let end = at.checked_add(10)?.checked_add(size)?.checked_add(footer)?;
    (end <= data.len()).then_some(end)
}

/// Where the MP3 data in `data[from..end]` starts: after any ID3v2 tags, the
/// first frame header that a second one confirms (the same version, layer
/// and sample rate one frame length on), or one whose frame ends the data.
/// Looked for up to a mebibyte on. Layers I, II and III are all confirmed.
pub fn find_mp3(data: &[u8], from: usize, end: usize) -> Option<usize> {
    let data = data.get(..end.min(data.len()))?;
    let mut start = from;
    while let Some(after) = id3v2_end(data, start) {
        start = after;
    }
    let stop = start.saturating_add(MP3_SEARCH).min(data.len());
    let mut at = start;
    while at < stop {
        at += data[at..stop].iter().position(|&b| b == 0xFF)?;
        if let Some((len, kind)) = mp3_frame(&data[at..]) {
            let next = at.checked_add(len)?;
            let confirmed = next == data.len()
                || data.get(next..).and_then(mp3_frame).is_some_and(|(_, k)| k == kind);
            if confirmed {
                return Some(at);
            }
        }
        at += 1;
    }
    None
}

/// What a RIFF WAVE's chunks say.
struct WaveInfo {
    /// The `fmt ` chunk's format tag.
    format: Option<u16>,
    /// Where the `fmt ` chunk ends, as it declares.
    fmt_end: u64,
    /// The `data` chunk: where its payload starts and the size it declares.
    data: Option<(usize, u64)>,
}

impl WaveInfo {
    /// Walks the chunks of a RIFF WAVE to `fmt ` and `data`.
    fn read(data: &[u8]) -> WaveInfo {
        let mut info = WaveInfo { format: None, fmt_end: 0, data: None };
        let mut pos = 12usize;
        while let Some(head) = pos.checked_add(8).and_then(|e| data.get(pos..e)) {
            let size = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
            let body = pos + 8;
            match &head[..4] {
                b"fmt " if info.format.is_none() => {
                    info.format =
                        data.get(body..body + 2).map(|t| u16::from_le_bytes([t[0], t[1]]));
                    info.fmt_end = body as u64 + size;
                }
                b"data" => {
                    info.data = Some((body, size));
                    break;
                }
                _ => {}
            }
            let next = body as u64 + size + (size & 1);
            if next > data.len() as u64 {
                break;
            }
            pos = next as usize;
        }
        info
    }

    /// The bytes of the `data` chunk, as far as the file has them.
    fn data_range(&self, len: usize) -> std::ops::Range<usize> {
        match self.data {
            Some((start, size)) => start..(start as u64 + size).min(len as u64) as usize,
            None => len..len,
        }
    }

    /// Whether the RIFF size field says less than the chunks need.
    fn riff_size_is_wrong(&self, data: &[u8]) -> bool {
        let Some(riff) = data.get(4..8) else { return false };
        let riff = u64::from(u32::from_le_bytes([riff[0], riff[1], riff[2], riff[3]]));
        let needed = self.fmt_end.max(self.data.map_or(0, |(start, size)| start as u64 + size));
        riff != u64::from(u32::MAX) && riff + 8 < needed
    }
}

/// Whether `data` is an RF64 or BW64 WAVE.
fn is_rf64(data: &[u8]) -> bool {
    data.len() >= 12 && matches!(&data[..4], b"RF64" | b"BW64") && &data[8..12] == b"WAVE"
}

/// What a sound file holds, and the bytes of it that are audio. An MP3's
/// range starts at its first frame (found, see [`find_mp3`]) and, inside a
/// WAVE, ends with its `data` chunk. `None` for what is neither WAVE nor MP3.
pub fn container(data: &[u8]) -> Option<(Container, std::ops::Range<usize>)> {
    let len = data.len();
    if data.starts_with(BMU_HEADER) {
        let start = find_mp3(data, BMU_HEADER.len(), len).unwrap_or(BMU_HEADER.len());
        return Some((Container::Mp3, start..len));
    }
    if len >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        let info = WaveInfo::read(data);
        if info.format == Some(WAVE_MPEGLAYER3) {
            let range = info.data_range(len);
            let start = find_mp3(data, range.start, range.end).unwrap_or(range.start);
            return Some((Container::Mp3, start..range.end));
        }
        return Some((Container::Wave, 0..len));
    }
    find_mp3(data, 0, len).map(|start| (Container::Mp3, start..len))
}

/// A sound's bytes, from where its audio starts to where it ends.
#[derive(Clone)]
struct Bytes {
    data: Arc<[u8]>,
    start: usize,
    end: usize,
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.data[self.start..self.end]
    }
}

/// A sound decoded as it plays: interleaved samples in [-1, 1].
pub struct Stream {
    reader: Box<dyn FormatReader>,
    decoder: Box<dyn AudioDecoder>,
    track: u32,
    rate: u32,
    channels: u16,
    chunk: Vec<f32>,
    pos: usize,
    done: bool,
}

impl std::fmt::Debug for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stream")
            .field("rate", &self.rate)
            .field("channels", &self.channels)
            .finish()
    }
}

impl Stream {
    /// Opens a game sound (`.wav` or `.bmu`) and decodes its first frames.
    pub fn open(data: Arc<[u8]>) -> Result<Stream, AudioError> {
        if is_rf64(&data) {
            return Err(AudioError::Rf64);
        }
        let (kind, range) = container(&data).ok_or(AudioError::Unknown)?;
        let mut data = data;
        if kind == Container::Wave {
            let info = WaveInfo::read(&data);
            if let Some(tag) = info.format.filter(|t| !SYMPHONIA_WAVE_TAGS.contains(t)) {
                return Err(AudioError::WaveFormat(tag));
            }
            if info.riff_size_is_wrong(&data) {
                // The size is not believed: Symphonia is given "unknown",
                // what a tool writing to a pipe leaves (a copy, only here).
                let mut fixed = data.to_vec();
                fixed[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
                data = Arc::from(fixed);
            }
        }
        let source = Cursor::new(Bytes { data, start: range.start, end: range.end });
        let stream = MediaSourceStream::new(Box::new(source), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        hint.with_extension(match kind {
            Container::Mp3 => "mp3",
            Container::Wave => "wav",
        });
        let reader = symphonia::default::get_probe().probe(
            &hint,
            stream,
            FormatOptions::default(),
            MetadataOptions::default(),
        )?;
        let track = reader.first_track_known_codec(TrackType::Audio).ok_or(AudioError::NoTrack)?;
        let id = track.id;
        let Some(CodecParameters::Audio(params)) = track.codec_params.clone() else {
            return Err(AudioError::NoTrack);
        };
        let decoder = symphonia::default::get_codecs()
            .make_audio_decoder(&params, &AudioDecoderOptions::default())?;
        let mut s = Stream {
            reader,
            decoder,
            track: id,
            rate: 0,
            channels: 0,
            chunk: Vec::new(),
            pos: 0,
            done: false,
        };
        if !s.refill()? || s.rate == 0 || s.channels == 0 {
            return Err(AudioError::Decode("no audio frames".into()));
        }
        Ok(s)
    }

    /// Samples per second (per channel).
    pub fn rate(&self) -> u32 {
        self.rate
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Decodes the next frames into the chunk; `false` at the end. Frames a
    /// decoder rejects are skipped, as players do; a truncated file ends
    /// where its data does.
    fn refill(&mut self) -> Result<bool, AudioError> {
        while !self.done {
            let packet = match self.reader.next_packet() {
                Ok(Some(p)) => p,
                Ok(None) | Err(SymphoniaError::IoError(_)) => {
                    self.done = true;
                    break;
                }
                Err(e) => return Err(e.into()),
            };
            if packet.track_id != self.track {
                continue;
            }
            match self.decoder.decode(&packet) {
                Ok(buf) => {
                    let channels = u16::try_from(buf.spec().channels().count()).unwrap_or(0);
                    if buf.frames() == 0 || channels == 0 {
                        continue;
                    }
                    (self.rate, self.channels) = (buf.spec().rate(), channels);
                    buf.copy_to_vec_interleaved(&mut self.chunk);
                    self.pos = 0;
                    return Ok(true);
                }
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(SymphoniaError::IoError(_)) => self.done = true,
                Err(e) => return Err(e.into()),
            }
        }
        Ok(false)
    }
}

impl Iterator for Stream {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        while self.pos >= self.chunk.len() {
            if !self.refill().unwrap_or(false) {
                return None;
            }
        }
        self.pos += 1;
        Some(self.chunk[self.pos - 1])
    }
}

/// Decodes a whole game sound (`.wav` or `.bmu`).
pub fn decode(data: &[u8]) -> Result<Pcm, AudioError> {
    let mut stream = Stream::open(Arc::from(data))?;
    let (rate, channels) = (stream.rate, stream.channels);
    let samples: Vec<f32> = stream.by_ref().collect();
    Ok(Pcm { rate, channels, samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A chunk: its tag, its size, its bytes and, if the size is odd, the
    /// pad byte RIFF puts after it.
    fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut out = tag.to_vec();
        out.extend((body.len() as u32).to_le_bytes());
        out.extend(body);
        if body.len() % 2 == 1 {
            out.push(0);
        }
        out
    }

    /// A RIFF WAVE of these chunks, its size as it should be.
    fn riff(chunks: &[Vec<u8>]) -> Vec<u8> {
        let body: Vec<u8> = chunks.concat();
        let mut out = b"RIFF".to_vec();
        out.extend((4 + body.len() as u32).to_le_bytes());
        out.extend(b"WAVE");
        out.extend(body);
        out
    }

    /// The 16 bytes of a `fmt ` chunk: format tag, channels, rate, bits.
    fn fmt_body(tag: u16, channels: u16, rate: u32, bits: u16) -> Vec<u8> {
        let align = channels * bits.div_ceil(8);
        let mut out = Vec::new();
        out.extend(tag.to_le_bytes());
        out.extend(channels.to_le_bytes());
        out.extend(rate.to_le_bytes());
        out.extend((rate * u32::from(align)).to_le_bytes());
        out.extend(align.to_le_bytes());
        out.extend(bits.to_le_bytes());
        out
    }

    /// A WAVE of a format tag, channels, rate and bits per sample holding
    /// `data` as it is.
    fn wav(tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        riff(&[chunk(b"fmt ", &fmt_body(tag, channels, rate, bits)), chunk(b"data", data)])
    }

    /// A `WAVE_FORMAT_EXTENSIBLE` WAVE (a 40-byte `fmt `) of PCM (tag 1)
    /// or float (tag 3) samples.
    fn extensible(sub: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
        let mut fmt = fmt_body(0xFFFE, channels, rate, bits);
        fmt.extend(22u16.to_le_bytes()); // the extension's size
        fmt.extend(bits.to_le_bytes()); // valid bits per sample
        fmt.extend(if channels == 1 { 4u32 } else { 3 }.to_le_bytes()); // the speakers
        fmt.extend(sub.to_le_bytes());
        fmt.extend([0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xAA, 0, 0x38, 0x9B, 0x71]);
        assert_eq!(fmt.len(), 40);
        riff(&[chunk(b"fmt ", &fmt), chunk(b"data", data)])
    }

    /// A PCM WAVE: mono, 8 kHz, 16-bit, `n` samples of a square wave.
    fn wave(n: u16) -> Vec<u8> {
        let data: Vec<u8> =
            (0..n).flat_map(|i| if i % 8 < 4 { 8000i16 } else { -8000 }.to_le_bytes()).collect();
        wav(1, 1, 8000, 16, &data)
    }

    /// Samples as fractions of full scale, left then right: the midpoint,
    /// a few between, and both ends.
    const FRAMES: [(f64, f64); 6] =
        [(0.0, 0.0), (0.5, -0.25), (1.0, -1.0), (-0.5, 0.25), (0.125, -0.125), (0.0, 0.0)];

    /// One sample of `bits` bits (or 32 for float, with `float`) as the
    /// file stores it.
    fn encode(bits: u16, float: bool, x: f64, out: &mut Vec<u8>) {
        if float {
            out.extend((x as f32).to_le_bytes());
            return;
        }
        let top = (1i64 << (bits - 1)) - 1;
        let v = ((x * (top + 1) as f64).round() as i64).clamp(-top - 1, top);
        match bits {
            8 => out.push((v + 128) as u8),
            16 => out.extend((v as i16).to_le_bytes()),
            24 => out.extend(&(v as i32).to_le_bytes()[..3]),
            _ => out.extend((v as i32).to_le_bytes()),
        }
    }

    /// `FRAMES` for `channels` channels, encoded.
    fn tone(bits: u16, float: bool, channels: u16) -> Vec<u8> {
        let mut out = Vec::new();
        for (l, r) in FRAMES {
            encode(bits, float, l, &mut out);
            if channels == 2 {
                encode(bits, float, r, &mut out);
            }
        }
        out
    }

    /// `pcm` holds `FRAMES` (to the format's precision), in the channel order.
    fn assert_frames(pcm: &Pcm, bits: u16, rate: u32, channels: u16, what: &str) {
        assert_eq!(
            (pcm.rate, pcm.channels, pcm.frames()),
            (rate, channels, FRAMES.len()),
            "{what}"
        );
        let tolerance = 1.5 / f64::from(1u32 << (bits.min(24) - 1));
        for (i, (l, r)) in FRAMES.iter().enumerate() {
            let at = i * usize::from(channels);
            let got = f64::from(pcm.samples[at]);
            assert!((got - l).abs() <= tolerance, "{what}: left {i}: {got} for {l}");
            if channels == 2 {
                let got = f64::from(pcm.samples[at + 1]);
                assert!((got - r).abs() <= tolerance, "{what}: right {i}: {got} for {r}");
            }
        }
    }

    #[test]
    fn decodes_pcm_waves() {
        let pcm = decode(&wave(800)).unwrap();
        assert_eq!((pcm.rate, pcm.channels, pcm.frames()), (8000, 1, 800));
        assert!((pcm.samples[0] - 8000.0 / 32768.0).abs() < 1e-4);
        assert!((pcm.seconds() - 0.1).abs() < 1e-6);
    }

    #[test]
    fn streams_what_decode_gives() {
        let w = wave(800);
        let s = Stream::open(Arc::from(&w[..])).unwrap();
        assert_eq!((s.rate(), s.channels()), (8000, 1));
        assert_eq!(s.collect::<Vec<f32>>(), decode(&w).unwrap().samples);
    }

    #[test]
    fn tells_the_containers_apart() {
        let m = mp3(3, false);
        let bmu = [BMU_HEADER.as_slice(), &m].concat();
        assert_eq!(container(&bmu), Some((Container::Mp3, 8..bmu.len())));
        assert_eq!(container(&m), Some((Container::Mp3, 0..m.len())));
        assert_eq!(container(&wave(4)), Some((Container::Wave, 0..wave(4).len())));
        // A bare header is no proof of MP3 data; the BMU header alone is.
        assert_eq!(container(b"\xff\xfb\x90"), None);
        assert_eq!(container(b"ID3\x04"), None);
        assert_eq!(container(b"BMU V1.0\xff\xfb"), Some((Container::Mp3, 8..10)));
        for other in [&b"OggS"[..], b"fLaC", b"just text, not a sound"] {
            assert_eq!(container(other), None);
            assert!(matches!(decode(other), Err(AudioError::Unknown)));
        }
    }

    #[test]
    fn bad_and_truncated_sounds_are_errors_not_panics() {
        assert!(matches!(decode(b"nothing"), Err(AudioError::Unknown)));
        assert!(decode(b"BMU V1.0").is_err());
        assert!(decode(b"BMU V1.0\xff\xfb\x90\x44\x00\x00").is_err());
        let w = wave(800);
        for cut in [12, 20, 40, 44, 45, 300] {
            let _ = decode(&w[..cut]);
        }
        // Cut in its data, a WAVE keeps what it has.
        let pcm = decode(&w[..44 + 200]).unwrap();
        assert_eq!(pcm.frames(), 100);
    }

    #[test]
    fn pcm_of_every_width_rate_and_channel_count_decodes() {
        // (Unsigned 8-bit, signed 16-, 24- and 32-bit, 32-bit float.)
        let formats = [(1u16, 8u16, false), (1, 16, false), (1, 24, false), (1, 32, false)]
            .into_iter()
            .chain([(3, 32, true)]);
        for (tag, bits, float) in formats {
            for rate in [8000, 11025, 22050, 44100, 48000] {
                for channels in [1u16, 2] {
                    let what = format!("tag {tag}, {bits} bits, {rate} Hz, {channels} channels");
                    let w = wav(tag, channels, rate, bits, &tone(bits, float, channels));
                    let pcm = decode(&w).unwrap_or_else(|e| panic!("{what}: {e}"));
                    assert_frames(&pcm, bits, rate, channels, &what);
                }
            }
        }
    }

    #[test]
    fn eight_bit_pcm_is_unsigned_around_128() {
        let pcm = decode(&wav(1, 1, 22050, 8, &[128, 255, 0, 192, 64])).unwrap();
        let want = [0.0, 127.0 / 128.0, -1.0, 0.5, -0.5];
        assert_eq!(pcm.samples.len(), want.len());
        for (got, want) in pcm.samples.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{got} for {want}");
        }
    }

    #[test]
    fn stereo_keeps_its_channels_in_order() {
        // Left loud and positive, right quiet and negative, in every width.
        for (bits, float) in [(8u16, false), (16, false), (24, false), (32, false), (32, true)] {
            let mut data = Vec::new();
            for _ in 0..10 {
                encode(bits, float, 0.75, &mut data);
                encode(bits, float, -0.125, &mut data);
            }
            let tag = if float { 3 } else { 1 };
            let pcm = decode(&wav(tag, 2, 44100, bits, &data)).unwrap();
            assert_eq!((pcm.channels, pcm.frames()), (2, 10), "{bits}");
            for f in pcm.samples.chunks(2) {
                assert!((f[0] - 0.75).abs() < 0.02 && (f[1] + 0.125).abs() < 0.02, "{bits}: {f:?}");
            }
        }
    }

    #[test]
    fn extensible_waves_decode() {
        for (sub, bits, float) in
            [(1u16, 16u16, false), (1, 24, false), (1, 8, false), (3, 32, true)]
        {
            for channels in [1u16, 2] {
                let what = format!("extensible {bits} bits, {channels} channels");
                let w = extensible(sub, channels, 48000, bits, &tone(bits, float, channels));
                let pcm = decode(&w).unwrap_or_else(|e| panic!("{what}: {e}"));
                assert_frames(&pcm, bits, 48000, channels, &what);
            }
        }
    }

    #[test]
    fn chunks_around_the_audio_are_skipped() {
        let fmt = chunk(b"fmt ", &fmt_body(1, 2, 44100, 16));
        let data = chunk(b"data", &tone(16, false, 2));
        let check = |w: &[u8], what: &str| {
            let pcm = decode(w).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_frames(&pcm, 16, 44100, 2, what);
        };
        // A LIST between fmt and data, as encoders write their name.
        let list = chunk(b"LIST", b"INFOISFT\x0e\0\0\0Lavf58.76.100\0");
        check(&riff(&[fmt.clone(), list.clone(), data.clone()]), "LIST between");
        // And after the data.
        check(&riff(&[fmt.clone(), data.clone(), list]), "LIST after");
        // JUNK (padding for alignment) before fmt.
        check(&riff(&[chunk(b"JUNK", &[0; 28]), fmt.clone(), data.clone()]), "JUNK before");
        // A chunk of odd size, with its pad byte, then the audio.
        let odd = chunk(b"note", b"abc");
        assert_eq!(odd.len(), 12);
        check(&riff(&[fmt.clone(), odd.clone(), data.clone()]), "odd chunk");
        check(&riff(&[odd, chunk(b"JUNK", &[1]), fmt.clone(), data.clone()]), "odd chunks first");
        // Bytes after the data that are no chunk (counted by the RIFF size, and not).
        let mut counted = riff(&[fmt.clone(), data.clone()]);
        counted.extend(b"trail");
        let size = counted.len() as u32 - 8;
        counted[4..8].copy_from_slice(&size.to_le_bytes());
        check(&counted, "trailing bytes, counted");
        let mut uncounted = riff(&[fmt.clone(), data.clone()]);
        uncounted.extend(b"trailing bytes of a tool");
        check(&uncounted, "trailing bytes, not counted");
        // What ffmpeg writes to a pipe: it cannot go back to say the sizes.
        let mut piped = riff(&[fmt, data]);
        piped[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        let at = piped.windows(4).position(|w| w == b"data").unwrap();
        piped[at + 4..at + 8].copy_from_slice(&u32::MAX.to_le_bytes());
        check(&piped, "sizes of 0xFFFFFFFF");
    }

    /// An IMA ADPCM block of one channel per 4 header bytes (predictor,
    /// step index, reserved) and then `body` nibble bytes.
    fn ima_block(predictors: &[i16], body: usize) -> Vec<u8> {
        let mut out = Vec::new();
        for p in predictors {
            out.extend(p.to_le_bytes());
            out.extend([0, 0]); // the step index, and a reserved byte
        }
        // (All-zero nibbles: each step adds an eighth of the step size.)
        out.resize(out.len() + body, 0);
        out
    }

    fn ima(channels: u16, align: u16, blocks: &[Vec<u8>]) -> Vec<u8> {
        // A `fmt ` of 20 bytes: the 16, the extension's size (2), and the
        // frames a block holds.
        let per_block = (align - 4 * channels) * 2 / channels + 1;
        let mut fmt = fmt_body(0x11, channels, 22050, 4);
        fmt[8..12]
            .copy_from_slice(&(22050 * u32::from(align) / u32::from(per_block)).to_le_bytes());
        fmt[12..14].copy_from_slice(&align.to_le_bytes());
        fmt.extend(2u16.to_le_bytes());
        fmt.extend(per_block.to_le_bytes());
        riff(&[chunk(b"fmt ", &fmt), chunk(b"data", &blocks.concat())])
    }

    #[test]
    fn ima_adpcm_decodes_a_block_to_the_frames_the_format_gives() {
        // The format's definition: a block of `align` bytes holds
        // (align - 4 * channels) * 2 / channels + 1 frames (the header's
        // predictor is the first, each nibble another). Asserted, and
        // equal to what Symphonia gives.
        let mono = ima(1, 256, &[ima_block(&[1000], 252), ima_block(&[-2000], 252)]);
        let pcm = decode(&mono).unwrap();
        assert_eq!((pcm.rate, pcm.channels), (22050, 1));
        assert_eq!(pcm.frames(), 2 * ((256 - 4) * 2 + 1));
        // (A block starts at its predictor.)
        assert!((pcm.samples[0] - 1000.0 / 32768.0).abs() < 1e-4, "{}", pcm.samples[0]);
        assert!((pcm.samples[505] + 2000.0 / 32768.0).abs() < 1e-4, "{}", pcm.samples[505]);
        // Stereo: the header of each channel, then 8 nibbles of each in turn.
        let stereo = ima(2, 512, &[ima_block(&[3000, -3000], 504)]);
        let pcm = decode(&stereo).unwrap();
        assert_eq!((pcm.rate, pcm.channels), (22050, 2));
        assert_eq!(pcm.frames(), (512 - 8) * 2 / 2 + 1);
        assert!((pcm.samples[0] - 3000.0 / 32768.0).abs() < 1e-4);
        assert!((pcm.samples[1] + 3000.0 / 32768.0).abs() < 1e-4);
    }

    /// The game reads a sound by what it holds, not by its name: PCM that a
    /// builder renamed `.bmu` is the same bytes as the `.wav`, which is
    /// what the decoder is given, and there is no name at this layer.
    #[test]
    fn pcm_kept_under_the_name_bmu_is_a_wave() {
        let bytes = wav(1, 2, 44100, 16, &tone(16, false, 2));
        assert_eq!(container(&bytes), Some((Container::Wave, 0..bytes.len())));
        let pcm = decode(&bytes).unwrap();
        assert_frames(&pcm, 16, 44100, 2, "bmu");
    }

    #[test]
    fn a_pcm_wave_cut_anywhere_is_an_answer_not_a_panic() {
        let pcm = wav(1, 2, 44100, 16, &tone(16, false, 2));
        let ext = extensible(1, 2, 44100, 24, &tone(24, false, 2));
        for (what, w) in [("pcm", pcm), ("extensible", ext)] {
            for cut in 0..=w.len() {
                let _ = decode(&w[..cut]);
                let _ = container(&w[..cut]);
            }
            assert!(decode(&w).is_ok(), "{what}");
        }
    }

    // ---- MP3 and the forms the game plays it in ----

    /// `frames` silent MPEG-1 layer III frames at 128 kbit/s and 44100 Hz
    /// (417 bytes each: a valid header, and zeros for the side information
    /// and the main data), mono or stereo.
    fn mp3(frames: usize, stereo: bool) -> Vec<u8> {
        let header = if stereo { [0xFF, 0xFB, 0x90, 0x00] } else { [0xFF, 0xFB, 0x90, 0xC4] };
        let mut frame = header.to_vec();
        frame.resize(417, 0);
        frame.repeat(frames)
    }

    /// Frames per MP3 frame (MPEG-1 layer III).
    const MP3_FRAME: usize = 1152;

    /// `pcm` is `n` MP3 frames of silence, 44100 Hz. The decoder may hold
    /// back or drop a frame at either end (the bit reservoir, a gap), so a
    /// frame either way is allowed; nothing more.
    fn assert_mp3(pcm: &Pcm, n: usize, channels: u16, what: &str) {
        assert_eq!((pcm.rate, pcm.channels), (44100, channels), "{what}");
        assert!(
            pcm.frames().abs_diff(n * MP3_FRAME) <= MP3_FRAME,
            "{what}: {} frames",
            pcm.frames()
        );
        assert!(pcm.samples.iter().all(|s| s.abs() < 1e-6), "{what}: not silent");
    }

    /// The 30-byte `fmt ` of an MPEGLAYER3 WAVE.
    fn mpeglayer3_fmt(channels: u16) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(0x55u16.to_le_bytes());
        out.extend(channels.to_le_bytes());
        out.extend(44100u32.to_le_bytes());
        out.extend(16000u32.to_le_bytes()); // bytes per second
        out.extend(1u16.to_le_bytes()); // block align
        out.extend(0u16.to_le_bytes()); // bits per sample
        out.extend(12u16.to_le_bytes()); // cbSize
        out.extend(1u16.to_le_bytes()); // wID: MPEGLAYER3_ID_MPEG
        out.extend(2u32.to_le_bytes()); // fdwFlags: padding off
        out.extend(417u16.to_le_bytes()); // nBlockSize
        out.extend(1u16.to_le_bytes()); // nFramesPerBlock
        out.extend(1393u16.to_le_bytes()); // nCodecDelay
        assert_eq!(out.len(), 30);
        out
    }

    /// An MP3 in a RIFF WAVE, with a `fact` before the data and a `LIST`
    /// (which holds MPEG-looking bytes) after it.
    fn mp3_in_wave(frames: usize) -> Vec<u8> {
        riff(&[
            chunk(b"fmt ", &mpeglayer3_fmt(1)),
            chunk(b"fact", &((frames * MP3_FRAME) as u32).to_le_bytes()),
            chunk(b"data", &mp3(frames, false)),
            chunk(b"LIST", &mp3(2, false)),
        ])
    }

    /// An ID3v2.3 tag of `size` bytes of payload.
    fn id3(size: usize) -> Vec<u8> {
        let mut out = b"ID3\x03\x00\x00".to_vec();
        out.extend([(size >> 21) as u8 & 0x7F, (size >> 14) as u8 & 0x7F]);
        out.extend([(size >> 7) as u8 & 0x7F, size as u8 & 0x7F]);
        out.resize(10 + size, 0);
        out
    }

    #[test]
    fn authored_mp3_decodes() {
        for (stereo, channels) in [(false, 1), (true, 2)] {
            let pcm = decode(&mp3(10, stereo)).unwrap();
            assert_mp3(&pcm, 10, channels, "bare");
        }
    }

    #[test]
    fn finds_where_the_mp3_starts() {
        let m = mp3(3, false);
        assert_eq!(find_mp3(&m, 0, m.len()), Some(0));
        let tagged = [id3(300), m.clone()].concat();
        assert_eq!(find_mp3(&tagged, 0, tagged.len()), Some(310));
        // Two tags in a row; a footer adds ten.
        let mut footer = id3(5);
        footer[5] = 0x10;
        footer.extend(b"3DI\x03\x00\x00\x00\x00\x00\x05"); // the footer
        let two = [footer.clone(), id3(7), m.clone()].concat();
        assert_eq!(find_mp3(&two, 0, two.len()), Some(25 + 17));
        let junk = [vec![0u8; 1024], m.clone()].concat();
        assert_eq!(find_mp3(&junk, 0, junk.len()), Some(1024));
        // A false sync is passed over; a lone frame is not confirmed...
        let mut falsy = vec![0x55u8; 500];
        falsy[10..14].copy_from_slice(&[0xFF, 0xFB, 0x90, 0xC4]);
        let false_then_real = [falsy.clone(), m.clone()].concat();
        assert_eq!(find_mp3(&false_then_real, 0, false_then_real.len()), Some(500));
        assert_eq!(find_mp3(&falsy, 0, falsy.len()), None);
        // ...unless it ends the data; a frame cut short is not one.
        assert_eq!(find_mp3(&m[..417], 0, 417), Some(0));
        assert_eq!(find_mp3(&m[..300], 0, 300), None);
        // Different sample rates are not confirmation: 48000 Hz after 44100.
        let mut other = mp3(2, false);
        other[417 + 2] = 0x94;
        assert_eq!(find_mp3(&other, 0, other.len()), None);
        // The search stops a mebibyte on.
        let far = [vec![0u8; (1 << 20) + 5], m.clone()].concat();
        assert_eq!(find_mp3(&far, 0, far.len()), None);
        let near = [vec![0u8; (1 << 20) - 5], m].concat();
        assert_eq!(find_mp3(&near, 0, near.len()), Some((1 << 20) - 5));
        // Start beyond the data, and an end beyond it, are answers.
        assert_eq!(find_mp3(b"abc", 10, 20), None);
        assert_eq!(find_mp3(b"abc", 0, usize::MAX), None);
    }

    #[test]
    fn frame_lengths_follow_the_headers() {
        let len = |h: [u8; 4]| mp3_frame(&h).map(|f| f.0);
        assert_eq!(len([0xFF, 0xFB, 0x90, 0x00]), Some(417)); // MPEG-1 III 128k 44.1k
        assert_eq!(len([0xFF, 0xFB, 0x92, 0x00]), Some(418)); // padded
        assert_eq!(len([0xFF, 0xF3, 0x90, 0x00]), Some(72 * 80000 / 22050)); // MPEG-2 III 80k
        assert_eq!(len([0xFF, 0xFD, 0x90, 0x00]), Some(144 * 160000 / 44100)); // MPEG-1 II
        assert_eq!(len([0xFF, 0xFF, 0x90, 0x00]), Some((12 * 288000 / 44100) * 4)); // I
        for bad in [
            [0xFF, 0xFB, 0x00, 0x00], // free format
            [0xFF, 0xFB, 0xF0, 0x00], // bitrate 15
            [0xFF, 0xFB, 0x9C, 0x00], // sample rate 3
            [0xFF, 0xF9, 0x90, 0x00], // layer 0
            [0xFF, 0xE9, 0x90, 0x00], // reserved version
            [0xFF, 0x1B, 0x90, 0x00], // no sync
        ] {
            assert_eq!(len(bad), None, "{bad:02x?}");
        }
    }

    #[test]
    fn mp3_in_the_forms_the_game_plays_decodes() {
        let m = mp3(8, false);
        let junk: Vec<u8> = (0..1024u32).map(|i| (i * 7 % 251) as u8 | 1).collect();
        let mut false_sync = junk.clone();
        false_sync[100..104].copy_from_slice(&[0xFF, 0xFB, 0x90, 0xC4]);
        false_sync[300..302].copy_from_slice(&[0xFF, 0xFB]);
        let bare: Vec<(&str, Vec<u8>)> = vec![
            ("plain", m.clone()),
            ("small tag", [id3(2000), m.clone()].concat()),
            ("2 MiB tag", [id3(2 << 20), m.clone()].concat()),
            ("1 MiB tag", [id3(1 << 20), m.clone()].concat()),
            ("zeros", [vec![0u8; 1024], m.clone()].concat()),
            ("junk with a false sync", [false_sync, m.clone()].concat()),
        ];
        for (what, body) in bare {
            let pcm = decode(&body).unwrap_or_else(|e| panic!("{what}: {e}"));
            assert_mp3(&pcm, 8, 1, what);
            let bmu = [BMU_HEADER.as_slice(), &body].concat();
            let pcm = decode(&bmu).unwrap_or_else(|e| panic!("BMU + {what}: {e}"));
            assert_mp3(&pcm, 8, 1, &format!("BMU + {what}"));
        }
    }

    #[test]
    fn mp3_inside_a_wave_decodes_from_its_data_chunk() {
        let w = mp3_in_wave(8);
        let (kind, range) = container(&w).unwrap();
        assert_eq!(kind, Container::Mp3);
        assert_eq!(range.len(), 8 * 417, "the data chunk, not the LIST after it");
        let pcm = decode(&w).unwrap();
        assert_mp3(&pcm, 8, 1, "mp3 in wave");
        // Without the extras, and with an ID3 tag inside the data.
        let plain = riff(&[chunk(b"fmt ", &mpeglayer3_fmt(1)), chunk(b"data", &mp3(8, false))]);
        assert_mp3(&decode(&plain).unwrap(), 8, 1, "plain");
        let tagged = riff(&[
            chunk(b"fmt ", &mpeglayer3_fmt(1)),
            chunk(b"data", &[id3(50), mp3(8, false)].concat()),
        ]);
        assert_mp3(&decode(&tagged).unwrap(), 8, 1, "tagged");
        // Its sizes need not be right, either.
        let mut piped = w.clone();
        piped[4..8].copy_from_slice(&0u32.to_le_bytes());
        assert_mp3(&decode(&piped).unwrap(), 8, 1, "RIFF size 0");
    }

    #[test]
    fn mp3_in_a_wave_streams_what_decode_gives() {
        let w = mp3_in_wave(6);
        let s = Stream::open(Arc::from(&w[..])).unwrap();
        assert_eq!((s.rate(), s.channels()), (44100, 1));
        assert_eq!(s.collect::<Vec<f32>>(), decode(&w).unwrap().samples);
    }

    #[test]
    fn a_wave_with_wrong_sizes_decodes() {
        let tone = tone(16, false, 2);
        let fmt = chunk(b"fmt ", &fmt_body(1, 2, 44100, 16));
        let good = riff(&[fmt.clone(), chunk(b"data", &tone)]);
        assert_frames(&decode(&good).unwrap(), 16, 44100, 2, "good");
        let at = good.windows(4).position(|w| w == b"data").unwrap();
        for (what, riff_size, data_size) in [
            ("RIFF size 0", Some(0u32), None),
            ("RIFF size too small", Some(30), None),
            ("data size past the RIFF size", None, Some(tone.len() as u32 + 1000)),
            ("both wrong", Some(4), Some(u32::MAX)),
        ] {
            let mut w = good.clone();
            if let Some(n) = riff_size {
                w[4..8].copy_from_slice(&n.to_le_bytes());
            }
            if let Some(n) = data_size {
                w[at + 4..at + 8].copy_from_slice(&n.to_le_bytes());
            }
            assert_frames(
                &decode(&w).unwrap_or_else(|e| panic!("{what}: {e}")),
                16,
                44100,
                2,
                what,
            );
        }
    }

    #[test]
    fn a_well_formed_wave_is_not_copied() {
        let w = wave(100);
        let info = WaveInfo::read(&w);
        assert!(!info.riff_size_is_wrong(&w));
        // A truncated file keeps its sizes and is not repaired either.
        assert!(!WaveInfo::read(&w[..60]).riff_size_is_wrong(&w[..60]));
        let mut zero = w.clone();
        zero[4..8].copy_from_slice(&[0; 4]);
        assert!(info.riff_size_is_wrong(&zero));
    }

    #[test]
    fn what_moonglow_does_not_play_is_refused_by_name() {
        let text = AudioError::Unknown.to_string();
        assert_eq!(text, "not a sound the game plays (neither WAVE nor MP3)");
        let refused = |b: &[u8]| decode(b).unwrap_err().to_string();
        assert_eq!(refused(b"just some text, not a sound"), text);
        assert_eq!(refused(b"OggS\0\x02\0\0\0\0\0\0\0\0\0\0\0\0"), text);
        assert_eq!(refused(b"fLaC\0\0\0\x22 and the rest"), text);
        assert_eq!(
            refused(b"RF64\xff\xff\xff\xffWAVEds64"),
            "an RF64 WAVE, which the game does not play"
        );
        assert_eq!(
            refused(b"BW64\xff\xff\xff\xffWAVEds64"),
            "an RF64 WAVE, which the game does not play"
        );
        let body = [0u8; 400];
        assert_eq!(
            refused(&wav(0x50, 2, 44100, 0, &body)),
            "a WAVE of a format Moonglow does not play (format 0x0050, MPEG layer 1 or 2)"
        );
        assert_eq!(
            refused(&wav(0x161, 2, 44100, 16, &body)),
            "a WAVE of a format Moonglow does not play (format 0x0161, Windows Media Audio)"
        );
        assert_eq!(
            refused(&wav(0xFF, 2, 44100, 16, &body)),
            "a WAVE of a format Moonglow does not play (format 0x00FF, AAC)"
        );
        assert_eq!(
            refused(&wav(0x1234, 2, 44100, 16, &body)),
            "a WAVE of a format Moonglow does not play (format 0x1234)"
        );
        assert!(
            refused(&wav(0x674F, 2, 44100, 16, &body)).ends_with("(format 0x674F, Ogg Vorbis)")
        );
        // A data chunk of size 0 with data after it is not guessed at.
        let mut empty = wav(1, 2, 44100, 16, &tone(16, false, 2));
        let at = empty.windows(4).position(|w| w == b"data").unwrap();
        empty[at + 4..at + 8].copy_from_slice(&0u32.to_le_bytes());
        assert!(decode(&empty).is_err());
    }

    #[test]
    fn cut_or_garbled_mp3_forms_are_answers_not_panics() {
        let in_wave = mp3_in_wave(3);
        let tagged = [id3(40), mp3(3, false)].concat();
        let mut repaired = wave(100);
        repaired[4..8].copy_from_slice(&[0; 4]);
        for (what, bytes) in [("wave", in_wave), ("tagged", tagged), ("repaired", repaired)] {
            for cut in 0..=bytes.len() {
                let _ = decode(&bytes[..cut]);
                let _ = container(&bytes[..cut]);
            }
            assert!(decode(&bytes).is_ok(), "{what}");
        }
        // Tags that claim too much, or everything.
        for size in [[0x7F; 4], [0, 0, 0x7F, 0x7F], [0, 0, 0, 0]] {
            let mut tag = b"ID3\x04\x00\x10".to_vec();
            tag.extend(size);
            tag.extend(mp3(2, false));
            let _ = decode(&tag);
            let _ = decode(&tag[..12]);
            let _ = decode(&[BMU_HEADER.as_slice(), &tag].concat());
        }
        // Chunk sizes of 0xFFFFFFFF everywhere.
        let mut huge = b"RIFF".to_vec();
        huge.extend([0xFF; 4]);
        huge.extend(b"WAVE");
        for tag in [b"JUNK", b"fmt ", b"data"] {
            huge.extend(tag);
            huge.extend([0xFF; 4]);
            huge.extend(fmt_body(0x55, 1, 44100, 0));
        }
        let _ = decode(&huge);
        let _ = decode(&huge[..30]);
    }

    #[test]
    fn noise_that_looks_like_frames_is_quick_to_refuse() {
        let ff = vec![0xFFu8; 2 << 20];
        let hdr: Vec<u8> = [0xFF, 0xFB, 0x90, 0xC4].repeat((2 << 20) / 4);
        for (what, bytes) in [("FF", ff), ("headers", hdr)] {
            let t = std::time::Instant::now();
            assert!(decode(&bytes).is_err(), "{what}");
            let bmu = [BMU_HEADER.as_slice(), &bytes].concat();
            assert!(decode(&bmu).is_err(), "BMU + {what}");
            assert!(t.elapsed() < std::time::Duration::from_secs(2), "{what}: {:?}", t.elapsed());
        }
    }
}
