//! The speakers: Moonglow's sounds through rodio, decoded as they play.

use std::collections::HashMap;
use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use mg_ui::audio::{Channel, Sound, Speaker};
use rodio::{ChannelCount, SampleRate};

/// A game sound decoded as it plays, from the start again if looped.
struct Playing {
    data: Arc<[u8]>,
    stream: mg_audio::Stream,
    looped: bool,
    rate: SampleRate,
    channels: ChannelCount,
}

impl Iterator for Playing {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if let Some(s) = self.stream.next() {
            return Some(s);
        }
        if !self.looped {
            return None;
        }
        self.stream = mg_audio::Stream::open(self.data.clone()).ok()?;
        self.stream.next()
    }
}

impl rodio::Source for Playing {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.rate
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// The default output device, a player per channel.
pub(crate) struct Speakers {
    sink: rodio::MixerDeviceSink,
    players: HashMap<Channel, rodio::Player>,
}

impl Speakers {
    /// The default output device, if there is one.
    pub(crate) fn open() -> Option<Speakers> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink().ok()?;
        sink.log_on_drop(false);
        Some(Speakers { sink, players: HashMap::new() })
    }
}

impl Speaker for Speakers {
    fn play(&mut self, channel: Channel, sound: Sound, volume: f32, looped: bool) {
        self.stop(channel);
        let Ok(stream) = mg_audio::Stream::open(sound.data.clone()) else { return };
        let (Some(rate), Some(channels)) =
            (NonZero::new(stream.rate()), NonZero::new(stream.channels()))
        else {
            return;
        };
        let player = rodio::Player::connect_new(self.sink.mixer());
        player.set_volume(volume);
        player.append(Playing { data: sound.data, stream, looped, rate, channels });
        self.players.insert(channel, player);
    }

    fn set_volume(&mut self, channel: Channel, volume: f32) {
        if let Some(p) = self.players.get(&channel) {
            p.set_volume(volume);
        }
    }

    fn stop(&mut self, channel: Channel) {
        if let Some(p) = self.players.remove(&channel) {
            p.stop();
        }
    }

    fn playing(&self, channel: Channel) -> bool {
        self.players.get(&channel).is_some_and(|p| !p.empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PCM WAVE: mono, 8 kHz, 16-bit, 80 samples.
    fn wave() -> Arc<[u8]> {
        let data: Vec<u8> = (0..80i16).flat_map(|i| (i * 100).to_le_bytes()).collect();
        let mut out = b"RIFF".to_vec();
        out.extend((36 + data.len() as u32).to_le_bytes());
        out.extend(b"WAVEfmt ");
        for v in [16u32, 1 | (1 << 16), 8000, 16000, 2 | (16 << 16)] {
            out.extend(v.to_le_bytes());
        }
        out.extend(b"data");
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(data);
        out.into()
    }

    #[test]
    fn a_looped_sound_starts_again() {
        let play = |looped| {
            let data = wave();
            let stream = mg_audio::Stream::open(data.clone()).unwrap();
            let (rate, channels) = (NonZero::new(8000).unwrap(), NonZero::new(1).unwrap());
            Playing { data, stream, looped, rate, channels }
        };
        assert_eq!(play(false).count(), 80);
        let looped: Vec<f32> = play(true).take(200).collect();
        assert_eq!(looped.len(), 200);
        assert_eq!(looped[80..160], looped[..80], "the second pass is the first");
    }
}
