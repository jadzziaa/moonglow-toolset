//! The game's sounds and music, decoded to PCM for playback. What the game
//! ships under the names `.wav` and `.bmu` (nwn.wiki, "Sounds and Music";
//! the base game's 12,800 sounds counted in `audio.rs`): MP3 behind an
//! eight-byte `BMU V1.0` header (most sounds; music without it), RIFF WAVE
//! with IMA ADPCM or PCM, and a few bare MP3s. Decoding is Symphonia's.

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
    /// MP3 data (after a `BMU V1.0` header, if any).
    Mp3,
    /// RIFF WAVE.
    Wave,
}

/// Why a sound could not be decoded.
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("not a sound the game plays (neither WAVE nor MP3)")]
    Unknown,
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

/// What a sound file holds, and where its data starts.
pub fn container(data: &[u8]) -> Option<(Container, usize)> {
    if data.starts_with(BMU_HEADER) {
        return Some((Container::Mp3, BMU_HEADER.len()));
    }
    if data.len() >= 12 && &data[..4] == b"RIFF" && &data[8..12] == b"WAVE" {
        return Some((Container::Wave, 0));
    }
    let sync = data.len() >= 2 && data[0] == 0xff && data[1] & 0xe0 == 0xe0;
    (data.starts_with(b"ID3") || sync).then_some((Container::Mp3, 0))
}

/// A sound's bytes from where its audio starts.
#[derive(Clone)]
struct Bytes {
    data: Arc<[u8]>,
    start: usize,
}

impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        &self.data[self.start..]
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
        let (kind, start) = container(&data).ok_or(AudioError::Unknown)?;
        let source = Cursor::new(Bytes { data, start });
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

    /// A PCM WAVE: mono, 8 kHz, 16-bit, `n` samples of a square wave.
    fn wave(n: u16) -> Vec<u8> {
        let data: Vec<u8> =
            (0..n).flat_map(|i| if i % 8 < 4 { 8000i16 } else { -8000 }.to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        out.extend(16u32.to_le_bytes());
        out.extend(1u16.to_le_bytes()); // PCM
        out.extend(1u16.to_le_bytes()); // mono
        out.extend(8000u32.to_le_bytes());
        out.extend(16000u32.to_le_bytes());
        out.extend(2u16.to_le_bytes());
        out.extend(16u16.to_le_bytes());
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out
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
        assert_eq!(container(b"BMU V1.0\xff\xfb"), Some((Container::Mp3, 8)));
        assert_eq!(container(&wave(4)), Some((Container::Wave, 0)));
        assert_eq!(container(b"ID3\x04"), Some((Container::Mp3, 0)));
        assert_eq!(container(b"\xff\xfb\x90"), Some((Container::Mp3, 0)));
        assert_eq!(container(b"OggS"), None);
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
}
