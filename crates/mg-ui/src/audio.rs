//! Sound playback. Moonglow's previews (a sound blueprint's Play, a
//! conversation line's sound, an area's ambient sound, music and placed
//! sounds) go to a [`Speaker`]: the desktop app's speakers, or [`Silence`]
//! (tests, or no sound device).

use std::collections::HashMap;
use std::sync::Arc;

use mg_core::{ResRef, ResType};
use mg_resman::ResKey;

use crate::Moonglow;

/// What plays: one sound at a time on each channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    /// A Play button's preview.
    Preview,
    /// The area's ambient sound.
    Ambient,
    /// The area's music.
    Music,
    /// A placed sound object, by its place in the area's list.
    Placed(usize),
}

/// A sound to play: its name and its file (`.wav` or `.bmu`, as
/// [`mg_audio::Stream`] decodes them).
#[derive(Clone)]
pub struct Sound {
    pub name: ResRef,
    pub data: Arc<[u8]>,
}

impl std::fmt::Debug for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Sound({}, {} bytes)", self.name, self.data.len())
    }
}

/// Where sounds go.
pub trait Speaker {
    /// Plays a sound on a channel, replacing what it plays, at a volume
    /// (0 to 1), once or over and over.
    fn play(&mut self, channel: Channel, sound: Sound, volume: f32, looped: bool);
    fn set_volume(&mut self, channel: Channel, volume: f32);
    fn stop(&mut self, channel: Channel);
    /// Whether a channel is still playing.
    fn playing(&self, channel: Channel) -> bool;
}

/// No speakers: what would play on each channel (name, volume, looped).
#[derive(Debug, Default)]
pub struct Silence {
    pub channels: HashMap<Channel, (ResRef, f32, bool)>,
    /// The file each channel was given.
    pub data: HashMap<Channel, Arc<[u8]>>,
}

impl Speaker for Silence {
    fn play(&mut self, channel: Channel, sound: Sound, volume: f32, looped: bool) {
        self.data.insert(channel, sound.data);
        self.channels.insert(channel, (sound.name, volume, looped));
    }

    fn set_volume(&mut self, channel: Channel, volume: f32) {
        if let Some(c) = self.channels.get_mut(&channel) {
            c.1 = volume;
        }
    }

    fn stop(&mut self, channel: Channel) {
        self.data.remove(&channel);
        self.channels.remove(&channel);
    }

    fn playing(&self, channel: Channel) -> bool {
        self.channels.contains_key(&channel)
    }
}

/// A shared [`Silence`], so a test can look at what plays.
impl Speaker for std::rc::Rc<std::cell::RefCell<Silence>> {
    fn play(&mut self, channel: Channel, sound: Sound, volume: f32, looped: bool) {
        self.borrow_mut().play(channel, sound, volume, looped);
    }

    fn set_volume(&mut self, channel: Channel, volume: f32) {
        self.borrow_mut().set_volume(channel, volume);
    }

    fn stop(&mut self, channel: Channel) {
        self.borrow_mut().stop(channel);
    }

    fn playing(&self, channel: Channel) -> bool {
        self.borrow().playing(channel)
    }
}

/// What a sound is for, as far as the game's lookup and playing go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    /// An `ambientmusic.2da` row.
    Music,
    /// An `ambientsound.2da` row.
    Ambient,
    /// Any other sound: a Play button's, a placed sound's.
    Other,
    /// A file shown in the hak editor's View, which has no role.
    Viewed,
}

impl Role {
    pub fn of(channel: Channel) -> Role {
        match channel {
            Channel::Music => Role::Music,
            Channel::Ambient => Role::Ambient,
            Channel::Preview | Channel::Placed(_) => Role::Other,
        }
    }

    /// The types the game looks for, in order (Moonglow also plays what the
    /// game never finds: music as `.wav`, a sound as `.bmu`).
    fn lookup(self) -> [ResType; 2] {
        match self {
            Role::Music => [ResType::BMU, ResType::WAV],
            _ => [ResType::WAV, ResType::BMU],
        }
    }
}

/// Why the game would stay silent on a file that Moonglow plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Silent {
    /// Music that is a WAVE under `.bmu`.
    MusicWave,
    /// Music that exists only as `.wav`.
    MusicOnlyWav,
    /// An ambient sound that is MP3 under `.wav`.
    AmbientMp3,
    /// An ambient sound that is a WAVE under `.bmu`.
    AmbientWave,
    /// A sound that exists only as `.bmu`.
    OnlyBmu,
    /// A WAVE under `.bmu`, seen without a role: played in none.
    ViewedWave,
}

impl Silent {
    /// The log's line about `name`.
    pub fn message(self, name: ResRef) -> String {
        match self {
            Silent::MusicWave => format!(
                "Music {name} is a WAVE under .bmu, which the game does not play: music must be MP3"
            ),
            Silent::MusicOnlyWav => format!(
                "Music {name} exists only as a .wav, which the game does not look for: music must be a .bmu"
            ),
            Silent::AmbientMp3 => format!(
                "Ambient sound {name} is MP3 under .wav, which the game does not play: it plays a WAVE there"
            ),
            Silent::AmbientWave => format!(
                "Ambient sound {name} is a WAVE under .bmu, which the game does not play: it plays MP3 there"
            ),
            Silent::OnlyBmu => format!(
                "Sound {name} exists only as a .bmu, which the game does not look for: sounds must be a .wav"
            ),
            Silent::ViewedWave => format!(
                "{name}.bmu is a WAVE, which the game does not play under .bmu: music must be MP3"
            ),
        }
    }
}

/// Whether the game stays silent on a file it would not play in a role,
/// and why: `found` is the type the file was found as (by the lookup in
/// [`Role::lookup`] order, so for music a `.wav` means there is no `.bmu`,
/// and for the others a `.bmu` means there is no `.wav`).
///
/// The client findings (`docs/findings.md`, "What the game plays as music
/// and as sounds"):
/// - music looks only for `NAME.bmu` and plays it if it holds MP3;
/// - an ambient sound looks for `.wav`, else `.bmu`; a `.wav` plays if it
///   holds a WAVE, a `.bmu` if it holds MP3;
/// - other sounds look only for `NAME.wav`.
pub fn silent_in_game(
    role: Role,
    found: ResType,
    container: mg_audio::Container,
) -> Option<Silent> {
    use mg_audio::Container::{Mp3, Wave};
    match (role, found, container) {
        // The client never finds a music track named .wav.
        (Role::Music, ResType::WAV, _) => Some(Silent::MusicOnlyWav),
        // It finds the .bmu and plays it only as MP3.
        (Role::Music, ResType::BMU, Wave) => Some(Silent::MusicWave),
        (Role::Music, ..) => None,
        // MP3 under .wav is silent, a WAVE plays.
        (Role::Ambient, ResType::WAV, Mp3) => Some(Silent::AmbientMp3),
        // Only a .bmu with MP3 plays as an ambient sound.
        (Role::Ambient, ResType::BMU, Wave) => Some(Silent::AmbientWave),
        (Role::Ambient, ..) => None,
        // Only NAME.wav is looked for (measured for PlaySound).
        (Role::Other, ResType::BMU, _) => Some(Silent::OnlyBmu),
        (Role::Other, ..) => None,
        // A WAVE under .bmu is played in no role.
        (Role::Viewed, ResType::BMU, Wave) => Some(Silent::ViewedWave),
        (Role::Viewed, ..) => None,
    }
}

impl Moonglow {
    /// A sound's file for a role: the module's, else the game's, for each
    /// type in the order the game looks for them. The type it was found as
    /// too.
    pub fn sound_file(&self, role: Role, name: ResRef) -> Option<(ResType, Arc<[u8]>)> {
        role.lookup().into_iter().find_map(|t| {
            let key = ResKey::new(name, t);
            let module = self.ws.as_ref().and_then(|ws| ws.module.get(&key)).map(Arc::from);
            let data = module
                .or_else(|| self.game.as_deref()?.resman.get(&key).ok().map(|d| Arc::from(&*d)))?;
            Some((t, data))
        })
    }

    /// Tells the log that the game is silent on a file, once for each file
    /// and role (a replaced file, which has another length, is judged anew).
    /// Whether it did.
    pub(crate) fn warn_silent(
        &mut self,
        role: Role,
        name: ResRef,
        found: ResType,
        data: &[u8],
    ) -> bool {
        let Some((container, _)) = mg_audio::container(data) else { return false };
        let Some(reason) = silent_in_game(role, found, container) else { return false };
        if self.sound_said.insert((role, name, found, data.len())) {
            self.log.warn(reason.message(name));
            return true;
        }
        false
    }

    /// Plays a sound on a channel; one that is missing or does not decode is
    /// reported in the log, and so is one that plays here and not in the
    /// game. Whether it plays.
    pub fn play_sound(
        &mut self,
        channel: Channel,
        name: ResRef,
        volume: f32,
        looped: bool,
    ) -> bool {
        if name.is_empty() {
            return false;
        }
        let role = Role::of(channel);
        let Some((found, data)) = self.sound_file(role, name) else {
            self.log.warn(format!("Sound {name} not found"));
            return false;
        };
        if let Err(e) = mg_audio::Stream::open(data.clone()) {
            self.log.warn(format!("Sound {name}: {e}"));
            return false;
        }
        self.warn_silent(role, name, found, &data);
        self.speaker.play(channel, Sound { name, data }, volume.clamp(0.0, 1.0), looped);
        true
    }

    pub fn stop_sound(&mut self, channel: Channel) {
        self.speaker.stop(channel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const BMU: ResType = ResType::BMU;
    const WAV: ResType = ResType::WAV;
    use Role::*;
    use mg_audio::Container::{Mp3, Wave};

    /// The whole matrix: three roles (and the viewer) by two types by two
    /// containers.
    #[test]
    fn the_game_is_silent_where_the_client_was() {
        let cases = [
            // Music: only NAME.bmu is looked for, and plays as MP3.
            (Music, BMU, Mp3, None),
            (Music, BMU, Wave, Some(Silent::MusicWave)),
            (Music, WAV, Mp3, Some(Silent::MusicOnlyWav)),
            (Music, WAV, Wave, Some(Silent::MusicOnlyWav)),
            // Ambient: .wav (else .bmu); a .wav plays as a WAVE, a .bmu as MP3.
            (Ambient, WAV, Wave, None),
            (Ambient, WAV, Mp3, Some(Silent::AmbientMp3)),
            (Ambient, BMU, Mp3, None),
            (Ambient, BMU, Wave, Some(Silent::AmbientWave)),
            // Other sounds: only NAME.wav is looked for.
            (Other, WAV, Wave, None),
            (Other, WAV, Mp3, None),
            (Other, BMU, Mp3, Some(Silent::OnlyBmu)),
            (Other, BMU, Wave, Some(Silent::OnlyBmu)),
            // Viewed: a WAVE under .bmu plays in no role.
            (Viewed, BMU, Wave, Some(Silent::ViewedWave)),
            (Viewed, BMU, Mp3, None),
            (Viewed, WAV, Wave, None),
            (Viewed, WAV, Mp3, None),
        ];
        for (role, found, container, want) in cases {
            assert_eq!(
                silent_in_game(role, found, container),
                want,
                "{role:?} {found:?} {container:?}"
            );
        }
    }

    #[test]
    fn music_looks_for_the_bmu_first() {
        assert_eq!(Music.lookup(), [BMU, WAV]);
        assert_eq!(Ambient.lookup(), [WAV, BMU]);
        assert_eq!(Other.lookup(), [WAV, BMU]);
    }

    #[test]
    fn the_messages_are_one_sentence_each() {
        let name = ResRef::from_str("mg_track").unwrap();
        for r in [
            Silent::MusicWave,
            Silent::MusicOnlyWav,
            Silent::AmbientMp3,
            Silent::AmbientWave,
            Silent::OnlyBmu,
            Silent::ViewedWave,
        ] {
            let m = r.message(name);
            assert!(m.contains("mg_track") && !m.contains(';') && !m.ends_with('.'), "{m}");
        }
    }
}
