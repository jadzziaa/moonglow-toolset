//! An area's sounds while its view shows, as Aurora's Play Ambient Sound,
//! Play Ambient Music and Play Placed Sounds (Options › Sounds; Aurora's
//! defaults: placed sounds on, ambient sound and music off, music at
//! 92/127). The ambient sound and music are the GIT's `AreaProperties`
//! (ambientsound.2da and ambientmusic.2da rows, day or night as the view
//! shows the area), looped; the placed sounds are its `SoundList`, heard
//! from the view's focus: full volume within the sound's `MinDistance`,
//! fading linearly to nothing at its `MaxDistance` (an approximation of
//! the game's falloff), each playing its list in order or at random,
//! seamlessly looping, repeating after its interval, or once.

use std::collections::HashMap;

use glam::Vec3;
use mg_core::{ResRef, ResType};
use mg_gff::Struct;
use mg_resman::ResKey;

use crate::Moonglow;
use crate::audio::Channel;

/// Aurora's ambient music volume (`Ambient Music Volume`), of 127.
pub(crate) const MUSIC_VOLUME: u8 = 92;

/// A placed sound's fields (a GIT `SoundList` entry, a UTS instance).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlacedSound {
    pub(crate) sounds: Vec<ResRef>,
    pub(crate) position: Vec3,
    pub(crate) active: bool,
    /// Heard from where it is (else everywhere in the area).
    pub(crate) positional: bool,
    pub(crate) min_distance: f32,
    pub(crate) max_distance: f32,
    /// 0 to 1 (`Volume` of 127).
    pub(crate) volume: f32,
    /// Seamlessly looping (`Looping`), repeating (`Continuous`) or once.
    pub(crate) looping: bool,
    pub(crate) continuous: bool,
    pub(crate) random: bool,
    /// Seconds between sounds, and how much that varies either way.
    pub(crate) interval: f32,
    pub(crate) interval_variation: f32,
    /// `Times`: 3 always, 1 day, 2 night, 0 the hours in `Hours`.
    pub(crate) times: u8,
    pub(crate) hours: u32,
}

impl PlacedSound {
    pub(crate) fn from_git(s: &Struct) -> PlacedSound {
        let int = |l: &str, d: i64| s.integer(l).unwrap_or(d);
        let float = |l: &str| s.float(l).unwrap_or(0.0);
        PlacedSound {
            sounds: s
                .list("Sounds")
                .unwrap_or(&[])
                .iter()
                .filter_map(|e| e.resref("Sound"))
                .filter(|r| !r.is_empty())
                .collect(),
            position: Vec3::new(float("XPosition"), float("YPosition"), float("ZPosition")),
            active: int("Active", 1) != 0,
            positional: int("Positional", 0) != 0,
            min_distance: float("MinDistance"),
            max_distance: float("MaxDistance"),
            volume: int("Volume", 127).clamp(0, 127) as f32 / 127.0,
            looping: int("Looping", 0) != 0,
            continuous: int("Continuous", 0) != 0,
            random: int("Random", 0) != 0,
            interval: int("Interval", 0).max(0) as f32 / 1000.0,
            interval_variation: int("IntervalVrtn", 0).max(0) as f32 / 1000.0,
            times: int("Times", 3).clamp(0, 3) as u8,
            hours: int("Hours", 0).max(0) as u32,
        }
    }

    /// Its volume heard at `listener`.
    pub(crate) fn gain(&self, listener: Vec3) -> f32 {
        if !self.positional {
            return self.volume;
        }
        let d = self.position.distance(listener);
        let (near, far) = (self.min_distance.max(0.0), self.max_distance.max(0.0));
        if d <= near {
            self.volume
        } else if d >= far {
            0.0
        } else {
            self.volume * (far - d) / (far - near)
        }
    }

    /// Whether it plays by day (noon) or night (midnight).
    pub(crate) fn plays(&self, night: bool) -> bool {
        match self.times {
            3 => true,
            1 => !night,
            2 => night,
            _ => self.hours & (1 << if night { 0 } else { 12 }) != 0,
        }
    }
}

/// What the area plays, from frame to frame.
#[derive(Debug, Default)]
pub(crate) struct AreaAudio {
    area: Option<(ResRef, bool)>,
    ambient: Option<ResRef>,
    music: Option<ResRef>,
    placed: HashMap<usize, Slot>,
}

/// A placed sound's turn: what it plays and when it may play again.
#[derive(Debug, Default)]
struct Slot {
    playing: bool,
    wait_until: f64,
    next: usize,
    done: bool,
}

/// The area view drawn this frame: its area, where it listens from, and
/// whether it shows night.
pub(crate) type Heard = (ResRef, Vec3, bool);

/// A 2DA's `Resource` for a row (ambientsound.2da, ambientmusic.2da).
fn resource(app: &Moonglow, table: &str, row: i64) -> Option<ResRef> {
    let t = app.game.as_ref()?.table(table).ok()?;
    let name = t.get(usize::try_from(row).ok()?, "Resource")?;
    ResRef::from_str(name).ok().filter(|r| !r.is_empty())
}

/// Plays and stops the area's sounds for this frame (`now` in seconds).
/// Whether anything plays or waits its turn (so frames should go on).
pub(crate) fn update(app: &mut Moonglow, heard: Option<Heard>, now: f64) -> bool {
    let mut state = std::mem::take(&mut app.area_audio);
    let place = heard.map(|(a, _, night)| (a, night));
    if state.area != place {
        stop_all(app, &mut state);
        state.area = place;
    }
    let Some((area, listener, night)) = heard else {
        app.area_audio = state;
        return false;
    };
    let git = app
        .ws
        .as_mut()
        .and_then(|ws| ws.doc(&ResKey::new(area, ResType::GIT)).ok())
        .map(|g| g.root.clone())
        .unwrap_or_default();
    let props = git.child("AreaProperties").cloned().unwrap_or_default();
    let int = |l: &str| props.integer(l).unwrap_or(0);

    // Ambient sound and music, looped.
    let (sound, sound_volume, music) = if night {
        (int("AmbientSndNight"), int("AmbientSndNitVol"), int("MusicNight"))
    } else {
        (int("AmbientSndDay"), int("AmbientSndDayVol"), int("MusicDay"))
    };
    let ambient = app.settings.ambient_sound.then(|| resource(app, "ambientsound", sound));
    let volume = sound_volume.clamp(0, 127) as f32 / 127.0;
    loop_on(app, Channel::Ambient, &mut state.ambient, ambient.flatten(), volume);
    let tune = app.settings.ambient_music.then(|| resource(app, "ambientmusic", music));
    let volume = f32::from(app.settings.music_volume.unwrap_or(MUSIC_VOLUME).min(127)) / 127.0;
    loop_on(app, Channel::Music, &mut state.music, tune.flatten(), volume);

    // Placed sounds.
    let list: Vec<PlacedSound> = if app.settings.no_placed_sounds {
        Vec::new()
    } else {
        git.list("SoundList").unwrap_or(&[]).iter().map(PlacedSound::from_git).collect()
    };
    state.placed.retain(|&i, _| {
        let keep = i < list.len();
        if !keep {
            app.speaker.stop(Channel::Placed(i));
        }
        keep
    });
    for (i, p) in list.iter().enumerate() {
        let channel = Channel::Placed(i);
        let slot = state.placed.entry(i).or_default();
        let gain = p.gain(listener);
        if !p.active || !p.plays(night) || p.sounds.is_empty() || gain <= 0.0 {
            if slot.playing {
                app.speaker.stop(channel);
                slot.playing = false;
            }
            continue;
        }
        if app.speaker.playing(channel) {
            app.speaker.set_volume(channel, gain);
            continue;
        }
        if slot.playing {
            // It ended: the next one after the interval (none when looping).
            slot.playing = false;
            let vary = f64::from(p.interval_variation) * (fastrand::f64() * 2.0 - 1.0);
            let pause = if p.looping { 0.0 } else { f64::from(p.interval) + vary };
            slot.wait_until = now + pause.max(0.0);
        }
        if slot.done || now < slot.wait_until {
            continue;
        }
        let pick = if p.random { fastrand::usize(..p.sounds.len()) } else { slot.next };
        slot.next = (pick + 1) % p.sounds.len();
        let looped = p.looping && p.sounds.len() == 1;
        if app.play_sound(channel, p.sounds[pick % p.sounds.len()], gain, looped) {
            slot.playing = true;
            slot.done = !p.looping && !p.continuous;
        } else {
            // Missing or broken: not tried again in this view.
            slot.done = true;
        }
    }
    let busy = state.ambient.is_some()
        || state.music.is_some()
        || state.placed.values().any(|s| s.playing || (!s.done && s.wait_until > now));
    app.area_audio = state;
    busy
}

/// Keeps `want` looping on a channel at a volume (or nothing).
fn loop_on(
    app: &mut Moonglow,
    channel: Channel,
    current: &mut Option<ResRef>,
    want: Option<ResRef>,
    volume: f32,
) {
    if *current != want {
        app.speaker.stop(channel);
        *current = want.filter(|&w| app.play_sound(channel, w, volume, true));
    } else if current.is_some() {
        app.speaker.set_volume(channel, volume);
    }
}

fn stop_all(app: &mut Moonglow, state: &mut AreaAudio) {
    for c in [Channel::Ambient, Channel::Music] {
        app.speaker.stop(c);
    }
    for &i in state.placed.keys() {
        app.speaker.stop(Channel::Placed(i));
    }
    *state = AreaAudio::default();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound(positional: bool) -> PlacedSound {
        PlacedSound {
            sounds: vec![ResRef::from_str("as_cv_bell1").unwrap()],
            position: Vec3::new(10.0, 10.0, 0.0),
            active: true,
            positional,
            min_distance: 5.0,
            max_distance: 15.0,
            volume: 0.5,
            looping: false,
            continuous: true,
            random: false,
            interval: 2.0,
            interval_variation: 0.0,
            times: 3,
            hours: 0,
        }
    }

    #[test]
    fn placed_sounds_fade_with_distance_and_keep_their_hours() {
        let s = sound(true);
        assert_eq!(s.gain(Vec3::new(12.0, 10.0, 0.0)), 0.5, "within the full volume distance");
        assert!((s.gain(Vec3::new(20.0, 10.0, 0.0)) - 0.25).abs() < 1e-6, "halfway out");
        assert_eq!(s.gain(Vec3::new(40.0, 10.0, 0.0)), 0.0, "past the cutoff");
        assert_eq!(sound(false).gain(Vec3::new(400.0, 0.0, 0.0)), 0.5, "everywhere");
        let at = |times, hours| PlacedSound { times, hours, ..sound(true) };
        assert!(at(1, 0).plays(false) && !at(1, 0).plays(true));
        assert!(!at(2, 0).plays(false) && at(2, 0).plays(true));
        assert!(at(0, 1 << 12).plays(false) && !at(0, 1 << 12).plays(true));
    }

    #[test]
    fn placed_sounds_read_their_git_fields() {
        let mut s = Struct::new(0);
        let mut e = Struct::new(0);
        e.set("Sound", mg_gff::Value::resref(ResRef::from_str("as_cv_bell1").unwrap()));
        s.set("Sounds", mg_gff::Value::List(vec![e]));
        s.set("XPosition", mg_gff::Value::Float(3.0));
        s.set("Volume", mg_gff::Value::Byte(127));
        s.set("Positional", mg_gff::Value::Byte(1));
        s.set("Interval", mg_gff::Value::Dword(1500));
        s.set("Times", mg_gff::Value::Byte(2));
        let p = PlacedSound::from_git(&s);
        assert_eq!(p.sounds.len(), 1);
        assert_eq!((p.position.x, p.volume, p.interval, p.times), (3.0, 1.0, 1.5, 2));
        assert!(p.positional && p.active && !p.looping);
    }
}
