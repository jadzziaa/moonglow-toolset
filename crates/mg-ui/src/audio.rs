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
}

impl Speaker for Silence {
    fn play(&mut self, channel: Channel, sound: Sound, volume: f32, looped: bool) {
        self.channels.insert(channel, (sound.name, volume, looped));
    }

    fn set_volume(&mut self, channel: Channel, volume: f32) {
        if let Some(c) = self.channels.get_mut(&channel) {
            c.1 = volume;
        }
    }

    fn stop(&mut self, channel: Channel) {
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

impl Moonglow {
    /// A sound's file: the module's, else the game's (`.wav`, then `.bmu`).
    pub fn sound_data(&self, name: ResRef) -> Option<Arc<[u8]>> {
        [ResType::WAV, ResType::BMU].into_iter().find_map(|t| {
            let key = ResKey::new(name, t);
            let module = self.ws.as_ref().and_then(|ws| ws.module.get(&key)).map(Arc::from);
            module.or_else(|| self.game.as_deref()?.resman.get(&key).ok().map(|d| Arc::from(&*d)))
        })
    }

    /// Plays a sound on a channel; one that is missing or does not decode is
    /// reported in the log. Whether it plays.
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
        let Some(data) = self.sound_data(name) else {
            self.log.warn(format!("Sound {name} not found"));
            return false;
        };
        if let Err(e) = mg_audio::Stream::open(data.clone()) {
            self.log.warn(format!("Sound {name}: {e}"));
            return false;
        }
        self.speaker.play(channel, Sound { name, data }, volume.clamp(0.0, 1.0), looped);
        true
    }

    pub fn stop_sound(&mut self, channel: Channel) {
        self.speaker.stop(channel);
    }
}
